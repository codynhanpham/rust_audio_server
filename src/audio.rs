use std::{
    collections::HashMap,
    fs,
    fs::OpenOptions,
    io::{BufReader, Write, Cursor},
};

use actix_web::HttpResponse;
use hound;
use rand::prelude::*;
use rodio::{
    source::Buffered,
    Decoder,
    Source,
};

use crate::structs::{ResponseMessage, RandomAudioQueueOptions};


// Preload audio files to RAM for faster playback
// Returns a HashMap of audio file names and their corresponding Buffered<Decoder<BufReader<std::fs::File>>> objects
pub fn preload_audio_files(audio_folder_path: &str) -> HashMap<String, Buffered<Decoder<BufReader<std::fs::File>>>> {
    println!("Preloading audio files...");

    // check if the audio folder exists, if not, display an error message and exit
    if !fs::metadata(audio_folder_path).is_ok() {
        println!("\x1b[2m    \x1b[38;5;8mError: Audio folder not found\x1b[0m");
        println!("Please create a folder named \"audio\" in the same directory as this executable and put your audio files in it.");

        // wait for user hitting enter before exiting
        println!("\n\n(Press Enter to exit)");
        let mut input = String::new();
        std::io::stdin().read_line(&mut input).unwrap();

        std::process::exit(1);
    }

    let mut files = HashMap::new();
    let paths = fs::read_dir(audio_folder_path).unwrap();

    let audio_extensions = vec!["mp3", "wav", "flac", "ogg"]; // allowed audio file extensions

    for path in paths {
        let path = path.unwrap().path();
        let file_name = path.file_name().unwrap().to_str().unwrap().to_string();

        // ignore files that are not audio files
        let extension = path.extension().unwrap().to_str().unwrap();
        if !audio_extensions.contains(&extension) {
            continue;
        }

        let file = std::fs::File::open(path).unwrap();
        let source = Decoder::new(BufReader::new(file)).unwrap().buffered();
        files.insert(file_name, source);
    }
    println!("Preloaded {} audio files to RAM\n", files.len());
    files
}

#[derive(Clone)]
pub enum PlaylistTypes {
    AudioFiles(String),
    Pause(u32),
}

// Load the .txt files in the playlists folder and validate the audio file names (make sure they exist in the audio folder)
pub fn load_and_validate_playlists(playlists_folder_path: &str, audio_files: &HashMap<String, Buffered<Decoder<BufReader<std::fs::File>>>>) -> HashMap<String, Vec<PlaylistTypes>> {
    // if no /playlists folder is found, return an empty HashMap
    if !fs::metadata(playlists_folder_path).is_ok() {
        return HashMap::new();
    }

    println!("Loading playlists...");

    // Playlist: Key is the name of the file.txt, Value is a Vec of audio file names (and breaks if applicable)
    let mut playlists: HashMap<String, Vec<PlaylistTypes>> = HashMap::new();

    let paths = fs::read_dir(playlists_folder_path).unwrap();

    for path in paths {
        let path = path.unwrap().path();
        let file_name = path.file_name().unwrap().to_str().unwrap().to_string();

        // ignore files that are not .txt files
        let extension = path.extension().unwrap().to_str().unwrap();
        if extension != "txt" {
            continue;
        }

        // read the file
        let file = std::fs::File::open(path).unwrap();
        let mut reader = BufReader::new(file);
        let mut contents = String::new();
        std::io::Read::read_to_string(&mut reader, &mut contents).unwrap();

        // split the file contents by line
        let lines = contents.split("\n");

        // validate the audio file names: if any of the audio file names are not found in the audio folder, or not start with "pause_", then ignore the playlist
        let mut playlist: Vec<PlaylistTypes> = Vec::new();
        let mut error_occurred = false; // Add this flag

        for line in lines {
            let line = line.trim();

            // ignore empty lines
            if line == "" {
                continue;
            }

            // if the line starts with "pause_", then it's a break
            if line.starts_with("pause_") {
                let break_duration = line.replace("pause_", "").replace("ms", "").parse::<u32>().unwrap();
                playlist.push(PlaylistTypes::Pause(break_duration));
                continue;
            }

            // if the line is not a break, then it's an audio file name
            // check if the audio file name exists in the audio folder
            if !audio_files.contains_key(line) {
                println!("\x1b[2m    \x1b[31mError: Audio file \"{}\" not found\x1b[0m", line);
                println!("\x1b[2m    Please make sure the audio file exists in the \"audio\" folder and try again.\x1b[0m");
                println!("\x1b[2m    Ignoring playlist \"{}\"...\n\x1b[0m", file_name);
                error_occurred = true; // Set the flag to true
                break; // Break the loop for audio in playlist
            }

            // if the audio file name exists in the audio folder, then add it to the playlist
            playlist.push(PlaylistTypes::AudioFiles(line.to_string()));
        }

        // if the playlist is empty or an error occurred, then ignore it
        if playlist.len() == 0 || error_occurred {
            continue;
        }

        // if the playlist is not empty, then add it to the playlists HashMap
        playlists.insert(file_name, playlist);
    }

    println!("Loaded {} playlists\n", playlists.len());

    playlists
}


// Correctly print and log the error when no audio output device is available
pub fn handle_audio_error(audio_file_name: &str, request_time: &str, e: &str, global_log_file_name: &std::sync::Mutex<String>) -> HttpResponse {
    println!("\x1b[2m    \x1b[31m{}\x1b[0m", e);
    println!("\x1b[2m    \x1b[31mError: Could not create OutputStream. Is there any audio output device available?\x1b[0m");

    let message = format!("Could not create OutputStream. Is there any audio output device available? - Error: {}", e);

    // update the log file with the error
    let time_start_nano = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos();
    let log_file_name = global_log_file_name.lock().unwrap();
    let mut file = OpenOptions::new()
        .write(true)
        .append(true)
        .create(true)
        .open(format!("{}.csv", *log_file_name))
        .unwrap();
    if let Err(e) = writeln!(file, "{},{},{},{}", time_start_nano, audio_file_name, "error", request_time) {
        eprintln!("Couldn't write to file: {}", e);
    } else {
        println!("\x1b[2m    \x1b[31mAppended to log file (error): {}\x1b[0m", *log_file_name);
    }

    drop(log_file_name); // release the lock
    drop(file); // release the lock

    HttpResponse::InternalServerError().json(ResponseMessage { message })
}


// create a pure sine tone and return a valid rodio Source to be played or saved to a file
// freq: frequency in Hz
// duration: duration in milliseconds
// amplitude: amplitude of the sine wave in dB
// sample_rate: sample rate in Hz
pub fn generate_sine_tone(freq: &f32, duration: &u32, amplitude: &f32, sample_rate: &u32) -> Vec<f32> {
    let duration = *duration as f32 / 1000.0; // convert to seconds
    let sample_rate = *sample_rate as f32;
    let amplitude = 10.0_f32.powf(*amplitude / 20.0); // convert from dB to linear

    let samples = (duration * sample_rate) as usize;
    let mut sine_tone = Vec::with_capacity(samples);

    for i in 0..samples {
        let t = i as f32 / sample_rate;
        let sample = amplitude * (t * freq * 2.0 * std::f32::consts::PI).sin();
        sine_tone.push(sample);
    }

    sine_tone
}

pub fn tone_to_source(freq: &f32, duration: &u32, amplitude: &f32, sample_rate: &u32) -> Buffered<Decoder<BufReader<Cursor<Vec<u8>>>>> {
    let sine_tone = generate_sine_tone(freq, duration, amplitude, sample_rate);

    // Create the Cursor<Vec<u8>> separately
    let mut cursor = Cursor::new(Vec::new());

    // Convert the sine_tone vector to a Wav file in memory
    let spec = hound::WavSpec {
        channels: 1,
        sample_rate: *sample_rate,
        bits_per_sample: 32,
        sample_format: hound::SampleFormat::Float,
    };
    {
        let mut writer = hound::WavWriter::new(&mut cursor, spec).unwrap();
        for sample in sine_tone {
            writer.write_sample(sample).unwrap();
        }
        writer.finalize().unwrap();
    }

    // Convert the Wav file in memory to a rodio Source
    let source = rodio::Decoder::new(BufReader::new(Cursor::new(cursor.into_inner()))).unwrap().buffered();
    source
}

pub fn tone_to_wav_file(freq: &f32, duration: &u32, amplitude: &f32, sample_rate: &u32) -> Vec<u8> {
    let sine_tone = generate_sine_tone(freq, duration, amplitude, sample_rate);

    // Convert the sine_tone vector to a Wav file in memory
    let spec = hound::WavSpec {
        channels: 1,
        sample_rate: *sample_rate,
        bits_per_sample: 32,
        sample_format: hound::SampleFormat::Float,
    };
    let mut cursor = Cursor::new(Vec::new());
    {
        let mut writer = hound::WavWriter::new(&mut cursor, spec).unwrap();
        for sample in sine_tone {
            writer.write_sample(sample).unwrap();
        }
        writer.finalize().unwrap();
    }

    // return a buffer for the client to download
    let buffer = cursor.into_inner();

    buffer
}

pub fn silence_as_source(duration: &u32, sample_rate: &u32) -> Buffered<Decoder<BufReader<Cursor<Vec<u8>>>>> {
    let silence = vec![0.0; (*duration as f32 / 1000.0 * *sample_rate as f32) as usize];

    // Create the Cursor<Vec<u8>> separately
    let mut cursor = Cursor::new(Vec::new());

    // Convert the sine_tone vector to a Wav file in memory
    let spec = hound::WavSpec {
        channels: 1,
        sample_rate: *sample_rate,
        bits_per_sample: 32,
        sample_format: hound::SampleFormat::Float,
    };
    {
        let mut writer = hound::WavWriter::new(&mut cursor, spec).unwrap();
        for sample in silence {
            writer.write_sample(sample).unwrap();
        }
        writer.finalize().unwrap();
    }

    // Convert the Wav file in memory to a rodio Source
    let source = rodio::Decoder::new(BufReader::new(Cursor::new(cursor.into_inner()))).unwrap().buffered();
    source 
}

// a custom QueueSource struct that can hold either a Vec<Buffered<Decoder<BufReader<std::fs::File>>>> or a Vec<Buffered<Decoder<BufReader<Cursor<Vec<u8>>>>>>

pub enum QueueSource {
    AudioFile(Buffered<Decoder<BufReader<std::fs::File>>>),
    Tone(Buffered<Decoder<BufReader<Cursor<Vec<u8>>>>>),
}

// create a random audio queue and return a valid rodio Source in a Vec to be played
// audio files from the audio_files HashMap will be randomly selected and played
// also take in the RandomAudioQueueOptions struct to determine how the audio queue should be generated
// if vec i + 1 has some other files, then also need to add the break_between_files duration
pub fn generate_random_audio_queue(audio_files: &HashMap<String, Buffered<Decoder<BufReader<std::fs::File>>>>, options: &RandomAudioQueueOptions, file_limit: u16) -> Vec<QueueSource> {
    // only generate the queue if there are audio files
    if audio_files.len() == 0 || file_limit == 0 {
        return Vec::new();
    }

    let mut rng = rand::thread_rng();
    let mut queue: Vec<QueueSource> = Vec::new();
    let files = audio_files.keys().collect::<Vec<&String>>();

    // randomly choose an audio file name from the list of audio files
    for _ in 0..file_limit { // generate the queue up to the file limit to avoid memory issues
        let random_audio_file_name = files.choose(&mut rng).unwrap();
        let source = audio_files.get(*random_audio_file_name).unwrap().clone();
        queue.push(QueueSource::AudioFile(source));
    }

    // add the break_between_files duration
    let silence_source = silence_as_source(&options.break_between_files, &48000);
    for i in 0..queue.len() {
        if i + 1 < queue.len() {
            queue.insert(i + 1, QueueSource::Tone(silence_source.clone()));
        }
    }

    queue
}

pub fn pause_sink_duration(sink: &rodio::Sink, duration: &u32) {
    // pause, sleep, then play
    sink.pause();
    std::thread::sleep(std::time::Duration::from_millis(*duration as u64));
    sink.play();
}