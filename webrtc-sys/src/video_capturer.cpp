#include "livekit/video_capturer.h"

namespace livekit {

rust::Vec<VideoDevice> get_video_device_list() {
  rust::Vec<VideoDevice> devices = {};
  std::unique_ptr<webrtc::VideoCaptureModule::DeviceInfo> info(
      webrtc::VideoCaptureFactory::CreateDeviceInfo());
  if (!info) {
    return devices;
  }
  int num_devices = info->NumberOfDevices();
  if (num_devices == 0) {
    return devices;
  }
  constexpr uint32_t nameSize = 256;
  constexpr uint32_t pidSize = 256;
  constexpr uint32_t uidSize = 256;
  for (int i = 0; i < num_devices; ++i) {
    char name[nameSize] = {};
    char uid[uidSize] = {};
    char pid[pidSize] = {};
    if (info->GetDeviceName(i, name, nameSize, pid, pidSize, uid, uidSize) ==
        0) {
      devices.push_back(VideoDevice{i, name, pid, uid});
    }
  }
  return devices;
}
}  // namespace livekit