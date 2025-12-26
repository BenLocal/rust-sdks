use livekit_runtime::Stream;

use crate::{imp::video_capturer as vc_imp, prelude::BoxVideoFrame};
use std::{
    pin::Pin,
    sync::Arc,
    task::{Context, Poll},
};

pub struct VideoCaptureCapability {
    width: i32,
    height: i32,
    max_fps: i32,
    interlaced: bool,
}

impl VideoCaptureCapability {
    pub fn new(width: i32, height: i32, max_fps: i32, interlaced: bool) -> Self {
        Self { width, height, max_fps, interlaced }
    }
}

impl Default for VideoCaptureCapability {
    fn default() -> Self {
        Self { width: 640, height: 480, max_fps: 30, interlaced: false }
    }
}

impl Into<vc_imp::VideoCaptureCapability> for VideoCaptureCapability {
    fn into(self) -> vc_imp::VideoCaptureCapability {
        vc_imp::VideoCaptureCapability::default()
            .set_width(self.width)
            .set_height(self.height)
            .set_max_fps(self.max_fps)
            .set_interlaced(self.interlaced)
    }
}

pub struct VideoCapturer {
    sys_handle: vc_imp::VideoCapturer,
}

pub struct VideoDevice {
    sys_handle: vc_imp::VideoDevice,
}

impl VideoCapturer {
    pub fn open_device(unique_id: &str) -> Option<(Self, NativeVideoCapturerStream)> {
        let m = vc_imp::VideoCapturer::new(unique_id).map(|i| Self { sys_handle: i })?;
        let stream = m.sys_handle.register_callback();
        Some((m, NativeVideoCapturerStream(stream)))
    }

    pub fn start(&self, capability: VideoCaptureCapability) -> bool {
        self.sys_handle.start(capability.into()) == 0
    }

    pub fn stop(&self) -> bool {
        self.sys_handle.stop() == 0
    }

    #[allow(dead_code)]
    pub fn unregister_callback(&self) {
        self.sys_handle.unregister_callback();
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

pub struct NativeVideoCapturerStream(vc_imp::NativeVideoCapturerStream);

impl Stream for NativeVideoCapturerStream {
    type Item = BoxVideoFrame;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Option<Self::Item>> {
        Pin::new(&mut self.get_mut().0).poll_next(cx)
    }
}
