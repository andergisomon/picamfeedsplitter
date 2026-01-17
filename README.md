# Pi Camera Feed Splitter

Zero-copy camera feed distribution for Raspberry Pi 5 with Pi Camera v3.

## Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                        Raspberry Pi 5                            │
├─────────────────────────────────────────────────────────────────┤
│                                                                  │
│   libcamera ──→ Shared Memory Pool ──┬──→ C++ OpenCV Service    │
│                   (iceoryx2)         │    (zero-copy, separate  │
│                                      │     process)             │
│                                      │                          │
│   (same process)                     └──→ OpenH264 ──→ WebRTC   │
│                                          (software      ↓       │
│                                           encoder)    WLAN      │
└─────────────────────────────────────────────────────────────────┘
                                                         │
                                                         ↓
┌─────────────────────────────────────────────────────────────────┐
│                          Laptop                                  │
├─────────────────────────────────────────────────────────────────┤
│                     WebRTC Consumer                              │
│                   (web/viewer.html)                              │
└─────────────────────────────────────────────────────────────────┘
```

## Key Design Decisions

1. **No hardware encoder** - Pi 5 removed the hardware H.264 encoder. We use Cisco's OpenH264 (software) instead.

2. **No ffmpeg subprocess** - OpenH264 is linked directly via the `openh264` crate. No pipe overhead, no subprocess management.

3. **C++ for OpenCV** - The OpenCV consumer is a separate C++ process. It reads frames from iceoryx2 shared memory with true zero-copy access. This keeps the Rust codebase simple and lets you write OpenCV code idiomatically in C++.

4. **Single Rust binary** - Camera capture, IPC publishing, encoding, and WebRTC streaming all happen in one process. The only IPC is for the C++ consumer.

## Copy Count Analysis

| Path | Copies | Notes |
|------|--------|-------|
| Camera DMA → libcamera buffer | 0 | Hardware DMA |
| libcamera → iceoryx2 shared memory | 1 | One copy into shared memory |
| Shared memory → C++ OpenCV | 0 | Zero-copy pointer access |
| Shared memory → OpenH264 encoder | 0 | Same process, direct access (via channel for now) |
| OpenH264 → WebRTC RTP packets | 1 | Packetization |

## Project Structure

```
splitter/
├── Cargo.toml
├── src/
│   ├── lib.rs              # Library exports
│   ├── main.rs             # Main service binary
│   ├── camera/
│   │   └── capture.rs      # libcamera frame capture
│   ├── ipc/
│   │   ├── frame.rs        # SharedFrame type (must match C++ header)
│   │   └── publisher.rs    # iceoryx2 publisher
│   ├── encoder/
│   │   └── h264.rs         # OpenH264 encoder wrapper
│   └── webrtc/
│       ├── streamer.rs     # WebRTC peer connection
│       └── signaling.rs    # WebSocket signaling
├── examples/
│   └── cpp_subscriber/     # C++ OpenCV consumer example
│       ├── CMakeLists.txt
│       ├── main.cpp
│       └── shared_frame.hpp  # Must match Rust's SharedFrame!
└── web/
    └── viewer.html         # Browser WebRTC viewer
```

## Building

### Rust Service (on Pi)

```bash
# Install nasm for faster OpenH264
sudo apt install nasm

# Build
cargo build --release
```

### C++ Consumer (on Pi)

```bash
cd examples/cpp_subscriber

# Install dependencies
sudo apt install libopencv-dev

# Build iceoryx2 C++ bindings first (see iceoryx2 docs)
# Then:
mkdir build && cd build
cmake ..
make
```

## Running

### On the Raspberry Pi

```bash
# Terminal 1: Start the main service
./target/release/splitter --width 1280 --height 720 --fps 30

# Terminal 2 (optional): Start C++ OpenCV consumer
./examples/cpp_subscriber/build/opencv_consumer --best-effort
```

### On the Laptop

1. Open `web/viewer.html` in a browser
2. Enter the Pi's address: `ws://192.168.x.x:8080`
3. Click "Connect"

## Configuration

```
--width <WIDTH>       Frame width (default: 1280)
--height <HEIGHT>     Frame height (default: 720)
--framerate <FPS>     Frame rate (default: 30)
--bitrate <KBPS>      Encoder bitrate in kbps (default: 2000)
--port <PORT>         Signaling port (default: 8080)
```

## Performance Notes

### OpenH264 on Pi 5

The Pi 5's Cortex-A76 cores handle 720p30 encoding reasonably well. For 1080p, you may need to:
- Lower framerate to 15-20 fps
- Reduce bitrate
- Accept higher latency

Install `nasm` for SIMD-optimized encoding:
```bash
sudo apt install nasm
```

### iceoryx2 Shared Memory

The shared memory pool is configured with:
- Safe overflow (drops oldest frames when full)
- History size of 1 (late subscribers get latest frame)
- Buffer size of 4 frames

For best-effort consumers (like OpenCV), use `receive_latest()` to skip to the newest frame and avoid processing stale data.

## Keeping Rust/C++ Types in Sync

**IMPORTANT**: The `SharedFrame` struct must be identical in Rust and C++!

- Rust definition: `src/ipc/frame.rs`
- C++ definition: `examples/cpp_subscriber/shared_frame.hpp`

If you change one, update the other. The struct uses `#[repr(C)]` in Rust to ensure C-compatible layout.

## Troubleshooting

### Camera not found
```bash
# Check libcamera can see the camera
libcamera-hello --list-cameras
```

### OpenH264 slow
```bash
# Make sure nasm is installed for SIMD
which nasm
```

### C++ consumer can't connect
```bash
# Check iceoryx2 shared memory
ls /dev/shm/
# Should see iceoryx2 files after Rust service starts
```

### High WebRTC latency
- Lower resolution/framerate
- Check network bandwidth
- Try wired ethernet instead of WiFi

## License

MIT OR Apache-2.0
