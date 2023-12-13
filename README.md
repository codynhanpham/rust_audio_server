# Audio Server, written in [Rust](https://www.rust-lang.org/)
A command-line utility which allows audio requests to be made to a server, which will then play the audio on the server's speakers.

Designed and **tested** to be compatible with Linux (Ubuntu 14.04 +) and Windows (10 +).

## Usage
### Server
The server must have some audio output device, such as speakers or headphones, connected to it.

To start the server, simply launch the executable included in the [release page](https://github.com/codynhanpham/rust-audio-server/releases) with the following command:
```bash
./start-audio-server # Linux
start-audio-server.exe # Windows
```

The default port is `5055`.

**The server must also have an `audio/` folder in the same directory as the executable**, which contains the audio files to be played. The only tested audio format is `.wav`, though in theory any format supported by [rodio](https://docs.rs/rodio/0.12.0/rodio/index.html) should work.

**Logs** are written to the `logs/` folder, which is created in the same directory as the executable. Logs are `csv` files, with these columns in order: `timestamp` (UNIX nanosecond), `audio_filename`, and `status` (either `success` or `error`).

</br>

### Client
The client can be run on any machine which can connect to the server via TCP.

There are 2 routes for the client:
#### `GET /play/:audio_filename`
Plays the audio file `audio_filename` on the server. The `audio_filename` must include the extension, and such a file must exist in the `audio/` folder on the server.

The server will display a message the moment the request is received, and log the exact time the audio file starts playing. The client will only receive a response once the audio file has finished playing.

The response is a `json` object with the following fields:
```json
{
  "message": "At {timestamp} played {audio_filename}"
}
```

Example request:
```bash
curl http://localhost:5055/play/doorbell.wav
```


#### `GET /startnewlog`
Start a new log file with the current `UTC` date time. The response is a `json` object with the following fields:
```json
{
  "message": "Started new log file: ./{YYYYMMDD-hhmmss}.csv"
}
```

From this point on, all logs will be written to the new log file, until a new log file is started.


## Development and Build Instructions
To make sure that the executable is compatible with Ubuntu 14.04, compiling the code must be done on a machine with Ubuntu 14.04. This can be done by using a virtual machine, such as [VirtualBox](https://www.virtualbox.org/) or Hyper-V. Many low level libraries such as `glibc` or `alsa` are required and dynamically linked, so building on a newer version of Ubuntu will result in a binary that is not compatible with older versions of Ubuntu.

For this same reason, the version of the [rodio](https://docs.rs/rodio/0.12.0/rodio/index.html) crate used is locked at `0.12.0`. Please do not update this crate without testing on your target server OS.

</br>

In general, this project is extremely simple, and can be built with the following command:
```bash
cargo build --release
```

The executable will be located at `target/release/rust-audio-server`.