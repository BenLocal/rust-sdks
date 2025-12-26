use std::{
    pin::Pin,
    sync::Arc,
    task::{Context, Poll},
};

use cxx::{SharedPtr, UniquePtr};
use livekit_runtime::Stream;
use tokio::sync::mpsc;
use webrtc_sys::video_track as sys_vt;

use super::video_frame::new_video_frame_buffer;
use crate::video_frame::{BoxVideoFrame, VideoFrame};

#[derive(Default)]
pub(crate) struct VideoCaptureCapability {
    width: i32,
    height: i32,
    max_fps: i32,
    interlaced: bool,
}

impl VideoCaptureCapability {
    pub(crate) fn set_width(mut self, width: i32) -> Self {
        self.width = width;
        self
    }

    pub(crate) fn set_height(mut self, height: i32) -> Self {
        self.height = height;
        self
    }

    pub(crate) fn set_max_fps(mut self, max_fps: i32) -> Self {
        self.max_fps = max_fps;
        self
    }

    pub(crate) fn set_interlaced(mut self, interlaced: bool) -> Self {
        self.interlaced = interlaced;
        self
    }
}

pub(crate) struct VideoCapturer {
    sys_handle: UniquePtr<webrtc_sys::video_capturer::ffi::VideoCapturer>,
}

pub(crate) struct VideoDevice {
    sys_handle: webrtc_sys::video_capturer::ffi::VideoDevice,
}

impl VideoCapturer {
    pub(crate) fn new(unique_id: &str) -> Option<Self> {
        let sys_handle = webrtc_sys::video_capturer::ffi::new_video_capturer(unique_id);
        if sys_handle.is_null() {
            None
        } else {
            Some(Self { sys_handle })
        }
    }

    pub(crate) fn register_callback(&self) -> NativeVideoCapturerStream {
        let (frame_tx, frame_rx) = mpsc::unbounded_channel();
        let observer = Arc::new(VideoCapturerTrackObserver { frame_tx });
        let native_sink = sys_vt::ffi::new_native_video_sink(Box::new(
            sys_vt::VideoSinkWrapper::new(observer.clone()),
        ));
        self.sys_handle.register_capture_data_callback(&native_sink);

        NativeVideoCapturerStream { _native_sink: native_sink, frame_rx }
    }

    pub(crate) fn start(&self, capability: VideoCaptureCapability) -> i32 {
        let capability = webrtc_sys::video_capturer::ffi::VideoCaptureCapability {
            width: capability.width,
            height: capability.height,
            maxFPS: capability.max_fps,
            interlaced: capability.interlaced,
        };
        self.sys_handle.start_capture(capability)
    }

    pub(crate) fn unregister_callback(&self) {
        self.sys_handle.deregister_capture_data_callback();
    }

    pub(crate) fn stop(&self) -> i32 {
        self.sys_handle.stop_capture()
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

pub struct NativeVideoCapturerStream {
    _native_sink: SharedPtr<sys_vt::ffi::NativeVideoSink>,
    frame_rx: mpsc::UnboundedReceiver<BoxVideoFrame>,
}

impl NativeVideoCapturerStream {
    fn close(&mut self) {
        self.frame_rx.close();
    }
}

impl Drop for NativeVideoCapturerStream {
    fn drop(&mut self) {
        self.close();
    }
}

impl Stream for NativeVideoCapturerStream {
    type Item = BoxVideoFrame;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context) -> Poll<Option<Self::Item>> {
        self.frame_rx.poll_recv(cx)
    }
}

struct VideoCapturerTrackObserver {
    frame_tx: mpsc::UnboundedSender<BoxVideoFrame>,
}

impl sys_vt::VideoSink for VideoCapturerTrackObserver {
    fn on_frame(&self, frame: UniquePtr<webrtc_sys::video_frame::ffi::VideoFrame>) {
        let _ = self.frame_tx.send(VideoFrame {
            rotation: frame.rotation().into(),
            timestamp_us: frame.timestamp_us(),
            buffer: new_video_frame_buffer(unsafe { frame.video_frame_buffer() }),
        });
    }

    fn on_discarded_frame(&self) {}

    fn on_constraints_changed(&self, _constraints: sys_vt::ffi::VideoTrackSourceConstraints) {}
}
