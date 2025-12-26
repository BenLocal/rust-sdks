use crate::imp::video_capturer as vc_imp;

pub struct VideoCapturer {
    sys_handle: vc_imp::VideoCapturer,
}

pub struct VideoDevice {
    sys_handle: vc_imp::VideoDevice,
}

impl VideoCapturer {
    pub fn new() -> Option<Self> {
        vc_imp::VideoCapturer::new().map(|i| Self { sys_handle: i })
    }

    pub fn device_list() -> Vec<VideoDevice> {
        vc_imp::VideoCapturer::device_list()
            .into_iter()
            .map(|x| VideoDevice { sys_handle: x })
            .collect()
    }
}

impl VideoDevice {
    pub fn index(&self) -> i32 {
        self.sys_handle.index()
    }

    pub fn name(&self) -> String {
        self.sys_handle.name()
    }

    pub fn unique_id(&self) -> String {
        self.sys_handle.unique_id()
    }

    pub fn product_id(&self) -> String {
        self.sys_handle.product_id()
    }
}
