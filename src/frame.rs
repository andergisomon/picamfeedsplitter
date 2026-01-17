use iceoryx2::prelude::ZeroCopySend;

/// Max frame size: 1080p YUV420 = 1920 * 1080 * 1.5 ≈ 3.1MB
pub const MAX_FRAME_SIZE: usize = 1920 * 1080 * 3 / 2;

#[repr(C)]
#[derive(Debug, ZeroCopySend)]
pub struct Frame {
    pub timestamp_ns: u64,
    pub sequence: u64,
    pub width: u32,
    pub height: u32,
    pub stride: u32,
    pub len: u32,
    pub data: [u8; MAX_FRAME_SIZE],
}

unsafe impl Send for Frame {}
unsafe impl Sync for Frame {}
