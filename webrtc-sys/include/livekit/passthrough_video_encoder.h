/*
 * Copyright 2025 LiveKit, Inc.
 *
 * Licensed under the Apache License, Version 2.0 (the "License");
 * you may not use this file except in compliance with the License.
 * You may obtain a copy of the License at
 *
 *     http://www.apache.org/licenses/LICENSE-2.0
 *
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific language governing permissions and
 * limitations under the License.
 */

#ifndef LIVEKIT_PASSTHROUGH_VIDEO_ENCODER_H_
#define LIVEKIT_PASSTHROUGH_VIDEO_ENCODER_H_

#include <atomic>
#include <memory>
#include <vector>

#include "api/video/encoded_image.h"
#include "api/video_codecs/sdp_video_format.h"
#include "api/video_codecs/video_encoder.h"
#include "api/video_codecs/video_encoder_factory.h"
#include "modules/video_coding/include/video_codec_interface.h"
#include "rtc_base/synchronization/mutex.h"
#include "rust/cxx.h"

namespace livekit {
class PassthroughVideoEncoder;
class PassthroughVideoEncoderFactory;
}  // namespace livekit
#include "webrtc-sys/src/passthrough_video_encoder.rs.h"

namespace livekit {

// Passthrough encoder - receives already encoded frames and passes them
// directly to the RTP layer without re-encoding.
class PassthroughVideoEncoder : public webrtc::VideoEncoder {
 public:
  explicit PassthroughVideoEncoder(const webrtc::SdpVideoFormat& format);
  ~PassthroughVideoEncoder() override;

  // VideoEncoder interface implementation
  int32_t InitEncode(const webrtc::VideoCodec* codec_settings,
                     const webrtc::VideoEncoder::Settings& settings) override;

  int32_t RegisterEncodeCompleteCallback(
      webrtc::EncodedImageCallback* callback) override;

  int32_t Release() override;

  // This method will receive dummy frames - we ignore them since we inject
  // pre-encoded frames directly
  int32_t Encode(
      const webrtc::VideoFrame& frame,
      const std::vector<webrtc::VideoFrameType>* frame_types) override;

  void SetRates(const RateControlParameters& parameters) override;

  EncoderInfo GetEncoderInfo() const override;

  // Custom method: inject an already-encoded frame
  // Returns WEBRTC_VIDEO_CODEC_OK on success
  int32_t InjectEncodedFrame(const uint8_t* data,
                             size_t size,
                             uint32_t rtp_timestamp,
                             int64_t capture_time_ms,
                             int64_t ntp_time_ms,
                             bool is_keyframe,
                             uint32_t width,
                             uint32_t height);

  // Request a keyframe (used by the application layer to signal that a
  // keyframe is needed)
  void RequestKeyframe();

  // Check if a keyframe has been requested
  bool IsKeyframeRequested() const;

  // Clear the keyframe request flag
  void ClearKeyframeRequest();

 private:
  webrtc::SdpVideoFormat format_;
  webrtc::EncodedImageCallback* callback_ = nullptr;
  uint32_t width_ = 0;
  uint32_t height_ = 0;
  uint32_t target_bitrate_bps_ = 0;
  double framerate_ = 30.0;
  std::atomic<bool> keyframe_requested_{false};
  std::atomic<bool> initialized_{false};
  webrtc::Mutex mutex_;

  // Encoded image buffer
  webrtc::EncodedImage encoded_image_;
};

// Passthrough encoder factory
class PassthroughVideoEncoderFactory : public webrtc::VideoEncoderFactory {
 public:
  PassthroughVideoEncoderFactory();
  ~PassthroughVideoEncoderFactory() override;

  std::vector<webrtc::SdpVideoFormat> GetSupportedFormats() const override;

  std::unique_ptr<webrtc::VideoEncoder> Create(
      const webrtc::Environment& env,
      const webrtc::SdpVideoFormat& format) override;

  // Get the last created encoder instance (for injecting frames)
  // Note: This is only valid until the next Create() call or until the
  // encoder is destroyed.
  PassthroughVideoEncoder* GetLastEncoder() const { return last_encoder_; }

 private:
  mutable PassthroughVideoEncoder* last_encoder_ = nullptr;
};

// Free functions for cxx bindings
std::shared_ptr<PassthroughVideoEncoderFactory>
new_passthrough_video_encoder_factory();

// Inject an encoded frame into the encoder
// Returns 0 on success, non-zero error code on failure
int32_t passthrough_encoder_inject_frame(PassthroughVideoEncoder* encoder,
                                         rust::Slice<const uint8_t> data,
                                         uint32_t rtp_timestamp,
                                         int64_t capture_time_ms,
                                         int64_t ntp_time_ms,
                                         bool is_keyframe,
                                         uint32_t width,
                                         uint32_t height);

// Check if keyframe is requested
bool passthrough_encoder_is_keyframe_requested(
    const PassthroughVideoEncoder* encoder);

// Clear the keyframe request flag
void passthrough_encoder_clear_keyframe_request(
    PassthroughVideoEncoder* encoder);

// Request a keyframe
void passthrough_encoder_request_keyframe(PassthroughVideoEncoder* encoder);

// Get the last encoder from the factory
PassthroughVideoEncoder* passthrough_factory_get_encoder(
    const PassthroughVideoEncoderFactory* factory);

// Dummy shared_ptr for cxx
static std::shared_ptr<PassthroughVideoEncoderFactory>
_shared_passthrough_video_encoder_factory() {
  return nullptr;
}

// Global passthrough factory accessors (defined in video_encoder_factory.cpp)
PassthroughVideoEncoderFactory* GetGlobalPassthroughEncoderFactory();
void SetGlobalPassthroughEncoderFactory(
    std::shared_ptr<PassthroughVideoEncoderFactory> factory);

}  // namespace livekit

#endif  // LIVEKIT_PASSTHROUGH_VIDEO_ENCODER_H_
