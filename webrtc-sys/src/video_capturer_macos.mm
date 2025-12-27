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

#import <AVFoundation/AVFoundation.h>
#import <Foundation/Foundation.h>

// Include the header to get full type definitions
#include "livekit/video_capturer.h"

#include "modules/video_capture/video_capture.h"
#include "modules/video_capture/video_capture_defines.h"
#include "api/video/video_frame.h"
#include "api/video/video_sink_interface.h"
#include "api/video/i420_buffer.h"
#include "rtc_base/ref_counted_object.h"
#include "third_party/libyuv/include/libyuv.h"
#include <memory>
#include <string>

namespace livekit {

class MacOSVideoCaptureAdapter;

}  // namespace livekit

// Objective-C delegate to receive video frames from AVCaptureVideoDataOutput
@interface VideoCapturerDelegate : NSObject<AVCaptureVideoDataOutputSampleBufferDelegate>
@property(nonatomic, assign) livekit::MacOSVideoCaptureAdapter* adapter;
@end

namespace livekit {

// Adapter class that wraps AVCaptureSession to implement VideoCaptureModule interface
class MacOSVideoCaptureAdapter : public webrtc::VideoCaptureModule {
 public:
  explicit MacOSVideoCaptureAdapter(AVCaptureDevice* device)
      : device_(device), 
        callback_(nullptr), 
        started_(false),
        apply_rotation_(false),
        capture_session_(nil),
        video_output_(nil),
        delegate_(nil),
        queue_(nil) {
    [device_ retain];
  }

  ~MacOSVideoCaptureAdapter() override {
    StopCapture();
    if (delegate_) {
      [delegate_ release];
    }
    if (queue_) {
      dispatch_release(queue_);
    }
    if (capture_session_) {
      [capture_session_ release];
    }
    if (video_output_) {
      [video_output_ release];
    }
    [device_ release];
  }

  // VideoCaptureModule implementation
  void RegisterCaptureDataCallback(
      webrtc::VideoSinkInterface<webrtc::VideoFrame>* dataCallback) override {
    callback_ = dataCallback;
  }

  void RegisterCaptureDataCallback(
      webrtc::RawVideoSinkInterface* dataCallback) override {
    // Not implemented for macOS
  }

  void DeRegisterCaptureDataCallback() override {
    callback_ = nullptr;
  }

  int32_t StartCapture(const webrtc::VideoCaptureCapability& capability) override {
    if (started_) {
      return 0;
    }

    NSAutoreleasePool *pool = [[NSAutoreleasePool alloc] init];

    // Create capture session
    capture_session_ = [[AVCaptureSession alloc] init];
    
    // Set session preset based on capability
    if (capability.width <= 640 && capability.height <= 480) {
      [capture_session_ setSessionPreset:AVCaptureSessionPreset640x480];
    } else if (capability.width <= 1280 && capability.height <= 720) {
      [capture_session_ setSessionPreset:AVCaptureSessionPreset1280x720];
    } else {
      [capture_session_ setSessionPreset:AVCaptureSessionPreset1920x1080];
    }
    
    NSError *error = nil;
    AVCaptureDeviceInput *input = [AVCaptureDeviceInput deviceInputWithDevice:device_ error:&error];
    
    if (error || !input) {
      [pool drain];
      return -1;
    }

    if ([capture_session_ canAddInput:input]) {
      [capture_session_ addInput:input];
    } else {
      [pool drain];
      return -1;
    }

    // Create video data output
    video_output_ = [[AVCaptureVideoDataOutput alloc] init];
    
    // Set pixel format to NV12 or I420
    NSDictionary *videoSettings = @{
      (id)kCVPixelBufferPixelFormatTypeKey: @(kCVPixelFormatType_420YpCbCr8BiPlanarVideoRange)
    };
    [video_output_ setVideoSettings:videoSettings];
    
    // Discard late frames
    [video_output_ setAlwaysDiscardsLateVideoFrames:YES];
    
    // Create delegate
    delegate_ = [[VideoCapturerDelegate alloc] init];
    delegate_.adapter = this;
    
    // Create serial queue for video frame processing
    queue_ = dispatch_queue_create("com.livekit.videocapturer", DISPATCH_QUEUE_SERIAL);
    
    // Set delegate and queue
    [video_output_ setSampleBufferDelegate:delegate_ queue:queue_];
    
    if ([capture_session_ canAddOutput:video_output_]) {
      [capture_session_ addOutput:video_output_];
    } else {
      [pool drain];
      return -1;
    }

    // Start the session
    [capture_session_ startRunning];
    
    [pool drain];
    
    started_ = true;
    return 0;
  }

  int32_t StopCapture() override {
    if (!started_) {
      return 0;
    }

    NSAutoreleasePool *pool = [[NSAutoreleasePool alloc] init];
    
    if (capture_session_ && [capture_session_ isRunning]) {
      [capture_session_ stopRunning];
    }
    
    if (video_output_) {
      [video_output_ setSampleBufferDelegate:nil queue:nil];
    }
    
    [pool drain];
    
    started_ = false;
    return 0;
  }

  bool CaptureStarted() override {
    return started_;
  }

  int32_t CaptureSettings(webrtc::VideoCaptureCapability& settings) override {
    // Return current settings
    return 0;
  }

  const char* CurrentDeviceName() const override {
    return [[device_ localizedName] UTF8String];
  }

  int32_t SetCaptureRotation(webrtc::VideoRotation rotation) override {
    // Not implemented for macOS
    return 0;
  }

  bool SetApplyRotation(bool enable) override {
    apply_rotation_ = enable;
    return true;
  }

  bool GetApplyRotation() override {
    return apply_rotation_;
  }

  // Called by delegate when a new video frame is captured
  void OnFrameCaptured(CMSampleBufferRef sampleBuffer) {
    if (!callback_) {
      return;
    }

    CVImageBufferRef pixelBuffer = CMSampleBufferGetImageBuffer(sampleBuffer);
    if (!pixelBuffer) {
      return;
    }

    CVPixelBufferLockBaseAddress(pixelBuffer, kCVPixelBufferLock_ReadOnly);

    const int width = CVPixelBufferGetWidth(pixelBuffer);
    const int height = CVPixelBufferGetHeight(pixelBuffer);
    
    // Get timestamp
    CMTime timestamp = CMSampleBufferGetPresentationTimeStamp(sampleBuffer);
    int64_t timestamp_us = CMTimeGetSeconds(timestamp) * 1000000;

    // Create I420 buffer
    rtc::scoped_refptr<webrtc::I420Buffer> i420_buffer = 
        webrtc::I420Buffer::Create(width, height);

    // Convert NV12 to I420
    OSType pixelFormat = CVPixelBufferGetPixelFormatType(pixelBuffer);
    
    if (pixelFormat == kCVPixelFormatType_420YpCbCr8BiPlanarVideoRange ||
        pixelFormat == kCVPixelFormatType_420YpCbCr8BiPlanarFullRange) {
      // NV12 format
      const uint8_t* y_plane = 
          static_cast<const uint8_t*>(CVPixelBufferGetBaseAddressOfPlane(pixelBuffer, 0));
      const uint8_t* uv_plane = 
          static_cast<const uint8_t*>(CVPixelBufferGetBaseAddressOfPlane(pixelBuffer, 1));
      
      const int y_stride = CVPixelBufferGetBytesPerRowOfPlane(pixelBuffer, 0);
      const int uv_stride = CVPixelBufferGetBytesPerRowOfPlane(pixelBuffer, 1);

      libyuv::NV12ToI420(
          y_plane, y_stride,
          uv_plane, uv_stride,
          i420_buffer->MutableDataY(), i420_buffer->StrideY(),
          i420_buffer->MutableDataU(), i420_buffer->StrideU(),
          i420_buffer->MutableDataV(), i420_buffer->StrideV(),
          width, height);
    } else {
      // Other formats - try to convert via ARGB
      const uint8_t* src = 
          static_cast<const uint8_t*>(CVPixelBufferGetBaseAddress(pixelBuffer));
      const int src_stride = CVPixelBufferGetBytesPerRow(pixelBuffer);
      
      // Create temporary ARGB buffer
      std::unique_ptr<uint8_t[]> argb_buffer(new uint8_t[width * height * 4]);
      
      // Convert to ARGB first (assuming BGRA input)
      libyuv::ARGBToI420(
          src, src_stride,
          i420_buffer->MutableDataY(), i420_buffer->StrideY(),
          i420_buffer->MutableDataU(), i420_buffer->StrideU(),
          i420_buffer->MutableDataV(), i420_buffer->StrideV(),
          width, height);
    }

    CVPixelBufferUnlockBaseAddress(pixelBuffer, kCVPixelBufferLock_ReadOnly);

    // Create VideoFrame
    webrtc::VideoFrame frame = webrtc::VideoFrame::Builder()
        .set_video_frame_buffer(i420_buffer)
        .set_timestamp_us(timestamp_us)
        .set_rotation(webrtc::kVideoRotation_0)
        .build();

    // Send to callback
    callback_->OnFrame(frame);
  }

 private:
  AVCaptureDevice* device_;
  webrtc::VideoSinkInterface<webrtc::VideoFrame>* callback_;
  bool started_;
  bool apply_rotation_;
  AVCaptureSession* capture_session_;
  AVCaptureVideoDataOutput* video_output_;
  VideoCapturerDelegate* delegate_;
  dispatch_queue_t queue_;
};

}  // namespace livekit

// Implementation of VideoCapturerDelegate
@implementation VideoCapturerDelegate

- (void)captureOutput:(AVCaptureOutput *)output
    didOutputSampleBuffer:(CMSampleBufferRef)sampleBuffer
           fromConnection:(AVCaptureConnection *)connection {
  if (self.adapter) {
    self.adapter->OnFrameCaptured(sampleBuffer);
  }
}

- (void)captureOutput:(AVCaptureOutput *)output
    didDropSampleBuffer:(CMSampleBufferRef)sampleBuffer
           fromConnection:(AVCaptureConnection *)connection {
  // Frame was dropped
}

@end

namespace livekit {

rust::Vec<VideoDevice> get_video_device_list_macos() {
  rust::Vec<VideoDevice> devices;
  
  NSAutoreleasePool *pool = [[NSAutoreleasePool alloc] init];
  
  // Use AVFoundation to get devices
  NSArray *captureDevices = [AVCaptureDevice devicesWithMediaType:AVMediaTypeVideo];
  
  for (NSInteger i = 0; i < [captureDevices count]; ++i) {
    AVCaptureDevice *device = [captureDevices objectAtIndex:i];
    NSString *deviceName = [device localizedName];
    NSString *deviceUID = [device uniqueID];
    NSString *devicePID = [device modelID];
    if (!devicePID) {
      devicePID = @"";
    }
    
    const char *nameCStr = [deviceName UTF8String];
    const char *uidCStr = [deviceUID UTF8String];
    const char *pidCStr = [devicePID UTF8String];
    
    std::string name = nameCStr ? std::string(nameCStr) : std::string("");
    std::string uid = uidCStr ? std::string(uidCStr) : std::string("");
    std::string pid = pidCStr ? std::string(pidCStr) : std::string("");
    
    // VideoDevice order: index, name, uid, pid
    devices.push_back(VideoDevice{static_cast<int32_t>(i), name, uid, pid});
  }
  
  [pool drain];
  
  return devices;
}

std::unique_ptr<VideoCapturer> new_video_capturer_macos(
    rust::Str deviceUniqueIdUTF8) {
  NSAutoreleasePool *pool = [[NSAutoreleasePool alloc] init];
  
  std::string id_str(deviceUniqueIdUTF8.data(), deviceUniqueIdUTF8.size());
  NSString *deviceId = [NSString stringWithUTF8String:id_str.c_str()];
  
  // Find the device by unique ID
  NSArray *devices = [AVCaptureDevice devicesWithMediaType:AVMediaTypeVideo];
  
  AVCaptureDevice *targetDevice = nil;
  for (NSInteger i = 0; i < [devices count]; ++i) {
    AVCaptureDevice *device = [devices objectAtIndex:i];
    if ([[device uniqueID] isEqualToString:deviceId]) {
      targetDevice = device;
      break;
    }
  }
  
  if (!targetDevice) {
    [pool drain];
    return nullptr;
  }
  
  // Create adapter
  webrtc::scoped_refptr<MacOSVideoCaptureAdapter> adapter(
      new rtc::RefCountedObject<MacOSVideoCaptureAdapter>(targetDevice));
  
  [pool drain];
  
  if (!adapter) {
    return nullptr;
  }
  
  return std::make_unique<VideoCapturer>(adapter);
}

}  // namespace livekit
