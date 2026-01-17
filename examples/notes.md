# Examples

## Structure

```
examples/
├── webrtc_streamer.rs           # Rust: iceoryx2 → FFmpeg → mediamtx
└── cpp_subscriber/
    ├── CMakeLists.txt
    └── main.cpp                  # C++: iceoryx2 → OpenCV (skeleton)
```

## Usage

```bash
# Terminal 1: Start mediamtx (download from GitHub releases)
./mediamtx

# Terminal 2: Start camera publisher
cargo run --release

# Terminal 3: Start WebRTC streamer
cargo run --release --example webrtc_streamer

# Browser: Open http://<pi-ip>:8889/camera
```

## Data Flow

```
Camera DMA buffer
       ↓ (1 copy)
iceoryx2 shared memory
       ↓ (zero-copy read)     ↓ (zero-copy read)
  webrtc_streamer          cpp_subscriber
       ↓ (pipe)
  FFmpeg (H264)
       ↓ (RTSP)
    mediamtx
       ↓ (WebRTC)
    Browser
```

## Notes

- The C++ example is a skeleton - iceoryx2's C bindings require building `iceoryx2-ffi-c` separately
- mediamtx is a single binary, no config needed for basic use
- FFmpeg does the heavy lifting for encoding, no Rust crate complexity

## mediamtx Setup

Download from: https://github.com/bluenviron/mediamtx/releases

```bash
# Linux arm64 (Raspberry Pi)
wget https://github.com/bluenviron/mediamtx/releases/download/v1.9.3/mediamtx_v1.9.3_linux_arm64v8.tar.gz
tar xzf mediamtx_v1.9.3_linux_arm64v8.tar.gz
./mediamtx
```

Default ports:
- RTSP: 8554 (FFmpeg pushes here)
- WebRTC: 8889 (browser connects here)
