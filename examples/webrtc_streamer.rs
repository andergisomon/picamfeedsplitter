//! WebRTC streamer via FFmpeg + mediamtx
//!
//! Subscribes to camera frames from iceoryx2, pipes to FFmpeg for H264 encoding,
//! which outputs RTSP to mediamtx. mediamtx then serves WebRTC to browsers.
//!
//! # Prerequisites
//!
//! 1. Install FFmpeg: `apt install ffmpeg`
//! 2. Install mediamtx: https://github.com/bluenviron/mediamtx/releases
//!
//! # Usage
//!
//! Terminal 1: Start mediamtx
//!   $ ./mediamtx
//!
//! Terminal 2: Start the publisher (main binary)
//!   $ cargo run
//!
//! Terminal 3: Start this streamer
//!   $ cargo run --example webrtc_streamer
//!
//! Browser: Open http://<pi-ip>:8889/camera to view WebRTC stream

use std::io::Write;
use std::process::{Command, Stdio};

use splitter::frame::{Frame, MAX_FRAME_SIZE};
use iceoryx2::prelude::*;

const SERVICE_NAME: &str = "camera/frames";

fn main() -> Result<(), Box<dyn std::error::Error>> {
    eprintln!("WebRTC streamer starting...");

    // Subscribe to camera frames
    let node = NodeBuilder::new().create::<ipc::Service>()?;

    let service = node
        .service_builder(&SERVICE_NAME.try_into().unwrap())
        .publish_subscribe::<Frame>()
        .open()?;

    let subscriber = service.subscriber_builder().create()?;

    eprintln!("Subscribed to {}", SERVICE_NAME);

    // Wait for first frame to get dimensions
    eprintln!("Waiting for first frame...");
    let first_frame = loop {
        if let Some(sample) = subscriber.receive()? {
            break sample;
        }
        std::thread::sleep(std::time::Duration::from_millis(10));
    };

    let width = first_frame.payload().width;
    let height = first_frame.payload().height;
    eprintln!("Got first frame: {}x{}", width, height);

    // Spawn FFmpeg
    // Input: raw YUV420 from stdin
    // Output: RTSP to mediamtx
    let mut ffmpeg = Command::new("ffmpeg")
        .args([
            "-f", "rawvideo",
            "-pix_fmt", "yuv420p",
            "-s", &format!("{}x{}", width, height),
            "-r", "30",
            "-i", "-",  // stdin
            "-c:v", "libx264",
            "-preset", "ultrafast",
            "-tune", "zerolatency",
            "-g", "30",  // keyframe every 30 frames
            "-f", "rtsp",
            "-rtsp_transport", "tcp",
            "rtsp://127.0.0.1:8554/camera",
        ])
        .stdin(Stdio::piped())
        .stderr(Stdio::inherit())
        .spawn()?;

    let mut stdin = ffmpeg.stdin.take().expect("Failed to open FFmpeg stdin");

    eprintln!("FFmpeg started, streaming to rtsp://127.0.0.1:8554/camera");
    eprintln!("Open http://<pi-ip>:8889/camera in browser for WebRTC");

    // Write first frame
    let payload = first_frame.payload();
    stdin.write_all(&payload.data[..payload.len as usize])?;

    // Main loop: receive frames and pipe to FFmpeg
    let mut count = 0u64;
    loop {
        match subscriber.receive()? {
            Some(sample) => {
                let payload = sample.payload();
                if let Err(e) = stdin.write_all(&payload.data[..payload.len as usize]) {
                    eprintln!("FFmpeg pipe closed: {}", e);
                    break;
                }
                count += 1;
                if count % 100 == 0 {
                    eprintln!("Streamed {} frames", count);
                }
            }
            None => {
                // No frame available, brief sleep
                std::thread::sleep(std::time::Duration::from_millis(1));
            }
        }
    }

    drop(stdin);
    ffmpeg.wait()?;
    Ok(())
}
