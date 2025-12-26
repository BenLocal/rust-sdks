use cxx::{SharedPtr, UniquePtr};

pub(crate) struct VideoCaptureCapability {
    width: i32,
    height: i32,
    max_fps: i32,
    interlaced: bool,
}

pub(crate) struct VideoCapturer {
    sys_handle: UniquePtr<webrtc_sys::video_capturer::ffi::VideoCapturer>,
}

pub(crate) struct VideoDevice {
    sys_handle: webrtc_sys::video_capturer::ffi::VideoDevice,
}

impl VideoCapturer {
    pub(crate) fn new() -> Option<Self> {
        let sys_handle = webrtc_sys::video_capturer::ffi::new_video_capturer();
        if sys_handle.is_null() {
            None
        } else {
            Some(Self { sys_handle })
        }
    }

    pub(crate) fn start(&self, capability: VideoCaptureCapability) {
        let capability = webrtc_sys::video_capturer::ffi::VideoCaptureCapability {
            width: capability.width,
            height: capability.height,
            maxFPS: capability.max_fps,
            interlaced: capability.interlaced,
        };
        self.sys_handle.start_capture(capability);
    }

    pub(crate) fn stop(&self) {
        self.sys_handle.stop_capture();
    }

    pub(crate) fn register_callback(
        &self,
        sink: SharedPtr<webrtc_sys::video_track::ffi::NativeVideoSink>,
    ) {
        self.sys_handle.register_capture_data_callback(&sink);
    }

    pub(crate) fn unregister_callback(&self) {
        self.sys_handle.deregister_capture_data_callback();
    }

    pub(crate) fn device_list() -> Vec<VideoDevice> {
        webrtc_sys::video_capturer::ffi::get_video_device_list()
            .into_iter()
            .map(|x| VideoDevice { sys_handle: x })
            .collect()
    }
}

impl VideoDevice {
    pub(crate) fn index(&self) -> i32 {
        self.sys_handle.index
    }

    pub(crate) fn name(&self) -> String {
        self.sys_handle.name.clone()
    }

    pub(crate) fn unique_id(&self) -> String {
        self.sys_handle.uid.clone()
    }

    pub(crate) fn product_id(&self) -> String {
        self.sys_handle.pid.clone()
    }
}
