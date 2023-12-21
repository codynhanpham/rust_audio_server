use std::{
    fs,
    fs::OpenOptions,
    io::Write,
};

use actix_web::{get, HttpResponse, Responder};
use chrono::Utc;

use crate::structs::ResponseMessage;
use crate::LOG_FILE_NAME;



#[get("/startnewlog")]
async fn start_new_log() -> impl Responder {
    let time_ns = std::time::SystemTime::now().duration_since(std::time::SystemTime::UNIX_EPOCH).unwrap().as_nanos();
    println!("{}: Received /startnewlog", time_ns);

    // make sure the log folder exists
    fs::create_dir_all("./logs").unwrap();

    let mut log_file_name = LOG_FILE_NAME.lock().unwrap();
    *log_file_name = Utc::now().format("logs/log_%Y%m%d-%H%M%S").to_string(); // update log file name

    // create new log file
    let mut file = OpenOptions::new()
        .write(true)
        .append(true)
        .create(true)
        .open(format!("{}.csv", *log_file_name))
        .unwrap();
    let mut message = format!("Started new log file: ./logs/{}.csv", *log_file_name);
    drop(log_file_name);

    if let Err(e) = writeln!(file, "timestamp_audio,audio_filename,status,timestamp_client") {
        eprintln!("Couldn't create new file: {}", e);

        message = format!("Error: Couldn't create new file: {}", e);
    }

    drop(file);

    println!("\x1b[2m    \x1b[38;5;8m{}\x1b[0m", message);
    HttpResponse::Ok().json(ResponseMessage { message })
}