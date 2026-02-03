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

use splitter::frame::{Frame, PixelFormat, MAX_FRAME_SIZE};
use iceoryx2::prelude::*;

const SERVICE_NAME: &str = "camera/frames";

/// Remove stride padding from YUV420 (I420) frame data.
fn depad_yuv420(data: &[u8], width: u32, height: u32, stride: u32, out: &mut Vec<u8>) {
    out.clear();
    let w = width as usize;
    let h = height as usize;
    let s = stride as usize;

    // Y plane: height rows of stride bytes -> height rows of width bytes
    let y_plane = &data[..s * h];
    for row in 0..h {
        out.extend_from_slice(&y_plane[row * s..row * s + w]);
    }

    // U plane: height/2 rows of stride/2 bytes -> height/2 rows of width/2 bytes
    let u_offset = s * h;
    let u_plane = &data[u_offset..u_offset + (s / 2) * (h / 2)];
    for row in 0..(h / 2) {
        out.extend_from_slice(&u_plane[row * (s / 2)..row * (s / 2) + (w / 2)]);
    }

    // V plane: height/2 rows of stride/2 bytes -> height/2 rows of width/2 bytes
    let v_offset = u_offset + (s / 2) * (h / 2);
    let v_plane = &data[v_offset..v_offset + (s / 2) * (h / 2)];
    for row in 0..(h / 2) {
        out.extend_from_slice(&v_plane[row * (s / 2)..row * (s / 2) + (w / 2)]);
    }
}

/// Remove stride padding from NV12 frame data.
fn depad_nv12(data: &[u8], width: u32, height: u32, stride: u32, out: &mut Vec<u8>) {
    out.clear();
    let w = width as usize;
    let h = height as usize;
    let s = stride as usize;

    // Y plane: height rows of stride bytes -> height rows of width bytes
    let y_plane = &data[..s * h];
    for row in 0..h {
        out.extend_from_slice(&y_plane[row * s..row * s + w]);
    }

    // UV plane (interleaved): height/2 rows of stride bytes -> height/2 rows of width bytes
    let uv_offset = s * h;
    let uv_plane = &data[uv_offset..uv_offset + s * (h / 2)];
    for row in 0..(h / 2) {
        out.extend_from_slice(&uv_plane[row * s..row * s + w]);
    }
}

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
    let stride = first_frame.payload().stride;
    let format = first_frame.payload().format;

    let ffmpeg_pix_fmt = match format {
        PixelFormat::Yuv420 => "yuv420p",
        PixelFormat::Nv12 => "nv12",
        PixelFormat::Nv21 => "nv21",
        PixelFormat::Unknown => {
            eprintln!("Unknown pixel format, assuming nv12");
            "nv12"
        }
    };

    eprintln!("Got first frame: {}x{} (stride={}, format={:?}/{})", width, height, stride, format, ffmpeg_pix_fmt);

    // Spawn FFmpeg
    // Input: raw video from stdin
    // Output: RTSP to mediamtx
    let mut ffmpeg = Command::new("ffmpeg")
        .args([
            "-f", "rawvideo",
            "-pix_fmt", ffmpeg_pix_fmt,
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

    // Buffer for depadded frame data
    let mut depadded = Vec::with_capacity((width * height * 3 / 2) as usize);
    let needs_depad = stride != width;

    // Choose depad function based on format
    let depad_fn: fn(&[u8], u32, u32, u32, &mut Vec<u8>) = match format {
        PixelFormat::Yuv420 => depad_yuv420,
        _ => depad_nv12, // NV12, NV21, Unknown all use same layout
    };

    // Write first frame
    let payload = first_frame.payload();
    if needs_depad {
        depad_fn(&payload.data[..payload.len as usize], width, height, stride, &mut depadded);
        stdin.write_all(&depadded)?;
    } else {
        stdin.write_all(&payload.data[..payload.len as usize])?;
    }

    // Main loop: receive frames and pipe to FFmpeg
    let mut count = 0u64;
    loop {
        match subscriber.receive()? {
            Some(sample) => {
                let payload = sample.payload();
                let result = if needs_depad {
                    depad_fn(&payload.data[..payload.len as usize], width, height, stride, &mut depadded);
                    stdin.write_all(&depadded)
                } else {
                    stdin.write_all(&payload.data[..payload.len as usize])
                };
                if let Err(e) = result {
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
