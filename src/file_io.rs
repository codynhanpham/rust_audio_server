use std::io::Write;
use std::collections::HashMap;

use actix_web::web;

use crate::{structs::AudioFiles, audio::PlaylistTypes, PORT};


// Create a batch file for Windows. Edit this template to change the batch file content
fn create_batch_file_audio(audio_file_name: &str, host_ip: &str, port: &str, with_async: bool) -> String {
    if with_async {
        let batch_file = format!(
            "@echo off\n\
            .\\async_get.exe -u http://{}:{}/play/{}\n\
            exit\n",
            host_ip, port, audio_file_name
        );
        return batch_file;
    } 
    else {
        let batch_file = format!(
            "@echo off\n\
            <!-- :\n\
            for /f \"tokens=* usebackq\" %%a in (`start /b cscript //nologo \"%~f0?.wsf\"`) do (set timestamp=%%a)\n\
            curl -X GET http://{}:{}/play/{}?time=%timestamp%000000\n\
            exit /b\n\
            -->\n\
            \n\
            <job><script language=\"JavaScript\">\n\
            WScript.Echo(new Date().getTime());\n\
            </script></job>\n",
            host_ip, port, audio_file_name
        );
        return batch_file;
    }
    // batch_file
}

fn create_batch_file_playlist(playlist_name: &str, host_ip: &str, port: &str, with_async: bool) -> String {
    if with_async {
        let batch_file = format!(
            "@echo off\n\
            .\\async_get.exe -u http://{}:{}/playlist/{}\n\
            exit\n",
            host_ip, port, playlist_name
        );
        return batch_file;
    } 
    else {
        let batch_file = format!(
            "@echo off\n\
            <!-- :\n\
            for /f \"tokens=* usebackq\" %%a in (`start /b cscript //nologo \"%~f0?.wsf\"`) do (set timestamp=%%a)\n\
            curl -X GET http://{}:{}/playlist/{}?time=%timestamp%000000\n\
            exit /b\n\
            -->\n\
            \n\
            <job><script language=\"JavaScript\">\n\
            WScript.Echo(new Date().getTime());\n\
            </script></job>\n",
            host_ip, port, playlist_name
        );
        return batch_file;
    }
    // batch_file
}


pub fn make_batch_zip_file(audio_files: &web::Data<AudioFiles>, playlists: &HashMap<String, Vec<PlaylistTypes>>, host_ip: &str, with_async: bool) -> Vec<u8> {
    // create a zip file containing all the batch files
    let mut zip_file = zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));
    let options = zip::write::FileOptions::default().compression_method(zip::CompressionMethod::Stored);

    // add audio files
    for (audio_file_name, _) in audio_files.files.iter() {
        let batch_file = create_batch_file_audio(audio_file_name, &host_ip, &PORT.to_string(), with_async);
        zip_file.start_file(format!("{}.bat", audio_file_name), options).unwrap();
        zip_file.write_all(batch_file.as_bytes()).unwrap();
    }

    // add playlists
    for (playlist_name, _) in playlists.iter() {
        let batch_file = create_batch_file_playlist(playlist_name, &host_ip, &PORT.to_string(), with_async);
        zip_file.start_file(format!("{}.bat", playlist_name), options).unwrap();
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

    // if with_async, also bundle the async_get.exe file
    if with_async {
        let async_get_file = include_bytes!("./async_get.exe");
        zip_file.start_file("async_get.exe", options).unwrap();
        zip_file.write_all(async_get_file).unwrap();
    }

    // finish the zip file
    let zip_file = zip_file.finish().unwrap().into_inner();
    zip_file
}