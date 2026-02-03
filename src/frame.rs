use iceoryx2::prelude::ZeroCopySend;

/// Max frame size: 1080p YUV420 = 1920 * 1080 * 1.5 ≈ 3.1MB
pub const MAX_FRAME_SIZE: usize = 1920 * 1080 * 3 / 2;

/// Pixel format identifier (matches common fourcc codes)
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, ZeroCopySend)]
pub enum PixelFormat {
    Yuv420 = 0x32315559,  // YU12 / I420
    Nv12 = 0x3231564E,    // NV12
    Nv21 = 0x3132564E,    // NV21
    Unknown = 0,
}

#[repr(C)]
#[derive(Debug, ZeroCopySend)]
#[type_name("Frame")]
pub struct Frame {
    pub timestamp_ns: u64,
    pub sequence: u64,
    pub width: u32,
    pub height: u32,
    pub stride: u32,
    pub format: PixelFormat,
    pub len: u32,
    pub data: [u8; MAX_FRAME_SIZE],
}

// struct IpcFrame {
//     uint64_t timestamp_ns;
//     uint64_t sequence;
//     uint32_t width;
//     uint32_t height;
//     uint32_t stride;
//     uint32_t len;
//     uint8_t data[MAX_FRAME_SIZE];
// };

unsafe impl Send for Frame {}
unsafe impl Sync for Frame {}
