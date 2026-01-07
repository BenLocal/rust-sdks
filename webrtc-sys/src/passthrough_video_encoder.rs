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

use crate::impl_thread_safety;

#[cxx::bridge(namespace = "livekit")]
pub mod ffi {
    unsafe extern "C++" {
        include!("livekit/passthrough_video_encoder.h");

        type PassthroughVideoEncoder;
        type PassthroughVideoEncoderFactory;

        /// Create a new passthrough video encoder factory
        fn new_passthrough_video_encoder_factory() -> SharedPtr<PassthroughVideoEncoderFactory>;

        /// Inject an encoded H.264 frame into the encoder
        /// Returns 0 on success, non-zero error code on failure
        unsafe fn passthrough_encoder_inject_frame(
            encoder: *mut PassthroughVideoEncoder,
            data: &[u8],
            rtp_timestamp: u32,
            capture_time_ms: i64,
            ntp_time_ms: i64,
            is_keyframe: bool,
            width: u32,
            height: u32,
        ) -> i32;

        /// Check if a keyframe has been requested by the receiver
        unsafe fn passthrough_encoder_is_keyframe_requested(
            encoder: *const PassthroughVideoEncoder,
        ) -> bool;

        /// Clear the keyframe request flag
        unsafe fn passthrough_encoder_clear_keyframe_request(encoder: *mut PassthroughVideoEncoder);

        /// Request a keyframe (can be called when we know we need one)
        unsafe fn passthrough_encoder_request_keyframe(encoder: *mut PassthroughVideoEncoder);

        /// Get the last created encoder from the factory
        /// Note: The returned pointer is only valid until the next Create() call
        /// or until the encoder is destroyed
        unsafe fn passthrough_factory_get_encoder(
            factory: *const PassthroughVideoEncoderFactory,
        ) -> *mut PassthroughVideoEncoder;

        /// Dummy function for shared_ptr instantiation
        fn _shared_passthrough_video_encoder_factory() -> SharedPtr<PassthroughVideoEncoderFactory>;
    }
}

impl_thread_safety!(ffi::PassthroughVideoEncoder, Send + Sync);
impl_thread_safety!(ffi::PassthroughVideoEncoderFactory, Send + Sync);
