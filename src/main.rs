use std::{
    fs,
    fs::OpenOptions,
    io::Write,
    sync::{Arc, Mutex},
};

use actix_web::{web, App, HttpServer};
use chrono::Utc;
use local_ip_address::local_ip;

mod structs;
use structs::AudioFiles;

mod audio;
use audio::preload_audio_files;

mod file_io;
mod routes;



// Define the global variable for the log file name
// This will be updated whenever a new /startnewlog request is received
lazy_static::lazy_static! {
    static ref LOG_FILE_NAME: Arc<Mutex<String>> = Arc::new(Mutex::new(Utc::now().format("logs/log_%Y%m%d-%H%M%S").to_string()));
}

// Define the port number
static PORT: u16 = 5055;



/// ---------- APP & ROUTES ---------- ///

// See individual route functions in src/routes/*.rs

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
    if let Err(e) = writeln!(file, "timestamp_audio,audio_filename,status,timestamp_client") {
        eprintln!("Couldn't create new file: {}", e);
    } else {
        println!("Started new log file: ./{}.csv\n", *log_file_name);
    }

    drop(log_file_name); // release the lock
    drop(file); // release the lock


    // start the server
    let host_ip = local_ip().unwrap();
    let host_ip = host_ip.to_string();
    println!("Server running at http://{}:{}/\n\n", host_ip, PORT);
    HttpServer::new(move || {
        App::new()
            .app_data(audio_files.clone())
            .service(routes::info::index)
            .service(routes::ping::ping)
            .service(routes::play::play)
            .service(routes::tone::play_tone)
            .service(routes::tone::save_tone)
            .service(routes::startnewlog::start_new_log)
            .service(routes::batch_files::generate_batch_files)
            .service(routes::batch_files::generate_batch_files_async)
    })
    .bind(("0.0.0.0", PORT))? // bind to all interfaces
    .run()
    .await
}