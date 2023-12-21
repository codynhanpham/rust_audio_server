use std::{collections::HashMap, io::BufReader};

use rodio::{source::Buffered, Decoder};
use serde::{Serialize, Deserialize};

use crate::audio::PlaylistTypes;


#[derive(Serialize)]
pub struct ResponseMessage {
    pub message: String,
}

#[derive(Deserialize)]
pub struct TimeQuery {
    // optional parameters
    #[serde(default)]
    pub time: String,
}

pub struct AudioFiles {
    pub files: HashMap<String, Buffered<Decoder<BufReader<std::fs::File>>>>,
}

pub struct Playlists {
    pub playlists: HashMap<String, Vec<PlaylistTypes>>
}

#[derive(Deserialize)]
pub struct Tone {
    pub freq: f32,
    pub duration: u32,
    pub amplitude: f32,
    pub sample_rate: u32,
}

#[derive(Deserialize)]
pub struct RandomAudioQueueOptions {
    // optional parameters
    #[serde(default)]
    pub break_between_files: u32, // in milliseconds
    #[serde(default)]
    pub file_count: u32, // number of files to play before stopping, overrides max_duration
}

#[derive(Deserialize)]
pub struct PlaylistOptions {
    // optional parameters
    #[serde(default)]
    pub break_between_files: u32, // in milliseconds
    #[serde(default)]
    pub file_count: u32, // number of files to play before stopping, overrides max_duration

    #[serde(default)] // this default to false --> download the file
    pub no_download: bool, // don't download the file, only create the playlist server-side
}