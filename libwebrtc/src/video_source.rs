// Copyright 2025 LiveKit, Inc.
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use livekit_protocol::enum_dispatch;

use crate::imp::video_source as vs_imp;

#[derive(Debug, Clone)]
pub struct VideoResolution {
    pub width: u32,
    pub height: u32,
}

impl Default for VideoResolution {
    // Default to 720p
    fn default() -> Self {
        VideoResolution { width: 1280, height: 720 }
    }
}

#[non_exhaustive]
#[derive(Debug, Clone)]
pub enum RtcVideoSource {
    // TODO(theomonnom): Web video sources (eq. to tracks on browsers?)
    #[cfg(not(target_arch = "wasm32"))]
    Native(native::NativeVideoSource),
    #[cfg(not(target_arch = "wasm32"))]
    Encoded(encoded::EncodedVideoSource),
}

impl RtcVideoSource {
    #[cfg(not(target_arch = "wasm32"))]
    enum_dispatch!(
        [Native, Encoded];
        pub fn video_resolution(self: &Self) -> VideoResolution;
    );
}

#[cfg(not(target_arch = "wasm32"))]
pub mod native {
    use std::fmt::{Debug, Formatter};

    use super::*;
    use crate::video_frame::{VideoBuffer, VideoFrame};

    #[derive(Clone)]
    pub struct NativeVideoSource {
        pub(crate) handle: vs_imp::NativeVideoSource,
    }

    impl Debug for NativeVideoSource {
        fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
            f.debug_struct("NativeVideoSource").finish()
        }
    }

    impl Default for NativeVideoSource {
        fn default() -> Self {
            Self::new(VideoResolution::default())
        }
    }

    impl NativeVideoSource {
        pub fn new(resolution: VideoResolution) -> Self {
            Self { handle: vs_imp::NativeVideoSource::new(resolution) }
        }

        pub fn capture_frame<T: AsRef<dyn VideoBuffer>>(&self, frame: &VideoFrame<T>) {
            self.handle.capture_frame(frame)
        }

        pub fn video_resolution(&self) -> VideoResolution {
            self.handle.video_resolution()
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub mod encoded {
    use std::fmt::{Debug, Formatter};
    use std::sync::{Arc, Mutex};
    use tokio::sync::mpsc;

    use super::VideoResolution;
    use crate::native::passthrough_video_source::{
        EncodedVideoFrame as PassthroughEncodedFrame, PassthroughEncoderFactory,
        PassthroughEncoderHandle,
    };

    /// Video codec type
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum VideoCodecType {
        H264,
        VP8,
        VP9,
        AV1,
    }

    /// Codec parameters
    #[derive(Debug, Clone)]
    pub struct CodecParameters {
        pub codec: VideoCodecType,
        pub profile: Option<String>,
        pub level: Option<String>,
    }

    impl Default for CodecParameters {
        fn default() -> Self {
            Self { codec: VideoCodecType::H264, profile: None, level: None }
        }
    }

    /// Encoded video frame
    #[derive(Debug, Clone)]
    pub struct EncodedVideoFrame {
        pub data: Vec<u8>,
        pub rtp_timestamp: u32,
        pub capture_time_ms: i64,
        pub ntp_time_ms: i64,
        pub is_keyframe: bool,
        pub width: u32,
        pub height: u32,
        pub codec: VideoCodecType,
    }

    impl EncodedVideoFrame {
        pub fn new(
            data: Vec<u8>,
            rtp_timestamp: u32,
            capture_time_ms: i64,
            is_keyframe: bool,
            width: u32,
            height: u32,
            codec: VideoCodecType,
        ) -> Self {
            Self {
                data,
                rtp_timestamp,
                capture_time_ms,
                ntp_time_ms: capture_time_ms,
                is_keyframe,
                width,
                height,
                codec,
            }
        }

        pub fn keyframe(
            data: Vec<u8>,
            rtp_timestamp: u32,
            capture_time_ms: i64,
            width: u32,
            height: u32,
            codec: VideoCodecType,
        ) -> Self {
            Self::new(data, rtp_timestamp, capture_time_ms, true, width, height, codec)
        }

        pub fn delta_frame(
            data: Vec<u8>,
            rtp_timestamp: u32,
            capture_time_ms: i64,
            width: u32,
            height: u32,
            codec: VideoCodecType,
        ) -> Self {
            Self::new(data, rtp_timestamp, capture_time_ms, false, width, height, codec)
        }
    }

    /// Encoded video source for injecting pre-encoded frames
    #[derive(Clone)]
    pub struct EncodedVideoSource {
        width: u32,
        height: u32,
        codec: VideoCodecType,
        encoder_factory: Arc<PassthroughEncoderFactory>,
        encoder_handle: Arc<Mutex<Option<PassthroughEncoderHandle>>>,
        frame_tx: Arc<Mutex<Option<mpsc::Sender<EncodedVideoFrame>>>>,
    }

    impl Debug for EncodedVideoSource {
        fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
            f.debug_struct("EncodedVideoSource")
                .field("width", &self.width)
                .field("height", &self.height)
                .field("codec", &self.codec)
                .finish()
        }
    }

    impl EncodedVideoSource {
        /// Create a new encoded video source
        pub fn new(codec: VideoCodecType, width: u32, height: u32) -> Option<Self> {
            // Only H.264 is currently supported for passthrough
            if codec != VideoCodecType::H264 {
                return None;
            }

            Some(Self {
                width,
                height,
                codec,
                encoder_factory: Arc::new(PassthroughEncoderFactory::new()),
                encoder_handle: Arc::new(Mutex::new(None)),
                frame_tx: Arc::new(Mutex::new(None)),
            })
        }

        /// Get the width
        pub fn width(&self) -> u32 {
            self.width
        }

        /// Get the height
        pub fn height(&self) -> u32 {
            self.height
        }

        /// Get the codec type
        pub fn codec(&self) -> VideoCodecType {
            self.codec
        }

        /// Get the video resolution
        pub fn video_resolution(&self) -> VideoResolution {
            VideoResolution { width: self.width, height: self.height }
        }

        /// Initialize the encoder (call this after the track is published)
        /// Returns true if encoder was successfully initialized
        pub fn initialize_encoder(&self) -> bool {
            // Poll for encoder availability
            for _ in 0..50 {
                // Try for up to 5 seconds (50 * 100ms)
                if let Some(encoder) = self.encoder_factory.get_encoder() {
                    *self.encoder_handle.lock().unwrap() = Some(encoder);
                    return true;
                }
                // Use std::thread::sleep since this is a blocking function
                std::thread::sleep(std::time::Duration::from_millis(100));
            }
            false
        }

        /// Async version of initialize_encoder (preferred in async contexts)
        pub async fn initialize_encoder_async(&self) -> bool {
            use tokio::time::{sleep, Duration};
            // Poll for encoder availability
            for _ in 0..50 {
                // Try for up to 5 seconds (50 * 100ms)
                if let Some(encoder) = self.encoder_factory.get_encoder() {
                    *self.encoder_handle.lock().unwrap() = Some(encoder);
                    return true;
                }
                sleep(Duration::from_millis(100)).await;
            }
            false
        }

        /// Push an encoded frame to the source
        pub fn push_frame(&self, frame: &EncodedVideoFrame) -> Result<(), String> {
            let encoder_handle = self.encoder_handle.lock().unwrap();
            if let Some(ref encoder) = *encoder_handle {
                // Convert to passthrough encoded frame
                let passthrough_frame = PassthroughEncodedFrame::new(
                    frame.data.clone(),
                    frame.rtp_timestamp,
                    frame.capture_time_ms,
                    frame.is_keyframe,
                    frame.width,
                    frame.height,
                );

                encoder.inject_frame(&passthrough_frame).map_err(|e| format!("{}", e))
            } else {
                Err("Encoder not initialized".to_string())
            }
        }

        /// Check if keyframe is requested
        pub fn is_keyframe_requested(&self) -> bool {
            let encoder_handle = self.encoder_handle.lock().unwrap();
            if let Some(ref encoder) = *encoder_handle {
                encoder.is_keyframe_requested()
            } else {
                false
            }
        }

        /// Clear keyframe request
        pub fn clear_keyframe_request(&self) {
            let encoder_handle = self.encoder_handle.lock().unwrap();
            if let Some(ref encoder) = *encoder_handle {
                encoder.clear_keyframe_request();
            }
        }

        /// Request a keyframe
        pub fn request_keyframe(&self) {
            let encoder_handle = self.encoder_handle.lock().unwrap();
            if let Some(ref encoder) = *encoder_handle {
                encoder.request_keyframe();
            }
        }
    }
}

#[cfg(target_arch = "wasm32")]
pub mod web {}
