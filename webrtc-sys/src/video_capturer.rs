use cxx::UniquePtr;

#[cxx::bridge(namespace = "livekit")]
mod ffi {

    #[derive(Clone)]
    struct VideoDevice {
        // Index of the device
        index: i32,
        // Name of the device
        name: String,
        // Unique identifier for the device
        uid: String,
        // Product ID of the device
        pid: String,
    }
    #[derive(Clone)]
    struct VideoCaptureCapability {
        width: i32,
        height: i32,
        maxFPS: i32,
        interlaced: bool,
    }

    unsafe extern "C++" {
        include!("livekit/video_capturer.h");
        type VideoCapturer;

        type NativeVideoSink = crate::video_track::ffi::NativeVideoSink;

        fn get_video_device_list() -> Vec<VideoDevice>;
        fn new_video_capturer() -> UniquePtr<VideoCapturer>;

        fn start_capture(self: &VideoCapturer, capability: VideoCaptureCapability) -> i32;
        fn stop_capture(self: &VideoCapturer) -> i32;
        fn register_capture_data_callback(self: &VideoCapturer, sink: &SharedPtr<NativeVideoSink>);
        fn deregister_capture_data_callback(self: &VideoCapturer);
    }
}
