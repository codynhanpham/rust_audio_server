use std::{
    fs,
    fs::OpenOptions,
    io::Write,
};

use actix_web::{get, web, HttpResponse};
use rodio::{OutputStream, Sink, Source};

use crate::structs::{ResponseMessage, TimeQuery, Tone};
use crate::audio::{tone_to_source, tone_to_wav_file, handle_audio_error};
use crate::LOG_FILE_NAME;

#[get("/tone/{freq}/{duration}/{amplitude}/{sample_rate}")]
async fn play_tone(tone: web::Path<Tone>, query: web::Query<TimeQuery>) -> HttpResponse {
    let time_ns = std::time::SystemTime::now().duration_since(std::time::SystemTime::UNIX_EPOCH).unwrap().as_nanos();

    // destructure the Tone struct to get the values
    let Tone { freq, duration, amplitude, sample_rate } = tone.into_inner();

    println!("{}: Received /tone/{}/{}/{}/{}", time_ns, freq, duration, amplitude, sample_rate);

    // create a pure sine tone
    let source = tone_to_source(&freq, &duration, &amplitude, &sample_rate);

    // Linux with ALSA will panic here if there is no audio output device available
    let output_stream_result = std::panic::catch_unwind(|| OutputStream::try_default());

    if output_stream_result.is_err() {
        return handle_audio_error(&freq.to_string(), &duration.to_string(), "OutputStream NoDevice", &LOG_FILE_NAME);
    }

    let output_stream_result = output_stream_result.unwrap();

    // Windows somehow panics when unwraping the output_stream_result for the same reason (no audio output device available)
    if let Err(e) = output_stream_result {
        return handle_audio_error(&freq.to_string(), &duration.to_string(), &e.to_string(), &LOG_FILE_NAME);
    }

    // now safe to unwrap
    let (_stream, stream_handle) = output_stream_result.unwrap();

    // print the source sample rate
    println!("\x1b[2m    \x1b[38;5;8mSource's Sample Rate: {} Hz\x1b[0m", source.sample_rate());

    let sink = Sink::try_new(&stream_handle).unwrap();
    sink.append(source); // init the sink with the audio file

    let audio_file_name = format!("tone_{}Hz_{}ms_{}dB_@{}Hz", freq, duration, amplitude, sample_rate);

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

#[get("/save_tone/{freq}/{duration}/{amplitude}/{sample_rate}")]
async fn save_tone(tone: web::Path<Tone>) -> HttpResponse {
    let time_ns = std::time::SystemTime::now().duration_since(std::time::SystemTime::UNIX_EPOCH).unwrap().as_nanos();

    // destructure the Tone struct to get the values
    let Tone { freq, duration, amplitude, sample_rate } = tone.into_inner();

    println!("{}: Received /save_tone/{}/{}/{}/{}", time_ns, freq, duration, amplitude, sample_rate);

    // create a wav file and send it to the client for download
    let wav_file = tone_to_wav_file(&freq, &duration, &amplitude, &sample_rate);

    let audio_file_name = format!("{}Hz_{}ms_{}dB_@{}Hz", freq, duration, amplitude, sample_rate);

    // send as a wav file
    HttpResponse::Ok()
        .content_type("audio/wav")
        .append_header(("Content-Disposition", format!("attachment; filename={}.wav", audio_file_name)))
        .body(wav_file)
}
