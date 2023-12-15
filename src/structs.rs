use std::{collections::HashMap, io::BufReader};

use rodio::{source::Buffered, Decoder};
use serde::{Serialize, Deserialize};


#[derive(Serialize)]
pub struct ResponseMessage {
    pub message: String,
}

#[derive(Deserialize)]
pub struct QueryStruct {
    // optional parameters
    #[serde(default)]
    pub time: String,
}

pub struct AudioFiles {
    pub files: HashMap<String, Buffered<Decoder<BufReader<std::fs::File>>>>,
}