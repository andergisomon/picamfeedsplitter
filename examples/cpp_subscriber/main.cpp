// C++ iceoryx2 subscriber example
//
// NOTE: iceoryx2 C bindings require building iceoryx2-ffi-c from the Rust crate.
// See: https://github.com/eclipse-iceoryx/iceoryx2
//
// Build the C bindings:
//   cd iceoryx2 && cargo build --release -p iceoryx2-ffi-c
//
// This example shows the frame layout and how you'd integrate with OpenCV.

#include <cstdint>
#include <cstdio>
#include <cstring>

// Frame layout must match Rust's #[repr(C)] struct exactly
constexpr size_t MAX_FRAME_SIZE = 1920 * 1080 * 3 / 2;  // ~3.1MB for 1080p YUV420

struct Frame {
    uint64_t timestamp_ns;
    uint64_t sequence;
    uint32_t width;
    uint32_t height;
    uint32_t stride;
    uint32_t len;
    uint8_t data[MAX_FRAME_SIZE];
};

static_assert(sizeof(Frame) == 8 + 8 + 4 + 4 + 4 + 4 + MAX_FRAME_SIZE, "Frame layout mismatch");

#ifdef WITH_OPENCV
#include <opencv2/opencv.hpp>

void process_frame(const Frame& frame) {
    // Create cv::Mat from YUV420 data (no copy, just wraps the pointer)
    cv::Mat yuv(frame.height + frame.height / 2, frame.width, CV_8UC1,
                const_cast<uint8_t*>(frame.data));

    // Convert to BGR for processing
    cv::Mat bgr;
    cv::cvtColor(yuv, bgr, cv::COLOR_YUV2BGR_I420);

    // Your processing here...
    printf("Frame %llu: %ux%u, processing with OpenCV\n",
           (unsigned long long)frame.sequence, frame.width, frame.height);

    // Example: display
    // cv::imshow("Camera", bgr);
    // cv::waitKey(1);
}
#else
void process_frame(const Frame& frame) {
    printf("Frame %llu: %ux%u, %u bytes (OpenCV not enabled)\n",
           (unsigned long long)frame.sequence, frame.width, frame.height, frame.len);
}
#endif

// Placeholder for iceoryx2 subscriber integration
// Replace with actual iceoryx2-ffi-c calls
int main() {
    printf("C++ iceoryx2 subscriber\n");
    printf("Frame struct size: %zu bytes\n", sizeof(Frame));

    // TODO: Initialize iceoryx2 node
    // iox2_node_builder_t* builder = iox2_node_builder_new();
    // iox2_node_t* node = ...;

    // TODO: Open service and create subscriber
    // const char* service_name = "camera/frames";
    // iox2_service_t* service = ...;
    // iox2_subscriber_t* subscriber = ...;

    // TODO: Receive loop
    // while (true) {
    //     iox2_sample_t* sample = ...;
    //     if (sample) {
    //         const Frame* frame = (const Frame*)iox2_sample_payload(sample);
    //         process_frame(*frame);
    //         iox2_sample_release(sample);
    //     }
    // }

    printf("See iceoryx2-ffi-c documentation for actual implementation\n");
    return 0;
}
