#include "livekit/video_capturer.h"

namespace livekit {

int32_t VideoCapturer::start_capture(
    const VideoCaptureCapability capability) const {
  webrtc::VideoCaptureCapability webrtc_capability;
  webrtc_capability.width = capability.width;
  webrtc_capability.height = capability.height;
  webrtc_capability.maxFPS = capability.maxFPS;
  webrtc_capability.videoType = webrtc::VideoType::kUnknown;
  webrtc_capability.interlaced = capability.interlaced;
  return capture_module_->StartCapture(webrtc_capability);
}

int32_t VideoCapturer::stop_capture() const {
  return capture_module_->StopCapture();
}

void VideoCapturer::register_capture_data_callback(
    const std::shared_ptr<NativeVideoSink>& sink) const {
  capture_module_->RegisterCaptureDataCallback(sink.get());
}

void VideoCapturer::deregister_capture_data_callback() const {
  capture_module_->DeRegisterCaptureDataCallback();
}

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
    if (info->GetDeviceName(i, name, nameSize, uid, uidSize, pid, pidSize) ==
        0) {
      devices.push_back(VideoDevice{i, name, uid, pid});
    }
  }
  return devices;
}

std::unique_ptr<VideoCapturer> new_video_capturer(
    rust::Str deviceUniqueIdUTF8) {
  std::string id_str(deviceUniqueIdUTF8.data(), deviceUniqueIdUTF8.size());
  webrtc::scoped_refptr<webrtc::VideoCaptureModule> capture_module(
      webrtc::VideoCaptureFactory::Create(id_str.c_str()));
  if (capture_module.get() == nullptr) {
    return nullptr;
  }
  return std::make_unique<VideoCapturer>(capture_module);
}
}  // namespace livekit