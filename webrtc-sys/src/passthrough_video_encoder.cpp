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

#include "livekit/passthrough_video_encoder.h"

#include <algorithm>

#include "api/video/video_codec_constants.h"
#include "common_video/h264/h264_common.h"
#include "modules/video_coding/include/video_error_codes.h"
#include "rtc_base/logging.h"

namespace livekit {

PassthroughVideoEncoder::PassthroughVideoEncoder(
    const webrtc::SdpVideoFormat& format)
    : format_(format) {
  RTC_LOG(LS_INFO) << "PassthroughVideoEncoder created for codec: "
                   << format_.name;
}

PassthroughVideoEncoder::~PassthroughVideoEncoder() {
  Release();
}

int32_t PassthroughVideoEncoder::InitEncode(
    const webrtc::VideoCodec* codec_settings,
    const webrtc::VideoEncoder::Settings& settings) {
  if (!codec_settings) {
    RTC_LOG(LS_ERROR)
        << "PassthroughVideoEncoder::InitEncode: null codec settings";
    return WEBRTC_VIDEO_CODEC_ERR_PARAMETER;
  }

  webrtc::MutexLock lock(&mutex_);

  width_ = codec_settings->width;
  height_ = codec_settings->height;
  target_bitrate_bps_ = codec_settings->startBitrate * 1000;
  framerate_ = codec_settings->maxFramerate;

  // Initialize encoded image buffer
  encoded_image_.SetEncodedData(webrtc::EncodedImageBuffer::Create(0));
  encoded_image_._encodedWidth = width_;
  encoded_image_._encodedHeight = height_;
  encoded_image_.set_size(0);

  initialized_ = true;

  RTC_LOG(LS_INFO) << "PassthroughVideoEncoder initialized: " << width_ << "x"
                   << height_ << " @ " << framerate_
                   << "fps, bitrate=" << target_bitrate_bps_ << "bps";

  return WEBRTC_VIDEO_CODEC_OK;
}

int32_t PassthroughVideoEncoder::RegisterEncodeCompleteCallback(
    webrtc::EncodedImageCallback* callback) {
  webrtc::MutexLock lock(&mutex_);
  callback_ = callback;
  return WEBRTC_VIDEO_CODEC_OK;
}

int32_t PassthroughVideoEncoder::Release() {
  webrtc::MutexLock lock(&mutex_);
  callback_ = nullptr;
  initialized_ = false;
  RTC_LOG(LS_INFO) << "PassthroughVideoEncoder released";
  return WEBRTC_VIDEO_CODEC_OK;
}

int32_t PassthroughVideoEncoder::Encode(
    const webrtc::VideoFrame& frame,
    const std::vector<webrtc::VideoFrameType>* frame_types) {
  // This method is called by WebRTC's video send stream, but we don't use it.
  // Instead, we inject pre-encoded frames via InjectEncodedFrame().
  //
  // However, we should check for keyframe requests here and set the flag.
  if (frame_types && !frame_types->empty()) {
    if ((*frame_types)[0] == webrtc::VideoFrameType::kVideoFrameKey) {
      keyframe_requested_ = true;
    }
  }

  // Return OK but don't produce any output - output comes from
  // InjectEncodedFrame()
  return WEBRTC_VIDEO_CODEC_OK;
}

void PassthroughVideoEncoder::SetRates(
    const RateControlParameters& parameters) {
  webrtc::MutexLock lock(&mutex_);

  if (parameters.framerate_fps > 0) {
    framerate_ = parameters.framerate_fps;
  }

  target_bitrate_bps_ = parameters.bitrate.get_sum_bps();

  RTC_LOG(LS_VERBOSE) << "PassthroughVideoEncoder::SetRates: bitrate="
                      << target_bitrate_bps_ << "bps, framerate=" << framerate_;
}

webrtc::VideoEncoder::EncoderInfo PassthroughVideoEncoder::GetEncoderInfo()
    const {
  EncoderInfo info;
  info.supports_native_handle = false;
  info.implementation_name = "Passthrough H264 Encoder";
  info.scaling_settings = VideoEncoder::ScalingSettings::kOff;
  info.is_hardware_accelerated = false;
  info.supports_simulcast = false;
  // We don't actually process frames, so we accept any format
  info.preferred_pixel_formats = {webrtc::VideoFrameBuffer::Type::kI420};
  return info;
}

int32_t PassthroughVideoEncoder::InjectEncodedFrame(const uint8_t* data,
                                                    size_t size,
                                                    uint32_t rtp_timestamp,
                                                    int64_t capture_time_ms,
                                                    int64_t ntp_time_ms,
                                                    bool is_keyframe,
                                                    uint32_t width,
                                                    uint32_t height) {
  webrtc::MutexLock lock(&mutex_);

  if (!initialized_) {
    RTC_LOG(LS_ERROR) << "PassthroughVideoEncoder::InjectEncodedFrame: "
                         "encoder not initialized";
    return WEBRTC_VIDEO_CODEC_UNINITIALIZED;
  }

  if (!callback_) {
    RTC_LOG(LS_ERROR) << "PassthroughVideoEncoder::InjectEncodedFrame: "
                         "no callback registered";
    return WEBRTC_VIDEO_CODEC_UNINITIALIZED;
  }

  if (!data || size == 0) {
    RTC_LOG(LS_ERROR) << "PassthroughVideoEncoder::InjectEncodedFrame: "
                         "invalid data";
    return WEBRTC_VIDEO_CODEC_ERR_PARAMETER;
  }

  // Update dimensions if provided
  if (width > 0 && height > 0) {
    encoded_image_._encodedWidth = width;
    encoded_image_._encodedHeight = height;
  }

  // Set timestamps
  encoded_image_.SetRtpTimestamp(rtp_timestamp);
  encoded_image_.capture_time_ms_ = capture_time_ms;
  encoded_image_.ntp_time_ms_ = ntp_time_ms;
  encoded_image_.SetSimulcastIndex(0);
  encoded_image_.rotation_ = webrtc::kVideoRotation_0;
  encoded_image_.content_type_ = webrtc::VideoContentType::UNSPECIFIED;
  encoded_image_.timing_.flags = webrtc::VideoSendTiming::kInvalid;

  // Set frame type
  if (is_keyframe) {
    encoded_image_._frameType = webrtc::VideoFrameType::kVideoFrameKey;
    // Clear keyframe request since we're providing one
    keyframe_requested_ = false;
  } else {
    encoded_image_._frameType = webrtc::VideoFrameType::kVideoFrameDelta;
  }

  // Copy the encoded data
  encoded_image_.SetEncodedData(webrtc::EncodedImageBuffer::Create(data, size));
  encoded_image_.set_size(size);

  // Set codec-specific info for H264
  webrtc::CodecSpecificInfo codec_info;
  codec_info.codecType = webrtc::kVideoCodecH264;
  codec_info.codecSpecific.H264.packetization_mode =
      webrtc::H264PacketizationMode::NonInterleaved;

  // Call the callback to send the frame
  const auto result = callback_->OnEncodedImage(encoded_image_, &codec_info);
  if (result.error != webrtc::EncodedImageCallback::Result::OK) {
    RTC_LOG(LS_ERROR)
        << "PassthroughVideoEncoder::InjectEncodedFrame: callback failed with "
        << result.error;
    return WEBRTC_VIDEO_CODEC_ERROR;
  }

  return WEBRTC_VIDEO_CODEC_OK;
}

void PassthroughVideoEncoder::RequestKeyframe() {
  keyframe_requested_ = true;
}

bool PassthroughVideoEncoder::IsKeyframeRequested() const {
  return keyframe_requested_.load();
}

void PassthroughVideoEncoder::ClearKeyframeRequest() {
  keyframe_requested_ = false;
}

// PassthroughVideoEncoderFactory implementation

PassthroughVideoEncoderFactory::PassthroughVideoEncoderFactory() {
  RTC_LOG(LS_INFO) << "PassthroughVideoEncoderFactory created";
}

PassthroughVideoEncoderFactory::~PassthroughVideoEncoderFactory() {
  RTC_LOG(LS_INFO) << "PassthroughVideoEncoderFactory destroyed";
}

std::vector<webrtc::SdpVideoFormat>
PassthroughVideoEncoderFactory::GetSupportedFormats() const {
  std::vector<webrtc::SdpVideoFormat> formats;

  // Support H.264 Constrained Baseline Profile
  formats.push_back(webrtc::SdpVideoFormat(
      "H264",
      {{"level-asymmetry-allowed", "1"},
       {"packetization-mode", "1"},
       {"profile-level-id", "42e01f"}}));  // Constrained Baseline, Level 3.1

  // Also support Baseline Profile
  formats.push_back(webrtc::SdpVideoFormat(
      "H264", {{"level-asymmetry-allowed", "1"},
               {"packetization-mode", "1"},
               {"profile-level-id", "42001f"}}));  // Baseline, Level 3.1

  // Support High Profile
  formats.push_back(webrtc::SdpVideoFormat(
      "H264", {{"level-asymmetry-allowed", "1"},
               {"packetization-mode", "1"},
               {"profile-level-id", "640c1f"}}));  // High, Level 3.1

  return formats;
}

std::unique_ptr<webrtc::VideoEncoder> PassthroughVideoEncoderFactory::Create(
    const webrtc::Environment& env,
    const webrtc::SdpVideoFormat& format) {
  auto encoder = std::make_unique<PassthroughVideoEncoder>(format);
  last_encoder_ = encoder.get();
  RTC_LOG(LS_INFO) << "PassthroughVideoEncoderFactory created encoder for "
                   << format.name;
  return encoder;
}

// Free functions for cxx bindings

std::shared_ptr<PassthroughVideoEncoderFactory>
new_passthrough_video_encoder_factory() {
  return std::make_shared<PassthroughVideoEncoderFactory>();
}

int32_t passthrough_encoder_inject_frame(PassthroughVideoEncoder* encoder,
                                         rust::Slice<const uint8_t> data,
                                         uint32_t rtp_timestamp,
                                         int64_t capture_time_ms,
                                         int64_t ntp_time_ms,
                                         bool is_keyframe,
                                         uint32_t width,
                                         uint32_t height) {
  if (!encoder) {
    RTC_LOG(LS_ERROR) << "passthrough_encoder_inject_frame: null encoder";
    return -1;
  }
  return encoder->InjectEncodedFrame(data.data(), data.size(), rtp_timestamp,
                                     capture_time_ms, ntp_time_ms, is_keyframe,
                                     width, height);
}

bool passthrough_encoder_is_keyframe_requested(
    const PassthroughVideoEncoder* encoder) {
  if (!encoder) {
    return false;
  }
  return encoder->IsKeyframeRequested();
}

void passthrough_encoder_clear_keyframe_request(
    PassthroughVideoEncoder* encoder) {
  if (encoder) {
    encoder->ClearKeyframeRequest();
  }
}

void passthrough_encoder_request_keyframe(PassthroughVideoEncoder* encoder) {
  if (encoder) {
    encoder->RequestKeyframe();
  }
}

PassthroughVideoEncoder* passthrough_factory_get_encoder(
    const PassthroughVideoEncoderFactory* factory) {
  if (!factory) {
    return nullptr;
  }
  return factory->GetLastEncoder();
}

}  // namespace livekit
