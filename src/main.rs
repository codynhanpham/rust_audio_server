use actix_web::{get, web, App, HttpServer, HttpResponse, Responder};
use std::fs;
use std::collections::HashMap;
use std::io::BufReader;
use rodio::{Decoder, OutputStream, Sink};
use rodio::source::{Source, Buffered};
use serde::Serialize;
use std::sync::{Arc, Mutex};
use chrono::Utc;
use std::fs::OpenOptions;
use std::io::Write;

// Define the global variable for the log file name
// This will be updated whenever a new /startnewlog request is received
lazy_static::lazy_static! {
    static ref LOG_FILE_NAME: Arc<Mutex<String>> = Arc::new(Mutex::new(Utc::now().format("logs/log_%Y%m%d-%H%M%S").to_string()));
}


#[derive(Serialize)]
struct ResponseMessage {
    message: String,
}

struct AudioFiles {
    files: HashMap<String, Buffered<Decoder<BufReader<std::fs::File>>>>,
}

// Preload audio files to RAM for faster playback
// Returns a HashMap of audio file names and their corresponding Buffered<Decoder<BufReader<std::fs::File>>> objects
fn preload_audio_files(audio_folder_path: &str) -> HashMap<String, Buffered<Decoder<BufReader<std::fs::File>>>> {
    println!("Preloading audio files...");
    let mut files = HashMap::new();
    let paths = fs::read_dir(audio_folder_path).unwrap();
    let audio_extensions = vec!["mp3", "wav", "flac", "ogg"];
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


#[get("/")]
async fn index() -> impl Responder {
    "
    Available routes:
        - GET /play/{audio_file_name}
    "
}


#[get("/ping")]
async fn ping() -> impl Responder {
    let time_ns = std::time::SystemTime::now().duration_since(std::time::SystemTime::UNIX_EPOCH).unwrap().as_nanos();
    println!("{}: Received /ping", time_ns);
    HttpResponse::Ok().json(ResponseMessage { message: "pong".to_string() })
}


#[get("/play/{audio_file_name}")]
async fn play(audio_files: web::Data<AudioFiles> , audio_file_name: web::Path<String>) -> HttpResponse {
    let time_ns = std::time::SystemTime::now().duration_since(std::time::SystemTime::UNIX_EPOCH).unwrap().as_nanos();
    println!("{}: Received /play/{}", time_ns, audio_file_name);

    let audio_file_name = audio_file_name.into_inner();
    let source = audio_files.files.get(&audio_file_name); // find decoded audio file by name

    // if the audio file is not found, return 404
    if let None = source {
        println!("\x1b[2m    \x1b[38;5;8mAudio file Not Found\x1b[0m");
        let message = format!("Audio file {} not found", audio_file_name);
        return HttpResponse::NotFound().json(ResponseMessage { message });
    }

    // if the audio file is found, try to play it
    let source = source.unwrap();
    let (_stream, stream_handle) = OutputStream::try_default().unwrap();
    let sink = Sink::try_new(&stream_handle).unwrap();
    sink.append(source.clone()); // init the sink with the audio file
    let time_start_nano = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos();
    
    println!("\x1b[2m    \x1b[38;5;8m{}: Started {}...\x1b[0m", time_start_nano, audio_file_name);
    sink.sleep_until_end(); // play the audio file synchronously. this thread will be blocked until the audio file is finished playing.
    println!("\x1b[2m    \x1b[38;5;8mFinished (job at {})\x1b[0m", time_start_nano);

    let message = format!("At {} played {}", time_start_nano, audio_file_name);
    drop(sink);
    drop(stream_handle);
    drop(_stream);
    
    // Append to the log file
    let log_file_name = LOG_FILE_NAME.lock().unwrap();
    let mut file = OpenOptions::new()
        .write(true)
        .append(true)
        .create(true)
        .open(format!("{}.csv", *log_file_name))
        .unwrap();
    if let Err(e) = writeln!(file, "{},{},{}", time_start_nano, audio_file_name, "success") {
        eprintln!("Couldn't write to file: {}", e);
    }
    println!("\x1b[2m    \x1b[38;5;8mAppended to log file: {}\x1b[0m", *log_file_name);

    drop(log_file_name);
    drop(file);

    // NOT IMPLEMENTED: return error if server cannot play the audio file for some reason

    HttpResponse::Ok().json(ResponseMessage { message })
}


#[get("/startnewlog")]
async fn start_new_log() -> impl Responder {
    let time_ns = std::time::SystemTime::now().duration_since(std::time::SystemTime::UNIX_EPOCH).unwrap().as_nanos();
    println!("{}: Received /startnewlog", time_ns);

    let mut log_file_name = LOG_FILE_NAME.lock().unwrap();
    *log_file_name = Utc::now().format("logs/log_%Y%m%d-%H%M%S").to_string(); // update log file name

    // create new log file
    let mut file = OpenOptions::new()
        .write(true)
        .append(true)
        .create(true)
        .open(format!("{}.csv", *log_file_name))
        .unwrap();
    if let Err(e) = writeln!(file, "timestamp,audio_filename,status") {
        eprintln!("Couldn't create new file: {}", e);
    }

    let message = format!("Started new log file: ./{}.csv", *log_file_name);
    drop(log_file_name);
    drop(file);

    println!("\x1b[2m    \x1b[38;5;8m{}\x1b[0m", message);
    HttpResponse::Ok().json(ResponseMessage { message })
}



#[actix_web::main]
async fn main() -> std::io::Result<()> {
    println!(" ---------- AUDIO SERVER ----------\n");
    println!("Looking for audio files in ./audio/*.wav ...");

    // preload audio files
    let audio_files = web::Data::new(AudioFiles {
        files: preload_audio_files("./audio"),
    });

    // make sure the log folder exists
    fs::create_dir_all("./logs").unwrap();

    // init a new log file name with the current date time
    let mut log_file_name = LOG_FILE_NAME.lock().unwrap();
    *log_file_name = Utc::now().format("logs/log_%Y%m%d-%H%M%S").to_string(); // update log file name
    
    
    // create new log file
    let mut file = OpenOptions::new()
        .write(true)
        .append(true)
        .create(true)
        .open(format!("{}.csv", *log_file_name))
        .unwrap();
    if let Err(e) = writeln!(file, "Timestamp,Audio File Name,Status") {
        eprintln!("Couldn't create new file: {}", e);
    }
    println!("Started new log file: ./{}.csv\n", *log_file_name);

    drop(log_file_name); // release the lock
    drop(file); // release the lock


    // start the server
    println!("Server running at http://127.0.0.1:5055/\n\n");
    HttpServer::new(move || {
        App::new()
            .app_data(audio_files.clone())
            .service(index)
            .service(ping)
            .service(play)
            .service(start_new_log)
    })
    .bind(("0.0.0.0", 5055))? // bind to all interfaces
    .run()
    .await
}