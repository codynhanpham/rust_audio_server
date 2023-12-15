use std::{
    collections::HashMap,
    fs,
    fs::OpenOptions,
    io::{BufReader, Write},
};

use actix_web::HttpResponse;
use rodio::{
    source::Buffered,
    Decoder,
    Source,
};

use crate::structs::ResponseMessage;


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