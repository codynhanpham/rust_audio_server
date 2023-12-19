use actix_web::{get, web, HttpResponse};
use local_ip_address::local_ip;

use crate::structs::AudioFiles;
use crate::file_io::make_batch_zip_file;
use crate::PORT;


#[get("/generate_batch_files")]
async fn generate_batch_files(audio_files: web::Data<AudioFiles>) -> HttpResponse {
    let time_ns = std::time::SystemTime::now().duration_since(std::time::SystemTime::UNIX_EPOCH).unwrap().as_nanos();
    println!("{}: Received /generate_batch_files", time_ns);

    let host_ip = local_ip().unwrap();
    let host_ip = host_ip.to_string();
    
    // create a zip file containing all the batch files
    let zip_file = make_batch_zip_file(&audio_files, &host_ip, false);

    println!("\x1b[2m    \x1b[38;5;8mHost IP (this server): {} - Port: {}\x1b[0m", host_ip, PORT);
    println!("\x1b[2m    \x1b[38;5;8mGenerated batch files for {} audio files\x1b[0m", audio_files.files.len());

    // return the zip file
    HttpResponse::Ok()
        .content_type("application/zip")
        .append_header(("Content-Disposition", format!("attachment; filename=\"{}_{}.zip\"", host_ip, PORT)))
        .body(zip_file)
}


#[get("/generate_batch_files_async")]
async fn generate_batch_files_async(audio_files: web::Data<AudioFiles>) -> HttpResponse {
    let time_ns = std::time::SystemTime::now().duration_since(std::time::SystemTime::UNIX_EPOCH).unwrap().as_nanos();
    println!("{}: Received /generate_batch_files_async", time_ns);

    let host_ip = local_ip().unwrap();
    let host_ip = host_ip.to_string();
    
    // create a zip file containing all the batch files
    let zip_file = make_batch_zip_file(&audio_files, &host_ip, true);

    println!("\x1b[2m    \x1b[38;5;8mHost IP (this server): {} - Port: {}\x1b[0m", host_ip, PORT);
    println!("\x1b[2m    \x1b[38;5;8mGenerated batch files for {} audio files\x1b[0m", audio_files.files.len());

    // return the zip file
    HttpResponse::Ok()
        .content_type("application/zip")
        .append_header(("Content-Disposition", format!("attachment; filename=\"{}_{}_async.zip\"", host_ip, PORT)))
        .body(zip_file)
}