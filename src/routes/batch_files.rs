use actix_web::{get, web, HttpResponse};
use local_ip_address::local_ip;

use crate::structs::AudioFiles;
use crate::file_io::make_batch_zip_file;
use crate::{PORT, PLAYLISTS};


#[get("/generate_batch_files")]
async fn generate_batch_files(audio_files: web::Data<AudioFiles>) -> HttpResponse {
    let time_ns = std::time::SystemTime::now().duration_since(std::time::SystemTime::UNIX_EPOCH).unwrap().as_nanos();
    println!("{}: Received /generate_batch_files", time_ns);

    let host_ip = local_ip().unwrap();
    let host_ip = host_ip.to_string();

    // make a copy of the playlists for this request
    let playlists_global = PLAYLISTS.lock().unwrap();
    let playlists = playlists_global.playlists.clone();
    drop(playlists_global); // release the lock on PLAYLISTS global
    
    // create a zip file containing all the batch files
    let zip_file = make_batch_zip_file(&audio_files, &playlists, &host_ip, false);

    println!("\x1b[2m    \x1b[38;5;8mHost IP (this server): {} - Port: {}\x1b[0m", host_ip, PORT);
    println!("\x1b[2m    \x1b[38;5;8mGenerated batch files for {} audio files and {} playlists\x1b[0m", audio_files.files.len(), playlists.len());

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

    // make a copy of the playlists for this request
    let playlists_global = PLAYLISTS.lock().unwrap();
    let playlists = playlists_global.playlists.clone();
    drop(playlists_global); // release the lock on PLAYLISTS global
    
    // create a zip file containing all the batch files
    let zip_file = make_batch_zip_file(&audio_files, &playlists, &host_ip, true);

    println!("\x1b[2m    \x1b[38;5;8mHost IP (this server): {} - Port: {}\x1b[0m", host_ip, PORT);
    println!("\x1b[2m    \x1b[38;5;8mGenerated batch files for {} audio files and {} playlists\x1b[0m", audio_files.files.len(), playlists.len());

    // return the zip file
    HttpResponse::Ok()
        .content_type("application/zip")
        .append_header(("Content-Disposition", format!("attachment; filename=\"{}_{}_async.zip\"", host_ip, PORT)))
        .body(zip_file)
}