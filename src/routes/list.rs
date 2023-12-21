use actix_web::{get, web, HttpResponse};

use crate::structs::AudioFiles;
use crate::PLAYLISTS;


#[get("/list")]
async fn list(audio_files: web::Data<AudioFiles>) -> HttpResponse {
    let time_ns = std::time::SystemTime::now().duration_since(std::time::SystemTime::UNIX_EPOCH).unwrap().as_nanos();
    println!("{}: Received /list", time_ns);

    let mut audio_file_names = String::new();
    let mut playlist_names = String::new();

    // Audio files name keys
    let mut audio_file_name_keys: Vec<&String> = audio_files.files.keys().collect();
    audio_file_name_keys.sort();

    // Playlist name keys
    let playlists = PLAYLISTS.lock().unwrap();
    // clone the keys and release the lock on PLAYLISTS global
    let mut playlist_name_keys: Vec<String> = playlists.playlists.keys().cloned().collect();
    drop(playlists);
    playlist_name_keys.sort();


    // Audio files
    for audio_file_name_key in audio_file_name_keys {
        audio_file_names.push_str(&format!("\t\t\t\t{}\n", audio_file_name_key));
    }

    // Playlists
    for playlist_name_key in &playlist_name_keys {
        playlist_names.push_str(&format!("\t\t\t\t{}\n", &playlist_name_key));
    }

    HttpResponse::Ok().body(format!("\n\tAudio files ({}):\n\n{}\n\n\n\n\tPlaylists ({}):\n\n{}\n\n", audio_files.files.len(), audio_file_names, playlist_name_keys.len(), playlist_names))
}