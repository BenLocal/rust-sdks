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

//! Passthrough video source for injecting pre-encoded H.264 frames.
//!
//! This module provides a way to send already-encoded H.264 frames through
//! WebRTC without re-encoding. This is useful when you have access to a
//! hardware encoder or pre-encoded video stream.

use cxx::SharedPtr;
use webrtc_sys::passthrough_video_encoder::ffi::{
    self as pt_ffi, PassthroughVideoEncoder, PassthroughVideoEncoderFactory,
};

/// Represents a pre-encoded H.264 frame ready for injection.
#[derive(Debug, Clone)]
pub struct EncodedVideoFrame {
    /// The encoded H.264 data (should include NAL units)
    pub data: Vec<u8>,
    /// RTP timestamp (90kHz clock)
    pub rtp_timestamp: u32,
    /// Capture time in milliseconds
    pub capture_time_ms: i64,
    /// NTP time in milliseconds
    pub ntp_time_ms: i64,
    /// Whether this frame is a keyframe (IDR frame)
    pub is_keyframe: bool,
    /// Frame width in pixels
    pub width: u32,
    /// Frame height in pixels
    pub height: u32,
}

impl EncodedVideoFrame {
    /// Create a new encoded video frame.
    pub fn new(
        data: Vec<u8>,
        rtp_timestamp: u32,
        capture_time_ms: i64,
        is_keyframe: bool,
        width: u32,
        height: u32,
    ) -> Self {
        Self {
            data,
            rtp_timestamp,
            capture_time_ms,
            ntp_time_ms: capture_time_ms, // Default NTP time to capture time
            is_keyframe,
            width,
            height,
        }
    }

    /// Create a keyframe.
    pub fn keyframe(
        data: Vec<u8>,
        rtp_timestamp: u32,
        capture_time_ms: i64,
        width: u32,
        height: u32,
    ) -> Self {
        Self::new(data, rtp_timestamp, capture_time_ms, true, width, height)
    }

    /// Create a delta frame (non-keyframe).
    pub fn delta_frame(
        data: Vec<u8>,
        rtp_timestamp: u32,
        capture_time_ms: i64,
        width: u32,
        height: u32,
    ) -> Self {
        Self::new(data, rtp_timestamp, capture_time_ms, false, width, height)
    }
}

/// Error type for passthrough video operations.
#[derive(Debug, Clone)]
pub enum PassthroughError {
    /// The encoder is not initialized
    NotInitialized,
    /// Invalid frame data
    InvalidData,
    /// Encoder error with error code
    EncoderError(i32),
    /// No encoder available
    NoEncoder,
}

impl std::fmt::Display for PassthroughError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PassthroughError::NotInitialized => write!(f, "Passthrough encoder not initialized"),
            PassthroughError::InvalidData => write!(f, "Invalid frame data"),
            PassthroughError::EncoderError(code) => write!(f, "Encoder error: {}", code),
            PassthroughError::NoEncoder => write!(f, "No encoder available"),
        }
    }
}

impl std::error::Error for PassthroughError {}

/// A handle to a passthrough video encoder factory.
///
/// This factory creates passthrough encoders that accept pre-encoded H.264 frames.
/// Use this to get access to the encoder after it's been created by WebRTC.
#[derive(Clone)]
pub struct PassthroughEncoderFactory {
    inner: SharedPtr<PassthroughVideoEncoderFactory>,
}

impl PassthroughEncoderFactory {
    /// Create a new passthrough encoder factory.
    pub fn new() -> Self {
        Self { inner: pt_ffi::new_passthrough_video_encoder_factory() }
    }

    /// Get the last encoder created by this factory.
    ///
    /// Returns None if no encoder has been created yet.
    /// Note: The encoder is created by WebRTC during SDP negotiation when
    /// a video track with H.264 codec is added.
    pub fn get_encoder(&self) -> Option<PassthroughEncoderHandle> {
        let encoder_ptr =
            unsafe { pt_ffi::passthrough_factory_get_encoder(self.inner.as_ref().unwrap()) };
        if encoder_ptr.is_null() {
            None
        } else {
            Some(PassthroughEncoderHandle { encoder_ptr })
        }
    }

    /// Get the underlying shared pointer for integration with webrtc-sys.
    pub(crate) fn sys_handle(&self) -> &SharedPtr<PassthroughVideoEncoderFactory> {
        &self.inner
    }
}

impl Default for PassthroughEncoderFactory {
    fn default() -> Self {
        Self::new()
    }
}

// Safety: The PassthroughVideoEncoderFactory is thread-safe
unsafe impl Send for PassthroughEncoderFactory {}
unsafe impl Sync for PassthroughEncoderFactory {}

/// A handle to a passthrough video encoder.
///
/// This allows injecting pre-encoded H.264 frames directly into the WebRTC pipeline.
pub struct PassthroughEncoderHandle {
    encoder_ptr: *mut PassthroughVideoEncoder,
}

impl PassthroughEncoderHandle {
    /// Inject an encoded H.264 frame.
    ///
    /// The frame data should contain valid H.264 NAL units.
    /// Returns Ok(()) on success, or an error if injection failed.
    pub fn inject_frame(&self, frame: &EncodedVideoFrame) -> Result<(), PassthroughError> {
        if frame.data.is_empty() {
            return Err(PassthroughError::InvalidData);
        }

        let result = unsafe {
            pt_ffi::passthrough_encoder_inject_frame(
                self.encoder_ptr,
                &frame.data,
                frame.rtp_timestamp,
                frame.capture_time_ms,
                frame.ntp_time_ms,
                frame.is_keyframe,
                frame.width,
                frame.height,
            )
        };

        if result == 0 {
            Ok(())
        } else {
            Err(PassthroughError::EncoderError(result))
        }
    }

    /// Check if a keyframe has been requested by the receiver.
    ///
    /// When this returns true, you should send a keyframe as soon as possible.
    pub fn is_keyframe_requested(&self) -> bool {
        unsafe { pt_ffi::passthrough_encoder_is_keyframe_requested(self.encoder_ptr) }
    }

    /// Clear the keyframe request flag.
    ///
    /// Call this after sending a keyframe.
    pub fn clear_keyframe_request(&self) {
        unsafe { pt_ffi::passthrough_encoder_clear_keyframe_request(self.encoder_ptr) };
    }

    /// Request a keyframe.
    ///
    /// This can be called when you know a keyframe is needed (e.g., new viewer joined).
    pub fn request_keyframe(&self) {
        unsafe { pt_ffi::passthrough_encoder_request_keyframe(self.encoder_ptr) };
    }
}

// Safety: The encoder pointer is thread-safe as it uses internal locking
unsafe impl Send for PassthroughEncoderHandle {}
unsafe impl Sync for PassthroughEncoderHandle {}
