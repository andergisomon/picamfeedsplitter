mod frame;

use std::sync::mpsc;

use frame::{Frame, MAX_FRAME_SIZE};
use iceoryx2::prelude::*;
use libcamera::{
    camera::CameraConfigurationStatus,
    camera_manager::CameraManager,
    framebuffer::AsFrameBuffer,
    framebuffer_allocator::{FrameBuffer, FrameBufferAllocator},
    framebuffer_map::MemoryMappedFrameBuffer,
    geometry::Size,
    request::ReuseFlag,
    stream::StreamRole,
};
use thiserror::Error;
use tracing::{debug, error, info, warn};

const SERVICE_NAME: &str = "camera/frames";

#[derive(Error, Debug)]
enum Error {
    #[error("No cameras found")]
    NoCamera,
    #[error("libcamera error: {0}")]
    Camera(String),
    #[error("iceoryx2 error: {0}")]
    Ipc(String)
}

fn main() -> Result<(), Error> {
    tracing_subscriber::fmt()
        .with_env_filter("info,splitter=debug")
        .init();

    // Parse args
    let mut width: u32 = 1280;
    let mut height: u32 = 720;
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--width" => width = args.next().and_then(|s| s.parse().ok()).unwrap_or(width),
            "--height" => height = args.next().and_then(|s| s.parse().ok()).unwrap_or(height),
            _ => {}
        }
    }

    info!(width, height, "Starting camera publisher");

    // Set up iceoryx2 publisher
    let node = NodeBuilder::new()
        .create::<ipc::Service>()
        .map_err(|e| Error::Ipc(format!("{e:?}")))?;

    let service = node
        .service_builder(&SERVICE_NAME.try_into().unwrap())
        .publish_subscribe::<Frame>()
        .open_or_create()
        .map_err(|e| Error::Ipc(format!("{e:?}")))?;

    let publisher = service
        .publisher_builder()
        .create()
        .map_err(|e| Error::Ipc(format!("{e:?}")))?;

    info!("IPC publisher ready");

    // Set up camera
    let mgr = CameraManager::new().map_err(|e| Error::Camera(format!("{e:?}")))?;
    let cameras = mgr.cameras();
    let cam = cameras.get(0).ok_or(Error::NoCamera)?;

    info!(id = %cam.id(), "Found camera");

    let mut cam = cam.acquire().map_err(|e| Error::Camera(format!("{e:?}")))?;
    let mut config = cam
        .generate_configuration(&[StreamRole::VideoRecording])
        .ok_or_else(|| Error::Camera("Failed to generate config".into()))?;

    config.get_mut(0).unwrap().set_size(Size::new(width, height));

    match config.validate() {
        CameraConfigurationStatus::Valid => info!("Config valid"),
        CameraConfigurationStatus::Adjusted => warn!("Config adjusted"),
        CameraConfigurationStatus::Invalid => return Err(Error::Camera("Invalid config".into())),
    }

    cam.configure(&mut config)
        .map_err(|e| Error::Camera(format!("{e:?}")))?;

    let stream_config = config.get(0).unwrap();
    let stream = stream_config.stream().unwrap();
    let actual_width = stream_config.get_size().width;
    let actual_height = stream_config.get_size().height;
    let stride = stream_config.get_stride() as u32;

    info!(actual_width, actual_height, stride, "Camera configured");

    let mut alloc = FrameBufferAllocator::new(&cam);
    let buffers: Vec<_> = alloc
        .alloc(&stream)
        .map_err(|e| Error::Camera(format!("{e:?}")))?
        .into_iter()
        .map(|b| MemoryMappedFrameBuffer::new(b).unwrap())
        .collect();

    info!(count = buffers.len(), "Allocated buffers");


    // move frame buffer into a camera capture request
    let requests: Vec<_> = buffers
        .into_iter()
        .map(|buf| {
            let mut req = cam
                .create_request(None)
                .ok_or_else(|| Error::Camera("Failed to create request".into()))?;
            req.add_buffer(&stream, buf)
                .map_err(|e| Error::Camera(format!("{e:?}")))?;
            Ok(req)
        })
        .collect::<Result<Vec<_>, Error>>()?;

    // specify callback once camera capture request is completed
    // the callback just sends the next camera capture request
    let (tx, rx) = mpsc::channel();
    cam.on_request_completed(move |req| {
        tx.send(req).unwrap();
    });

    cam.start(None)
        .map_err(|e| Error::Camera(format!("{e:?}")))?;

    for req in requests {
        cam.queue_request(req)
            .map_err(|(_, e)| Error::Camera(format!("{e:?}")))?;
    }

    info!("Capture loop starting");

    let mut seq: u64 = 0;

    loop {
        // block on receive camera capture request
        let mut req = rx.recv().map_err(|e| Error::Camera(format!("{e:?}")))?;

        let fb: &MemoryMappedFrameBuffer<FrameBuffer> = match req.buffer(&stream) {
            Some(b) => b,
            None => {
                warn!("No buffer in request");
                continue;
            }
        };

        let metadata = match fb.metadata() {
            Some(m) => m,
            None => {
                warn!("metadata ist none");
                continue;
            }
        };

        let ts = metadata.timestamp();
        let planes = fb.data();

        if let Some(plane_data) = planes.first() {
            let bytes_used = metadata
                .planes()
                .get(0)
                .map(|p| p.bytes_used as usize)
                .unwrap_or(plane_data.len());

            let data = &plane_data[..bytes_used];

            if data.len() > MAX_FRAME_SIZE {
                error!(len = data.len(), "Frame too large, skipping");
            } else {
                match publisher.loan_uninit() {
                    Ok(sample) => {
                        let sample = sample.write_payload(Frame {
                            timestamp_ns: ts,
                            sequence: seq,
                            width: actual_width,
                            height: actual_height,
                            stride,
                            len: data.len() as u32,
                            data: {
                                let mut arr = [0u8; MAX_FRAME_SIZE];
                                arr[..data.len()].copy_from_slice(data);
                                arr
                            },
                        });
                        let _ = sample.send();
                        debug!(seq, len = data.len(), "Published");
                    }
                    Err(e) => {
                        warn!("Loan failed: {e:?}");
                    }
                }
            }

            seq += 1;
            if seq % 100 == 0 {
                info!(seq, "Progress");
            }
        }

        req.reuse(ReuseFlag::REUSE_BUFFERS);
        cam.queue_request(req)
            .map_err(|(_, e)| Error::Camera(format!("{e:?}")))?;
    }
}
