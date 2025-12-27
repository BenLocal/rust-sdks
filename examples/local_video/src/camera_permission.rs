// macOS 摄像头权限检查工具
// 使用 objc2 调用 AVFoundation API

#[cfg(target_os = "macos")]
mod macos {
    use objc2::msg_send;
    use objc2::runtime::AnyClass;

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum CameraPermissionStatus {
        NotDetermined, // 尚未请求权限 (AVAuthorizationStatusNotDetermined = 0)
        Restricted,    // 受限制（家长控制等）(AVAuthorizationStatusRestricted = 1)
        Denied,        // 已拒绝 (AVAuthorizationStatusDenied = 2)
        Authorized,    // 已授权 (AVAuthorizationStatusAuthorized = 3)
    }

    impl CameraPermissionStatus {
        pub fn is_authorized(&self) -> bool {
            matches!(self, CameraPermissionStatus::Authorized)
        }

        pub fn can_request(&self) -> bool {
            matches!(self, CameraPermissionStatus::NotDetermined)
        }
    }

    /// 检查摄像头权限状态
    pub fn check_camera_permission() -> CameraPermissionStatus {
        unsafe {
            use std::ffi::CStr;

            // 获取 AVCaptureDevice 类
            let av_capture_device =
                match AnyClass::get(CStr::from_bytes_with_nul(b"AVCaptureDevice\0").unwrap()) {
                    Some(cls) => cls,
                    None => {
                        log::warn!("AVCaptureDevice class not found");
                        return CameraPermissionStatus::NotDetermined;
                    }
                };

            // AVMediaTypeVideo 是一个 NSString 常量，值为 "vide"
            // 我们需要创建一个 NSString 对象
            use std::ffi::CString;
            let media_type_video_cstr = CString::new("vide").unwrap();

            // 创建 NSString 对象
            let nsstring_class =
                match AnyClass::get(CStr::from_bytes_with_nul(b"NSString\0").unwrap()) {
                    Some(cls) => cls,
                    None => {
                        log::warn!("NSString class not found");
                        return CameraPermissionStatus::NotDetermined;
                    }
                };

            let media_type_video: *mut objc2::runtime::AnyObject =
                msg_send![nsstring_class, stringWithUTF8String: media_type_video_cstr.as_ptr()];

            // 调用类方法: +[AVCaptureDevice authorizationStatusForMediaType:]
            // NSInteger 在 64 位系统上是 i64 (long long)
            let status: i64 =
                msg_send![av_capture_device, authorizationStatusForMediaType: media_type_video];

            match status {
                0 => CameraPermissionStatus::NotDetermined,
                1 => CameraPermissionStatus::Restricted,
                2 => CameraPermissionStatus::Denied,
                3 => CameraPermissionStatus::Authorized,
                _ => {
                    log::warn!("Unknown authorization status: {}", status);
                    CameraPermissionStatus::NotDetermined
                }
            }
        }
    }

    /// 触发摄像头权限请求
    /// 通过尝试访问摄像头设备来触发 macOS 的权限提示框
    /// 注意：对于命令行应用，权限提示可能不会立即显示，需要实际访问设备
    /// 简化版本：只获取设备，不创建 session，避免 AVCapture 错误
    pub fn trigger_permission_request() {
        unsafe {
            use std::ffi::CStr;
            use std::ffi::CString;

            let av_capture_device =
                match AnyClass::get(CStr::from_bytes_with_nul(b"AVCaptureDevice\0").unwrap()) {
                    Some(cls) => cls,
                    None => {
                        log::warn!("AVCaptureDevice class not found");
                        return;
                    }
                };

            let nsstring_class =
                match AnyClass::get(CStr::from_bytes_with_nul(b"NSString\0").unwrap()) {
                    Some(cls) => cls,
                    None => {
                        log::warn!("NSString class not found");
                        return;
                    }
                };

            let media_type_video_cstr = CString::new("vide").unwrap();
            let media_type_video: *mut objc2::runtime::AnyObject =
                msg_send![nsstring_class, stringWithUTF8String: media_type_video_cstr.as_ptr()];

            // 只尝试获取默认设备，这会触发权限检查
            // 不创建 session 或 input，避免 AVCapture 配置错误
            let _device: *mut objc2::runtime::AnyObject =
                msg_send![av_capture_device, defaultDeviceWithMediaType: media_type_video];

            log::info!(
                "Permission request triggered. The permission dialog should appear when you try to access the camera."
            );
        }
    }

    /// 请求摄像头权限（异步）
    /// 注意：这需要创建 Objective-C block，实现较复杂
    /// 对于命令行应用，通常权限会在首次访问摄像头时自动弹出
    pub async fn request_camera_permission() -> bool {
        let status = check_camera_permission();

        if status.is_authorized() {
            return true;
        }

        if !status.can_request() {
            log::error!("Camera permission cannot be requested. Status: {:?}", status);
            return false;
        }

        // 触发权限请求
        log::info!("Triggering camera permission request...");
        trigger_permission_request();

        // 等待权限对话框显示和用户响应
        // macOS 权限对话框是异步的，需要给足够的时间
        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

        // 轮询检查权限状态，最多等待 10 秒
        let max_wait = 10;
        let mut waited = 0;
        loop {
            let new_status = check_camera_permission();
            if new_status.is_authorized() {
                return true;
            }
            if new_status == CameraPermissionStatus::Denied {
                return false;
            }
            if waited >= max_wait {
                log::warn!(
                    "Permission request timeout. User may need to grant permission manually."
                );
                return false;
            }
            tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
            waited += 1;
        }
    }
}

#[cfg(target_os = "macos")]
pub use macos::{
    CameraPermissionStatus, check_camera_permission, request_camera_permission,
    trigger_permission_request,
};

#[cfg(not(target_os = "macos"))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CameraPermissionStatus {
    NotDetermined,
    Restricted,
    Denied,
    Authorized,
}

#[cfg(not(target_os = "macos"))]
impl CameraPermissionStatus {
    pub fn is_authorized(&self) -> bool {
        matches!(self, CameraPermissionStatus::Authorized)
    }

    pub fn can_request(&self) -> bool {
        matches!(self, CameraPermissionStatus::NotDetermined)
    }
}

#[cfg(not(target_os = "macos"))]
pub fn check_camera_permission() -> CameraPermissionStatus {
    CameraPermissionStatus::Authorized // 非 macOS 平台默认授权
}

#[cfg(not(target_os = "macos"))]
pub fn trigger_permission_request() {
    // 非 macOS 平台不需要权限请求
}

#[cfg(not(target_os = "macos"))]
pub async fn request_camera_permission() -> bool {
    true
}
