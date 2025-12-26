use libwebrtc::video_capturer::VideoCapturer;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let device_list = VideoCapturer::device_list();
    for device in device_list {
        println!(
            "Device: {}, index: {}, unique_id: {}, product_id:{}",
            device.name(),
            device.index(),
            device.unique_id(),
            device.product_id()
        );
    }

    Ok(())
}
