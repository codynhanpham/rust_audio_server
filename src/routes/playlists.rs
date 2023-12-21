use std::{
    fs,
    fs::OpenOptions,
    io::Write,
};

use actix_web::{get, web, HttpResponse};

use local_ip_address::local_ip;

use rand::Rng;
use rodio::{OutputStream, Sink, Source};
use sha256::digest;

use crate::structs::{ResponseMessage, TimeQuery, AudioFiles, PlaylistOptions, Playlists};
use crate::audio::{handle_audio_error, pause_sink_duration, PlaylistTypes};
use crate::{LOG_FILE_NAME, PLAYLISTS, PORT};


// Create and send back a .txt file containing the playlist
// The text file is simply the list of audio file names in the playlist, one per line
// If the break_between_files parameter is specified, the text file will contain the break duration in milliseconds between each audio file (interweaved with the audio file names)
// The break should be formatted as "pause_<duration_in_milliseconds>ms"
#[get("/playlist/create")]
async fn create_playlist(audio_files: web::Data<AudioFiles>, query: web::Query<PlaylistOptions>) -> HttpResponse {
    let time_ns = std::time::SystemTime::now().duration_since(std::time::SystemTime::UNIX_EPOCH).unwrap().as_nanos();
    println!("{}: Received /playlist/create", time_ns);
    
    // If no audio files are found, return 404
    if audio_files.files.len() == 0 {
        println!("\x1b[2m    \x1b[31mNo audio files found\x1b[0m");
        let message = format!("No audio files found");
        return HttpResponse::NotFound().json(ResponseMessage { message });
    }


    let playlist_options = query.into_inner();
    
    // Default playlist options (if the value is 0): file_count = 10
    let file_count = if playlist_options.file_count == 0 { 10 } else { playlist_options.file_count };
    
    let mut total_duration = 0;

    // Pick random audio files from the audio_files HashMap up to the file_count, also count the total duration of the audio files
    let mut rng = rand::thread_rng();
    let mut playlist: Vec<String> = Vec::new();
    for _ in 0..file_count {
        let random_index = rng.gen_range(0..audio_files.files.len());
        let random_audio_file_name = audio_files.files.keys().nth(random_index).unwrap();
        playlist.push(random_audio_file_name.clone());
        total_duration += audio_files.files.get(random_audio_file_name).unwrap().total_duration().unwrap().as_millis();
    }

    // If break_between_files is specified, add the break duration in milliseconds between each audio file, also update the total duration
    if playlist_options.break_between_files != 0 {
        let break_duration = playlist_options.break_between_files;
        total_duration += (break_duration * (playlist.len() - 1) as u32) as u128;

        // Add breaks interweaved with the audio file names
        let mut playlist_with_breaks: Vec<String> = Vec::new();
        for i in 0..playlist.len() {
            playlist_with_breaks.push(playlist[i].clone());
            if i + 1 < playlist.len() {
                playlist_with_breaks.push(format!("pause_{}ms", break_duration));
            }
        }
        playlist = playlist_with_breaks;
    }

    // Name: {HashID (8 characters)}_{Duration (in milliseconds)}s_{FileCount}count.txt
    let output_string = playlist.join("\n");
    let id = digest(output_string.as_bytes()).chars().take(8).collect::<String>();

    // This file name will be used as the return header
    let playlist_file_name = format!("playlist_{}_{:?}s_{}count.txt", id, total_duration as f64 / 1000.0, playlist.len());


    // Always update the server-side playlist, then hot reload the playlists
    fs::create_dir_all("./playlists").unwrap();
    // Save the new playlist to file
    let mut file = OpenOptions::new()
        .write(true)
        .append(true)
        .create(true)
        .open(format!("./playlists/{}", &playlist_file_name))
        .unwrap();

    if let Err(e) = writeln!(file, "{}", output_string) {
        eprintln!("Couldn't create new file: {}", e);
    } else {
        println!("\x1b[2m    \x1b[38;5;8mCreated new playlist file server-side: ./playlists/{}\x1b[0m", &playlist_file_name);
    }

    // Reload the playlists
    println!(" !! Hot Reloading Playlists !!");
    let mut playlists = PLAYLISTS.lock().unwrap();
    *playlists = Playlists { playlists: crate::load_and_validate_playlists("./playlists", &audio_files.files) };
    drop(playlists);
    

    // Check for playlist_options.no_download: Change the HTTP response accordingly
    if playlist_options.no_download {
        // If no_download is true, return a JSON response with the playlist file name
        let message = format!("Created new playlist file server-side: ./playlists/{}. To play this new playlist, visit: http://{}:{}/playlist/{}", &playlist_file_name, local_ip().unwrap(), PORT, &playlist_file_name);
        return HttpResponse::Ok().json(ResponseMessage { message });
    }

    // Return the playlist file as a text file to download
    HttpResponse::Ok()
        .content_type("text/plain")
        .append_header(("Content-Disposition", format!("attachment; filename={}", playlist_file_name)))
        .body(output_string)

}


// Play the playlist
#[get("/playlist/{playlist_file_name}")]
async fn play(playlist_file_name: web::Path<String>, audio_files: web::Data<AudioFiles>, query: web::Query<TimeQuery>) -> HttpResponse {
    let time_ns = std::time::SystemTime::now().duration_since(std::time::SystemTime::UNIX_EPOCH).unwrap().as_nanos();
    println!("{}: Received /playlist/{}", time_ns, playlist_file_name);

    // If no audio files are found, return 404
    if audio_files.files.len() == 0 {
        println!("\x1b[2m    \x1b[31mNo audio files found\x1b[0m");
        let message = format!("No audio files found");
        return HttpResponse::NotFound().json(ResponseMessage { message });
    }

    let available_playlists = PLAYLISTS.lock().unwrap();

    // If the playlist file name is not found in the available_playlists HashMap, return 404
    if !available_playlists.playlists.contains_key(&playlist_file_name.to_string()) {
        println!("\x1b[2m    \x1b[31mPlaylist file name not found\x1b[0m");
        let message = format!("Playlist file name not found");
        return HttpResponse::NotFound().json(ResponseMessage { message });
    }

    // If the playlist file name is found in the available_playlists HashMap, but the playlist is empty, return 404
    if available_playlists.playlists.get(&playlist_file_name.to_string()).unwrap().len() == 0 {
        println!("\x1b[2m    \x1b[31mPlaylist is empty\x1b[0m");
        let message = format!("Playlist is empty");
        return HttpResponse::NotFound().json(ResponseMessage { message });
    }

    // In case audio device error --> handle this first
    // Linux with ALSA will panic here if there is no audio output device available
    let output_stream_result = std::panic::catch_unwind(|| OutputStream::try_default());

    if output_stream_result.is_err() {
        return handle_audio_error("/play/random", "/play/random", "OutputStream NoDevice", &LOG_FILE_NAME);
    }

    let output_stream_result = output_stream_result.unwrap();

    // Windows somehow panics when unwraping the output_stream_result for the same reason (no audio output device available)
    if let Err(e) = output_stream_result {
        return handle_audio_error("/play/random", "/play/random", &e.to_string(), &LOG_FILE_NAME);
    }

    // now safe to unwrap
    let (_stream, stream_handle) = output_stream_result.unwrap();

    let sink = Sink::try_new(&stream_handle).unwrap();
    sink.pause(); // pause the sink so that it doesn't play anything yet

    

    // Each play request will have its own log file, and the first line of the log file will be the request start time.
    // Need to start the log file here first.
    fs::create_dir_all("./logs").unwrap();

    // init a new log file name with the current date time, specific for playlist, though
    let log_file_name_process = chrono::Utc::now().format("logs/log_playlist_%Y%m%d-%H%M%S").to_string();

    // create new log file
    let mut file = OpenOptions::new()
        .write(true)
        .append(true)
        .create(true)
        .open(format!("{}.csv", &log_file_name_process))
        .unwrap();

    if let Err(e) = writeln!(file, "timestamp_audio,audio_filename,status,timestamp_client") {
        eprintln!("Couldn't create new file: {}", e);
    } else {
        println!("\x1b[1m    Started new log file: ./{}.csv\x1b[0m", &log_file_name_process);
    }

    // Append to the log file the request start time
    let text = format!("{},Received /playlist/{},{},{}", time_ns, playlist_file_name, "success", &query.time);

    if let Err(e) = writeln!(file, "{}", text) {
        eprintln!("Couldn't write to file: {}", e);
    } else {
        println!("\x1b[2m    \x1b[38;5;8mAppended request info to log file: {}\x1b[0m\n", &log_file_name_process);
    }

    
    // Get the playlist from the available_playlists HashMap
    let playlist = available_playlists.playlists.get(&playlist_file_name.to_string()).unwrap().clone();

    drop(available_playlists); // release the lock on global PLAYLISTS

    // One option here is to just append all of the audio files in the playlist to the sink
    // However, the trade off is that, we don't really know when which audio file is playing --> not as verbose
    // So? Append and play each audio file one by one, and log the start time of each audio file (or pause, of course)
    // Will have a bit of delay between each audio file, but that should be alright
    let time_ns_playback = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos();
    for (i, audio_file_name) in playlist.iter().enumerate() {
        // See if this is a pause or an audio file by checking its PlayListTypes (AudioFiles or Pause)
        match audio_file_name {
            PlaylistTypes::AudioFiles(audio_file_name) => {
                // Append the audio file to the sink
                let audio_file = audio_files.files.get(audio_file_name).unwrap();
                sink.append(audio_file.clone());

                let time_start_nano = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos();
                println!("\x1b[32m    [{}/{}] {}: Playing {}...\x1b[0m", i+1, playlist.len(), time_start_nano, audio_file_name);
                sink.play(); // play the audio file
                sink.sleep_until_end(); // play the audio file synchronously. this thread will be blocked until the audio file has finished playing.
                
                println!("\x1b[2m    \x1b[38;5;8mFinished (job at {})\x1b[0m", time_start_nano);

                let mut file = OpenOptions::new()
                    .write(true)
                    .append(true)
                    .create(true)
                    .open(format!("{}.csv", &log_file_name_process))
                    .unwrap();

                if let Err(e) = writeln!(file, "{},{},{},{}", time_start_nano, audio_file_name, "success", "N/A") {
                    eprintln!("Couldn't write to file: {}", e);
                } else {
                    println!("\x1b[2m    \x1b[38;5;8mAppended to log file: {}\x1b[0m\n", &log_file_name_process);
                }
            },

            PlaylistTypes::Pause(pause_duration) => {
                // If this is a pause, pause the sink for the specified duration
                let time_start_nano = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos();
                println!("\x1b[34m    [{}/{}] {}: Pausing for {} milliseconds...\x1b[0m", i+1, playlist.len(), time_start_nano, pause_duration);
                pause_sink_duration(&sink, &pause_duration);

                let mut file = OpenOptions::new()
                    .write(true)
                    .append(true)
                    .create(true)
                    .open(format!("{}.csv", &log_file_name_process))
                    .unwrap();

                if let Err(e) = writeln!(file, "{},{},{},{}", time_start_nano, format!("pause_{}ms", pause_duration), "success", "N/A") {
                    eprintln!("Couldn't write to file: {}", e);
                } else {
                    println!("\x1b[2m    \x1b[38;5;8mAppended to log file: {}\x1b[0m\n", &log_file_name_process);
                }
            }
        }
    }

    let request_duration = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos() - time_ns;
    let playback_duration = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos() - time_ns_playback;
    let message = format!("At {} started playlist {} ({} audio files). Playback took {} seconds. Total time since request: {} seconds.", time_ns, playlist_file_name, playlist.len(), playback_duration as f64 / 1_000_000_000.0, request_duration as f64 / 1_000_000_000.0);

    println!("\x1b[1m    \x1b[38;5;8m{}\x1b[0m", message);

    HttpResponse::Ok().json(ResponseMessage { message })
}
