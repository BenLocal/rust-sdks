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

    unsafe extern "C++" {
        include!("livekit/video_capturer.h");

        fn get_video_device_list() -> Vec<VideoDevice>;
    }
}
