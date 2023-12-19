use std::{
    collections::HashMap,
    fs,
    fs::OpenOptions,
    io::{BufReader, Write, Cursor},
};

use actix_web::HttpResponse;
use hound;
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

    // write to disk for debugging
    // let file_name = format!("{}Hz_{}ms_{}dB_@{}Hz.wav", freq, duration, amplitude, sample_rate);
    // let mut file = std::fs::File::create(file_name).unwrap();
    // file.write_all(&cursor.clone().into_inner()).unwrap();


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