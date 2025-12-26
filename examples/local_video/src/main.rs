use std::env;

use futures_util::StreamExt as _;
use libwebrtc::{
    prelude::{RtcVideoSource, VideoResolution},
    video_capturer::{VideoCaptureCapability, VideoCapturer},
    video_source::native::NativeVideoSource,
};
use livekit::{
    Room, RoomOptions,
    options::{TrackPublishOptions, VideoCodec},
    track::{LocalTrack, LocalVideoTrack, TrackSource},
};
use livekit_api::access_token::{self};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    env_logger::init();
    let device_list = VideoCapturer::device_list();
    for device in device_list.iter() {
        println!(
            "Device: {}, index: {}, unique_id: {}, product_id:{}",
            device.name(),
            device.index(),
            device.unique_id(),
            device.product_id()
        );
    }

    if device_list.is_empty() {
        panic!("No video capturer found");
    }

    let url = env::var("LIVEKIT_URL").expect("LIVEKIT_URL is not set");
    let api_key = env::var("LIVEKIT_API_KEY").expect("LIVEKIT_API_KEY is not set");
    let api_secret = env::var("LIVEKIT_API_SECRET").expect("LIVEKIT_API_SECRET is not set");

    let token = access_token::AccessToken::with_api_key(&api_key, &api_secret)
        .with_identity("rust-bot")
        .with_name("Rust Bot")
        .with_grants(access_token::VideoGrants {
            room_join: true,
            room: "dev_room".to_string(),
            ..Default::default()
        })
        .to_jwt()
        .unwrap();

    let (room, _) = Room::connect(&url, &token, RoomOptions::default()).await.unwrap();
    log::info!("Connected to room: {} - {}", room.name(), String::from(room.sid().await));

    let first = device_list.first().unwrap();
    let video_capturer = VideoCapturer::open_device(&first.unique_id());
    let (video_capturer, mut stream) = match video_capturer {
        Some(x) => x,
        None => panic!("Can not open video capturer"),
    };

    let buffer_source = NativeVideoSource::new(VideoResolution::default());
    let track = LocalVideoTrack::create_video_track(
        "camera",
        RtcVideoSource::Native(buffer_source.clone()),
    );

    if !video_capturer.start(VideoCaptureCapability::default()) {
        panic!("Can not start video capturer");
    }

    tokio::spawn(async move {
        while let Some(frame) = stream.next().await {
            buffer_source.capture_frame(&frame);
        }
    });

    room.local_participant()
        .publish_track(
            LocalTrack::Video(track),
            TrackPublishOptions {
                source: TrackSource::Camera,
                video_codec: VideoCodec::VP9,
                ..Default::default()
            },
        )
        .await
        .unwrap();

    tokio::signal::ctrl_c().await?;
    println!("Ctrl-C received, stopping video capturer");

    video_capturer.stop();
    Ok(())
}
