#pragma once

#include "modules/video_capture/video_capture.h"
#include "modules/video_capture/video_capture_factory.h"
#include "modules/video_capture/video_capture_options.h"
#include "rust/cxx.h"

namespace livekit_ffi {
class VideoCapturer;
class VideoDevice;
class VideoCaptureCapability;
}  // namespace livekit_ffi

#include "livekit/video_track.h"
#include "webrtc-sys/src/video_capturer.rs.h"

namespace livekit_ffi {
class VideoCapturer {
 public:
  explicit VideoCapturer(
      webrtc::scoped_refptr<webrtc::VideoCaptureModule> capture_module)
      : capture_module_(capture_module) {}

  int32_t start_capture(const VideoCaptureCapability capability) const;

  int32_t stop_capture() const;

  void register_capture_data_callback(
      const std::shared_ptr<NativeVideoSink>& sink) const;
  void deregister_capture_data_callback() const;

 private:
  webrtc::scoped_refptr<webrtc::VideoCaptureModule> capture_module_;
};

#if defined(__APPLE__)
rust::Vec<VideoDevice> get_video_device_list_macos();
std::unique_ptr<VideoCapturer> new_video_capturer_macos(
    rust::Str deviceUniqueIdUTF8);
#endif

rust::Vec<VideoDevice> get_video_device_list();

std::unique_ptr<VideoCapturer> new_video_capturer(rust::Str deviceUniqueIdUTF8);
}  // namespace livekit_ffi