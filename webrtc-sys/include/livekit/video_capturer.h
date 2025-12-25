#pragma once

#include "modules/video_capture/video_capture.h"
#include "modules/video_capture/video_capture_factory.h"
#include "rust/cxx.h"
#include "webrtc-sys/src/video_capturer.rs.h"

namespace livekit {

rust::Vec<VideoDevice> get_video_device_list();
}  // namespace livekit