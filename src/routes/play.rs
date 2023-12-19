use std::{
    fs,
    fs::OpenOptions,
    io::Write,
};

use actix_web::{get, web, HttpResponse};
use rodio::{OutputStream, Sink, Source};

use crate::structs::{ResponseMessage, QueryStruct, AudioFiles};
use crate::audio::handle_audio_error;
use crate::LOG_FILE_NAME;


#[get("/play/{audio_file_name}")]
async fn play(audio_files: web::Data<AudioFiles> , audio_file_name: web::Path<String>, query: web::Query<QueryStruct>) -> HttpResponse {
    let time_ns = std::time::SystemTime::now().duration_since(std::time::SystemTime::UNIX_EPOCH).unwrap().as_nanos();
    println!("{}: Received /play/{}", time_ns, audio_file_name);

    let audio_file_name = audio_file_name.into_inner();
    let source = audio_files.files.get(&audio_file_name); // find decoded audio file by name

    // if the audio file is not found, return 404
    if source.is_none() {
        println!("\x1b[2m    \x1b[31mAudio file Not Found\x1b[0m");
        let message = format!("Audio file {} not found", audio_file_name);
        return HttpResponse::NotFound().json(ResponseMessage { message });
    }

    // Linux with ALSA will panic here if there is no audio output device available
    let output_stream_result = std::panic::catch_unwind(|| OutputStream::try_default());

    if output_stream_result.is_err() {
        return handle_audio_error(&audio_file_name, &query.time, "OutputStream NoDevice", &LOG_FILE_NAME);
    }

    let output_stream_result = output_stream_result.unwrap();

    // Windows somehow panics when unwraping the output_stream_result for the same reason (no audio output device available)
    if let Err(e) = output_stream_result {
        return handle_audio_error(&audio_file_name, &query.time, &e.to_string(), &LOG_FILE_NAME);
    }

    // now safe to unwrap
    let (_stream, stream_handle) = output_stream_result.unwrap();

    // print the source sample rate
    println!("\x1b[2m    \x1b[38;5;8mSource's Sample Rate: {} Hz\x1b[0m", source.unwrap().sample_rate());

    let sink = Sink::try_new(&stream_handle).unwrap();
    sink.append(source.unwrap().clone()); // init the sink with the audio file

    let time_start_nano = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos();
    
    println!("\x1b[2m    \x1b[38;5;8m{}: Started {}...\x1b[0m", time_start_nano, audio_file_name);
    sink.sleep_until_end(); // play the audio file synchronously. this thread will be blocked until the audio file has finished playing.
    println!("\x1b[2m    \x1b[38;5;8mFinished (job at {})\x1b[0m", time_start_nano);

    let message = format!("At {} played {}", time_start_nano, audio_file_name);
    drop(sink);
    drop(stream_handle);
    drop(_stream);
    
    // Append to the log file
    fs::create_dir_all("./logs").unwrap(); // make sure the logs/ folder exists first

    let log_file_name = LOG_FILE_NAME.lock().unwrap();
    let mut file = OpenOptions::new()
        .write(true)
        .append(true)
        .create(true)
        .open(format!("{}.csv", *log_file_name))
        .unwrap();
    if let Err(e) = writeln!(file, "{},{},{},{}", time_start_nano, audio_file_name, "success", &query.time) {
        eprintln!("Couldn't write to file: {}", e);
    } else {
        println!("\x1b[2m    \x1b[38;5;8mAppended to log file: {}\x1b[0m", *log_file_name);
    }

    drop(log_file_name); // release the lock
    drop(file); // release the lock

    HttpResponse::Ok().json(ResponseMessage { message })
}
