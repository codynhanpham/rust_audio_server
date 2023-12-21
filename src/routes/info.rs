use actix_web::{get, Responder};
#[get("/")]
async fn index() -> impl Responder {        
    "
    Available routes:
        - GET /ping                         --> pong
                (eg. /ping ==> pong)


        - GET /list                         --> list all available audio files and playlists
                (eg. /list ==> Audio files: ... Playlists: ...)


        - GET /startnewlog                  --> start a new log file
                (eg. /startnewlog ==> Started new log file: ./logs/log_20210321-171234.csv)

                
        - GET /play/{audio_file_name}       --> play the audio file
                (eg. /play/1.wav ==> 1.wav started playing on the server)


        - GET /tone/{freq}/{duration}/{amplitude}/{sample_rate} 
                                            --> play a pure sine tone
                (eg. /tone/1000/500/40/96000 ==> 1000Hz tone started playing on the server for 500ms at 40dB)


        - GET /save_tone/{freq}/{duration}/{amplitude}/{sample_rate}
                                            --> create a .wav file of a pure sine tone
                (eg. /save_tone/1000/500/40/96000 ==> generate file 1000Hz_500ms_40dB_@96000Hz.wav to download)


        - GET /play/random                  --> play some random audio files. 2 optional parameters:
                - break_between_files (in milliseconds, default = 0)
                - file_count (number of files to play, default = 100)
                (eg. /play/random?break_between_files=1000&file_count=10 ==> 10 random files started playing on the server)


        - GET /playlist/create              --> create a random playlist with available audio files. 3 optional parameters:
                - break_between_files (in milliseconds, default = 0)
                - file_count (number of files to play, default = 100)
                - no_download (don't download the files, only create the playlist server-side and return the new playlist name, default = false)
                (eg. /playlist/create?break_between_files=1000&file_count=10 ==> random playlist: playlist_{hash}_{duration}s_{size}count.txt to download)
                (eg. /playlist/create?break_between_files=1000&file_count=20&no_download=true ==> random playlist: playlist_{hash}_{duration}s_{size}count.txt created on the server)
                
                * The {hash} is the first 8 characters of the SHA256 hash of the playlist file. This serves as a unique identifier for the playlist, so that no two duplicate playlists are created.


        - GET /playlist/{playlist_name}     --> play a playlist on the server
                (eg. /playlist/playlist_file.txt ==> playlist_file.txt started playing on the server)


        - GET /generate_batch_files         --> generate a .zip containing batch files to request the audio files and playlists (close when audio file is finished playing)
                (eg. /generate_batch_files ==> ZIP file to download)


        - GET /generate_batch_files_async   --> generate a .zip containing batch files to request the audio files and playlists (asynchronous, close immediately)
                (eg. /generate_batch_files_async ==> ZIP file to download)
        


    Note:
        - The batch files generated by /generate_batch_files and /generate_batch_files_async are for Windows only.

        - For /tone, freq is in Hz, duration is in milliseconds, amplitude is in dB, and sample_rate is in Hz.

        - /playlist playback is not truely gapless. There is a small gap between files, not too significant, though.

        - /play/random and /playlist will always create a new log file for that session playback. The log file will contain \"playrandom\" or \"playlist\" in the file name.

        - /playlist/create will also hot reload the playlists folder, so you can create a new playlist and play it right away.

    "
}