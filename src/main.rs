use actix_web::{get, web, App, HttpServer, HttpResponse, Responder};
use std::{fs, panic};
use std::collections::HashMap;
use std::io::BufReader;
use rodio::{Decoder, OutputStream, Sink};
use rodio::source::{Source, Buffered};
use serde::Serialize;
use std::sync::{Arc, Mutex};
use chrono::Utc;
use std::fs::OpenOptions;
use std::io::Write;
use local_ip_address::local_ip;

// Define the global variable for the log file name
// This will be updated whenever a new /startnewlog request is received
lazy_static::lazy_static! {
    static ref LOG_FILE_NAME: Arc<Mutex<String>> = Arc::new(Mutex::new(Utc::now().format("logs/log_%Y%m%d-%H%M%S").to_string()));
}

// Define the port number
static PORT: u16 = 5055;


#[derive(Serialize)]
struct ResponseMessage {
    message: String,
}

struct AudioFiles {
    files: HashMap<String, Buffered<Decoder<BufReader<std::fs::File>>>>,
}

// Preload audio files to RAM for faster playback
// Returns a HashMap of audio file names and their corresponding Buffered<Decoder<BufReader<std::fs::File>>> objects
fn preload_audio_files(audio_folder_path: &str) -> HashMap<String, Buffered<Decoder<BufReader<std::fs::File>>>> {
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
    let audio_extensions = vec!["mp3", "wav", "flac", "ogg"];
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


fn create_batch_file(audio_file_name: &str, host_ip: &str, port: &str) -> String {
    let batch_file = format!(
        "@echo off\n\
        curl -X GET http://{}:{}/play/{}\n\
        exit\n",
        host_ip, port, audio_file_name
    );
    batch_file
}

/// ---------- APP & ROUTES ---------- ///

#[get("/")]
async fn index() -> impl Responder {
    "
    Available routes:
        - GET /ping\t\t\t --> pong
        - GET /play/{audio_file_name}\t --> play the audio file
        - GET /startnewlog\t\t --> start a new log file
        - GET /generate_batch_files\t --> generate a zip file containing batch files for all available audio files
    "
}


#[get("/ping")]
async fn ping() -> impl Responder {
    let time_ns = std::time::SystemTime::now().duration_since(std::time::SystemTime::UNIX_EPOCH).unwrap().as_nanos();
    println!("{}: Received /ping", time_ns);
    HttpResponse::Ok().body("pong")
}


#[get("/play/{audio_file_name}")]
async fn play(audio_files: web::Data<AudioFiles> , audio_file_name: web::Path<String>) -> HttpResponse {
    let time_ns = std::time::SystemTime::now().duration_since(std::time::SystemTime::UNIX_EPOCH).unwrap().as_nanos();
    println!("{}: Received /play/{}", time_ns, audio_file_name);

    let audio_file_name = audio_file_name.into_inner();
    let source = audio_files.files.get(&audio_file_name); // find decoded audio file by name

    // if the audio file is not found, return 404
    if source.is_none() {
        println!("\x1b[2m    \x1b[31;5;8mAudio file Not Found\x1b[0m");
        let message = format!("Audio file {} not found", audio_file_name);
        return HttpResponse::NotFound().json(ResponseMessage { message });
    }

    let output_stream_result = panic::catch_unwind(|| OutputStream::try_default());
    if output_stream_result.is_err() {
        println!("\x1b[2m    \x1b[31;5;8mError: Could not create OutputStream. Is there any audio output device available?\x1b[0m");
        return HttpResponse::InternalServerError().finish();
    }
    
    let (_stream, stream_handle) = output_stream_result.unwrap().unwrap();
    let sink_result = panic::catch_unwind(|| Sink::try_new(&stream_handle));
    if sink_result.is_err() {
        println!("\x1b[2m    \x1b[31;5;8mError: Could not create Sink. Is there any audio output device available?\x1b[0m");
        return HttpResponse::InternalServerError().finish();
    }
    
    let sink = sink_result.unwrap().unwrap();
    let source_result = panic::catch_unwind(|| source.unwrap().clone());
    if source_result.is_err() {
        println!("\x1b[2m    \x1b[31;5;8mError: Could not clone source. Is the source valid?\x1b[0m");
        return HttpResponse::InternalServerError().finish();
    }
    
    sink.append(source_result.unwrap()); // init the sink with the audio file
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
    if let Err(e) = writeln!(file, "{},{},{}", time_start_nano, audio_file_name, "success") {
        eprintln!("Couldn't write to file: {}", e);
    } else {
        println!("\x1b[2m    \x1b[38;5;8mAppended to log file: {}\x1b[0m", *log_file_name);
    }

    drop(log_file_name); // release the lock
    drop(file); // release the lock

    HttpResponse::Ok().json(ResponseMessage { message })
}


#[get("/startnewlog")]
async fn start_new_log() -> impl Responder {
    let time_ns = std::time::SystemTime::now().duration_since(std::time::SystemTime::UNIX_EPOCH).unwrap().as_nanos();
    println!("{}: Received /startnewlog", time_ns);

    let mut log_file_name = LOG_FILE_NAME.lock().unwrap();
    *log_file_name = Utc::now().format("logs/log_%Y%m%d-%H%M%S").to_string(); // update log file name

    // create new log file
    let mut file = OpenOptions::new()
        .write(true)
        .append(true)
        .create(true)
        .open(format!("{}.csv", *log_file_name))
        .unwrap();
    let mut message = format!("Started new log file: ./{}.csv", *log_file_name);
    drop(log_file_name);

    if let Err(e) = writeln!(file, "timestamp,audio_filename,status") {
        eprintln!("Couldn't create new file: {}", e);

        message = format!("Error: Couldn't create new file: {}", e);
    }

    drop(file);

    println!("\x1b[2m    \x1b[38;5;8m{}\x1b[0m", message);
    HttpResponse::Ok().json(ResponseMessage { message })
}


#[get("/generate_batch_files")]
async fn generate_batch_files(audio_files: web::Data<AudioFiles>) -> HttpResponse {
    let time_ns = std::time::SystemTime::now().duration_since(std::time::SystemTime::UNIX_EPOCH).unwrap().as_nanos();
    println!("{}: Received /generate_batch_files/", time_ns);

    let host_ip = local_ip().unwrap();
    let host_ip = host_ip.to_string();
    
    // create a zip file containing all the batch files
    let mut zip_file = zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));
    let options = zip::write::FileOptions::default().compression_method(zip::CompressionMethod::Stored);

    // add audio files
    for (audio_file_name, _) in audio_files.files.iter() {
        let batch_file = create_batch_file(audio_file_name, &host_ip, &PORT.to_string());
        zip_file.start_file(format!("{}.bat", audio_file_name), options).unwrap();
        zip_file.write_all(batch_file.as_bytes()).unwrap();
    }

    // add one that call /startnewlog as well for convenience
    let batch_file = format!(
        "@echo off\n\
        curl -X GET http://{}:{}/startnewlog\n\
        exit\n",
        host_ip, PORT
    );
    zip_file.start_file("startnewlog.bat", options).unwrap();
    zip_file.write_all(batch_file.as_bytes()).unwrap();

    // finish the zip file
    let zip_file = zip_file.finish().unwrap().into_inner();

    println!("\x1b[2m    \x1b[38;5;8mHost IP (this server): {} - Port: {}\x1b[0m", host_ip, PORT);
    println!("\x1b[2m    \x1b[38;5;8mGenerated batch files for {} audio files\x1b[0m", audio_files.files.len());

    // return the zip file
    HttpResponse::Ok()
        .content_type("application/zip")
        .append_header(("Content-Disposition", format!("attachment; filename=\"{}_{}.zip\"", host_ip, PORT)))
        .body(zip_file)
}


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
    if let Err(e) = writeln!(file, "Timestamp,Audio File Name,Status") {
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
            .service(index)
            .service(ping)
            .service(play)
            .service(start_new_log)
            .service(generate_batch_files)
    })
    .bind(("0.0.0.0", PORT))? // bind to all interfaces
    .run()
    .await
}