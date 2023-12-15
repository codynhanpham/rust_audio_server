# Audio Server, written in [Rust](https://www.rust-lang.org/)
A command-line utility which allows audio requests to be made to a server, which will then play the audio on the server's speakers.

Designed and **tested** to be compatible with Linux (Ubuntu 14.04 +) and Windows (10 +).

## Usage
### Server
The server must have some audio output device, such as speakers or headphones, connected to it.

To start the server, simply launch the executable included in the [release page](https://github.com/codynhanpham/rust_audio_server/releases) with the following command:
```bash
# navigate to the directory containing the executable
cd /path/to/folder/rust_audio_server-(platform)-(version)

# Windows: run the executable
rust-audio-server.exe # Windows (or just double click it)

# Linux: run the executable
./rust-audio-server # Linux

# if permission denied (or no such file or directory error), try chmod a+x first
chmod a+x ./rust-audio-server
./rust-audio-server # Linux
```

The default port is `5055`.

**The server must also have an `audio/` folder in the same directory as the executable**, which contains the audio files to be played. The only tested audio format is `.wav`, though in theory any format supported by [rodio](https://docs.rs/rodio/0.12.0/rodio/index.html) should work.

**Logs** are written to the `logs/` folder, which is created in the same directory as the executable. Logs are `csv` files, with these columns in order: `timestamp_audio` (UNIX nanosecond), `audio_filename`, `status` (either `success` or `error`), and `timestamp_client` (*anything from the client*).

</br>

### Client
The client can be run on any machine which can connect to the server via TCP.

There are 4 routes for the client:
#### GET `/play/:audio_filename`
Plays the audio file `audio_filename` on the server. The `audio_filename` must include the extension, and such a file must exist in the `audio/` folder on the server.

The server will display a message the moment the request is received, and log the exact time the audio file starts playing. The client will only receive a response once the audio file has finished playing.

The response is a `json` object with the following fields:
```json
{
  "message": "At {timestamp} played {audio_filename}"
}
```

*Example request:*
```bash
curl http://localhost:5055/play/doorbell.wav
```

##### Note

> A very optional parameter, `time`, is available for this route. You can indicate the request time (ideally in UNIX nanosecond) in the request, and the server will log it. This is useful if you want to compare the request time with the audio start time in the logs. ***THIS ASSUME THAT THE CLIENT AND SERVER ARE SYNCED TO THE SAME CLOCK / TIME***. If you are unsure of the time difference between the client and server, simply ignore this parameter, as well as the `timestamp_client` column in the logs.
>
> *Example request:*
> ```bash
> curl http://localhost:5055/play/doorbell.wav?time=1620000000000000000
> ```

</br>

#### GET `/startnewlog`
Start a new log file with the current **`UTC`** date time on the server. The response is a `json` object with the following fields:
```json
{
  "message": "Started new log file: ./{YYYYMMDD-hhmmss}.csv"
}
```

From this point on, all logs will be written to the new log file, until a new log file is started.

*Example request:*
```bash
curl http://localhost:5055/startnewlog
```

</br>

#### GET `/generate_batch_files`
Generate batch files for all audio files in the `audio/` folder. The batch files are `.bat` files for Windows. The request will be automatically filled with the current server local IP address, and the default port `5055`.

There will be one batch file for each audio file, and an extra `.bat` file that call the `/startnewlog` route. The request will return a `.zip` file containing all the batch files. The `.zip` file will be named with the IP address and port of the server: `{host_ip}_{port}.zip`.

> The batch files for requesting audio have the value for the optional query `?time=` included. It automatically pull the client clock time and use it as the request time. ***The accuracy of Windows clock is only to the millisecond, so the request time will be rounded to the nearest millisecond, then times 1,000,000 for `nanoseconds`***. The same assumption as the [Note](#note) above applies for the data to be meaningful: the client and server must be synced to the same clock. ***Please ignore the `timestamp_client` column in the logs if you are unsure of the time difference between the client and server.***

*Example request:*
```bash
curl -O -J http://localhost:5055/generate_batch_files

# -O to save the file to the current directory
# -J to use the filename from the header

# the file will be named something like:
# 192.168.1.1_5055.zip
```

</br>

#### GET `/generate_batch_files_async`
Similar to [`/generate_batch_files`](#get-generate_batch_files), but these new `.bat` files will fire the requests and then exit the process immediately without waiting for the response. This is useful if you want to fire a lot of requests at once, or if the batch files are called from another program.

In short, this version will send the request in a non-blocking manner. ***The timestamp of the client at the time of the request is also more accurate (to the tenth of a microsecond on Windows). Well, the same assumption as above, though.***

The `.zip` file will be named with the IP address and port of the server, along with the suffix `async` to differentiate it from the other version: `{host_ip}_{port}_async.zip`.

You will realize that the changes and improvements are thanks to the included `async_get.exe` file. **Please don't delete it...** But feel free to run that file on its own from the command line, if you want to test it out.

*Example request:*
```bash
curl -O -J http://localhost:5055/generate_batch_files_async
```

</br>

## Development and Build Instructions
To make sure that the executable is compatible with Ubuntu 14.04, compiling the code must be done on a machine with Ubuntu 14.04. This can be done by using a virtual machine, such as [VirtualBox](https://www.virtualbox.org/) or Hyper-V. Many low level libraries such as `glibc` or `alsa` are required and dynamically linked, so building on a newer version of Ubuntu will result in a binary that is not compatible with older versions of Ubuntu.

For this same reason, the version of the [rodio](https://docs.rs/rodio/0.12.0/rodio/index.html) crate used is locked at `0.12.0`. Please do not update this crate without testing on your target server OS.

</br>

In general, this project is extremely simple, and can be built with the following command:
```bash
cargo build --release
```

The executable will be located at `target/release/rust-audio-server`.
