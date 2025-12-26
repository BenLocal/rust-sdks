use livekit_runtime::Stream;

use crate::{imp::video_capturer as vc_imp, prelude::BoxVideoFrame};
use std::{
    pin::Pin,
    sync::Arc,
    task::{Context, Poll},
};

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

    pub fn start(&self) {
        self.sys_handle.start(vc_imp::VideoCaptureCapability::default());
    }

    pub fn stop(&self) {
        self.sys_handle.stop();
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
