use std::{
    fs,
    fs::OpenOptions,
    io::Write,
};

use actix_web::{get, web, HttpResponse};
use rodio::{OutputStream, Sink, Source};

use crate::structs::{ResponseMessage, TimeQuery, AudioFiles, RandomAudioQueueOptions};
use crate::audio::{handle_audio_error, pause_sink_duration};
use crate::LOG_FILE_NAME;


#[get("/play/{audio_file_name}")]
async fn play(audio_files: web::Data<AudioFiles> , audio_file_name: web::Path<String>, query: web::Query<TimeQuery>) -> HttpResponse {
    let time_ns = std::time::SystemTime::now().duration_since(std::time::SystemTime::UNIX_EPOCH).unwrap().as_nanos();
    println!("{}: Received /play/{}", time_ns, audio_file_name);

    let audio_file_name = audio_file_name.into_inner();
    let source = audio_files.files.get(&audio_file_name); // find decoded audio file by name

    // if the audio file is not found, return 404
    if source.is_none() {
        println!("\x1b[2m    \x1b[31mAudio file Not Found\x1b[0m");
        let message = format!("Audio file {} not found", audio_file_name);
        return HttpResponse::NotFound().json(ResponseMessage { message });
    }

    // Linux with ALSA will panic here if there is no audio output device available
    let output_stream_result = std::panic::catch_unwind(|| OutputStream::try_default());

    if output_stream_result.is_err() {
        return handle_audio_error(&audio_file_name, &query.time, "OutputStream NoDevice", &LOG_FILE_NAME);
    }

    let output_stream_result = output_stream_result.unwrap();

    // Windows somehow panics when unwraping the output_stream_result for the same reason (no audio output device available)
    if let Err(e) = output_stream_result {
        return handle_audio_error(&audio_file_name, &query.time, &e.to_string(), &LOG_FILE_NAME);
    }

    // now safe to unwrap
    let (_stream, stream_handle) = output_stream_result.unwrap();

    // print the source sample rate
    println!("\x1b[2m    \x1b[38;5;8mSource's Sample Rate: {} Hz\x1b[0m", source.unwrap().sample_rate());

    let sink = Sink::try_new(&stream_handle).unwrap();
    sink.append(source.unwrap().clone()); // init the sink with the audio file

    let time_start_nano = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos();
    
    println!("\x1b[2m    \x1b[38;5;8m{}: Started {}...\x1b[0m", time_start_nano, audio_file_name);
    sink.sleep_until_end(); // play the audio file synchronously. this thread will be blocked until the audio file has finished playing.
    println!("\x1b[2m    \x1b[38;5;8mFinished (job at {})\x1b[0m", time_start_nano);

    let message = format!("At {} played {}", time_start_nano, audio_file_name);
    drop(sink);
    drop(stream_handle);
    drop(_stream);
    
    // Append to the log file
    fs::create_dir_all("./logs").unwrap(); // make sure the logs/ folder exists first

    let log_file_name = LOG_FILE_NAME.lock().unwrap();
    let mut file = OpenOptions::new()
        .write(true)
        .append(true)
        .create(true)
        .open(format!("{}.csv", *log_file_name))
        .unwrap();
    if let Err(e) = writeln!(file, "{},{},{},{}", time_start_nano, audio_file_name, "success", &query.time) {
        eprintln!("Couldn't write to file: {}", e);
    } else {
        println!("\x1b[2m    \x1b[38;5;8mAppended to log file: {}\x1b[0m", *log_file_name);
    }

    drop(log_file_name); // release the lock
    drop(file); // release the lock

    HttpResponse::Ok().json(ResponseMessage { message })
}

#[get("/play/random")]
async fn play_random(audio_files: web::Data<AudioFiles>, query: web::Query<TimeQuery>, audio_queue_opt: web::Query<RandomAudioQueueOptions>) -> HttpResponse {
    let time_ns = std::time::SystemTime::now().duration_since(std::time::SystemTime::UNIX_EPOCH).unwrap().as_nanos();
    println!("{}: Received /play/random", time_ns);

    // If no audio_files are found, return 404
    if audio_files.files.len() == 0 {
        println!("\x1b[2m    \x1b[31mNo audio files found\x1b[0m");
        let message = format!("No audio files found");
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



    // Each play random request will have its own log file, and the first line of the log file will be the request start time.
    // Need to start the log file here first.
    fs::create_dir_all("./logs").unwrap();

    // init a new log file name with the current date time, specific for random, though
    let log_file_name_process = chrono::Utc::now().format("logs/log_playrandom_%Y%m%d-%H%M%S").to_string();

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
    let text = format!("{},Received /play/random (break: {} ms | file_count: {}),{},{}", time_ns, &audio_queue_opt.break_between_files, audio_queue_opt.file_count, "success", &query.time);

    if let Err(e) = writeln!(file, "{}", text) {
        eprintln!("Couldn't write to file: {}", e);
    } else {
        println!("\x1b[2m    \x1b[38;5;8mAppended request info to log file: {}\x1b[0m\n", &log_file_name_process);
    }



    // default to 100 files if file_count is not specified
    let file_count = if audio_queue_opt.file_count == 0 { 100 } else { audio_queue_opt.file_count };

    let mut remaining_files: u32 = file_count;
    if remaining_files == 0 {
        remaining_files = 100;
    }

    while remaining_files > 0 {
        // randomly choose a file from the audio_files HashMap
        let audio_file_name = audio_files.files.keys().nth(rand::random::<usize>() % audio_files.files.len()).unwrap();
        let source = audio_files.files.get(audio_file_name).unwrap().clone(); // find decoded audio file by name

        // append the audio file to the sink
        sink.append(source);
        
        remaining_files -= 1;
        
        let time_start_nano = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos();
        println!("\x1b[32m    {}: Playing {}...\x1b[0m", time_start_nano, audio_file_name);
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

        // if there are more files to play AND the break is not 0, pause for the break duration
        if remaining_files > 0 && audio_queue_opt.break_between_files > 0 {
            let time_start_nano = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos();
            println!("\x1b[34m    {}: Pausing for {} ms...\x1b[0m", time_start_nano, &audio_queue_opt.break_between_files);
            pause_sink_duration(&sink, &audio_queue_opt.break_between_files);

            // Append to the log file here (the "file name" is "pause_{ms}ms")
            let mut file = OpenOptions::new()
                .write(true)
                .append(true)
                .create(true)
                .open(format!("{}.csv", &log_file_name_process))
                .unwrap();

            if let Err(e) = writeln!(file, "{},{},{},{}", time_start_nano, format!("pause_{}ms", &audio_queue_opt.break_between_files), "success", "N/A") {
                eprintln!("Couldn't write to file: {}", e);
            } else {
                println!("\x1b[2m    \x1b[38;5;8mAppended (pause/break/interval) to log file: {}\x1b[0m\n", &log_file_name_process);
            }
        }
    }

    let request_duration = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos() - time_ns;
    let message = format!("At {} started playing {} random audio files. Took {} seconds.", time_ns, &file_count, (request_duration as f64/ 1_000_000_000 as f64) as f32);

    println!("\x1b[1m    \x1b[38;5;8m{}\x1b[0m", message);

    HttpResponse::Ok().json(ResponseMessage { message })
}