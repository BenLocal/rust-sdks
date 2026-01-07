//! RTSP H.264 Passthrough Example
//!
//! This example demonstrates how to stream an RTSP H.264 video stream
//! to LiveKit without re-encoding. The H.264 frames are injected directly
//! into the WebRTC pipeline using the passthrough encoder.

use anyhow::{Context, Result};
use libwebrtc::prelude::{EncodedVideoFrame, EncodedVideoSource, VideoCodecType};
use livekit::prelude::*;
use livekit_api::access_token;
use std::env;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::sync::mpsc;
use tokio::time::{sleep, Instant};

// RTSP H.264 frame structure
#[derive(Debug, Clone)]
struct H264Frame {
    data: Vec<u8>,
    timestamp_us: i64,
    is_keyframe: bool,
    width: u32,
    height: u32,
}

// Simple RTSP client mock - replace this with actual RTSP client implementation
// For example, using rtsp-client crate or ffmpeg bindings
struct RtspClient {
    url: String,
    width: u32,
    height: u32,
    framerate: u32,
}

impl RtspClient {
    fn new(url: String, width: u32, height: u32, framerate: u32) -> Self {
        Self { url, width, height, framerate }
    }

    // Mock RTSP receiver - replace with actual RTSP client
    // This is a placeholder that simulates receiving H.264 frames
    async fn start(&self, mut tx: mpsc::Sender<H264Frame>, cancel: Arc<AtomicBool>) -> Result<()> {
        log::info!("Starting RTSP client for: {}", self.url);
        log::warn!("Using mock RTSP client - replace with actual RTSP implementation");

        let frame_interval = Duration::from_secs_f64(1.0 / self.framerate as f64);
        let mut frame_count = 0u64;

        loop {
            if cancel.load(Ordering::Relaxed) {
                log::info!("RTSP client stopped");
                break;
            }

            let start = Instant::now();

            // Simulate receiving an H.264 frame
            // In a real implementation, you would:
            // 1. Connect to RTSP stream
            // 2. Parse RTP packets
            // 3. Extract H.264 NAL units
            // 4. Reconstruct frames

            let is_keyframe = frame_count % (self.framerate * 2) == 0; // Keyframe every 2 seconds

            // Create a mock H.264 frame
            // In reality, this would be actual H.264 NAL unit data from RTSP
            let mock_frame_data = if is_keyframe {
                // Mock IDR frame (keyframe) - would contain SPS, PPS, and IDR NAL units
                vec![
                    0x00, 0x00, 0x00, 0x01, 0x67, // SPS NAL unit start code + type
                    // ... actual SPS data ...
                    0x00, 0x00, 0x00, 0x01, 0x68, // PPS NAL unit start code + type
                    // ... actual PPS data ...
                    0x00, 0x00, 0x00, 0x01,
                    0x65, // IDR NAL unit start code + type
                          // ... actual IDR slice data ...
                ]
            } else {
                // Mock P frame (delta frame)
                vec![
                    0x00, 0x00, 0x00, 0x01,
                    0x41, // Non-IDR slice NAL unit
                         // ... actual slice data ...
                ]
            };

            let timestamp_us =
                SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_micros() as i64;

            let frame = H264Frame {
                data: mock_frame_data,
                timestamp_us,
                is_keyframe,
                width: self.width,
                height: self.height,
            };

            if tx.send(frame).await.is_err() {
                log::warn!("Frame receiver dropped");
                break;
            }

            frame_count += 1;

            // Maintain frame rate
            let elapsed = start.elapsed();
            if elapsed < frame_interval {
                sleep(frame_interval - elapsed).await;
            }
        }

        Ok(())
    }
}

async fn publish_rtsp_h264_passthrough(
    room: &Room,
    rtsp_url: &str,
    width: u32,
    height: u32,
    framerate: u32,
) -> Result<()> {
    log::info!("Publishing RTSP H.264 passthrough stream: {}", rtsp_url);

    // 1. Create encoded video source
    let encoded_source = EncodedVideoSource::new(VideoCodecType::H264, width, height)
        .ok_or_else(|| anyhow::anyhow!("Failed to create encoded video source"))?;

    // 2. Create video track using encoded source
    let track = LocalVideoTrack::create_video_track(
        "rtsp_h264_passthrough",
        RtcVideoSource::Encoded(encoded_source.clone()),
    );

    // 3. Publish the track (this triggers encoder creation)
    let local_participant = room.local_participant();
    let publication = local_participant
        .publish_track(
            LocalTrack::Video(track),
            TrackPublishOptions {
                source: TrackSource::Camera,
                video_codec: VideoCodec::H264,
                simulcast: false, // Disable simulcast for passthrough
                ..Default::default()
            },
        )
        .await
        .context("Failed to publish track")?;

    log::info!("Track published, waiting for encoder initialization...");

    // 4. Initialize encoder (wait for it to be created by WebRTC)
    // WebRTC creates the encoder during SDP negotiation
    // Use async version for better integration with tokio
    if !encoded_source.initialize_encoder_async().await {
        return Err(anyhow::anyhow!("Failed to initialize passthrough encoder"));
    }

    log::info!("Passthrough encoder ready, starting RTSP stream...");

    // 5. Create RTSP client and frame channel
    let (frame_tx, mut frame_rx) = mpsc::channel::<H264Frame>(100);
    let rtsp_client = RtspClient::new(rtsp_url.to_string(), width, height, framerate);
    let cancel = Arc::new(AtomicBool::new(false));

    // 6. Start RTSP receiver in background
    let rtsp_cancel = cancel.clone();
    let rtsp_task = tokio::spawn(async move {
        if let Err(e) = rtsp_client.start(frame_tx, rtsp_cancel).await {
            log::error!("RTSP client error: {}", e);
        }
    });

    // 7. Process frames and inject into encoded source
    let mut rtp_timestamp = 0u32;
    let rtp_timestamp_interval = 90_000 / framerate; // 90kHz RTP clock

    // Process frames in a loop
    while let Some(h264_frame) = frame_rx.recv().await {
        // Convert to encoded video frame
        let encoded_frame = if h264_frame.is_keyframe {
            EncodedVideoFrame::keyframe(
                h264_frame.data,
                rtp_timestamp,
                h264_frame.timestamp_us,
                h264_frame.width,
                h264_frame.height,
                VideoCodecType::H264,
            )
        } else {
            EncodedVideoFrame::delta_frame(
                h264_frame.data,
                rtp_timestamp,
                h264_frame.timestamp_us,
                h264_frame.width,
                h264_frame.height,
                VideoCodecType::H264,
            )
        };

        // Push frame to encoded source (which injects into passthrough encoder)
        match encoded_source.push_frame(&encoded_frame) {
            Ok(()) => {
                // Check for keyframe requests
                if encoded_source.is_keyframe_requested() {
                    log::info!("Keyframe requested by receiver");
                    encoded_source.clear_keyframe_request();
                    // Note: In a real implementation, you would request
                    // a keyframe from the RTSP source here
                }
            }
            Err(e) => {
                log::error!("Failed to push frame: {}", e);
            }
        }

        rtp_timestamp = rtp_timestamp.wrapping_add(rtp_timestamp_interval);
    }

    log::warn!("Frame channel closed");

    // Cleanup
    cancel.store(true, Ordering::Relaxed);
    let _ = rtsp_task.await;

    // Unpublish track
    local_participant
        .unpublish_track(&publication.sid())
        .await
        .context("Failed to unpublish track")?;

    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();

    let url = env::var("LIVEKIT_URL").expect("LIVEKIT_URL is not set");
    let api_key = env::var("LIVEKIT_API_KEY").expect("LIVEKIT_API_KEY is not set");
    let api_secret = env::var("LIVEKIT_API_SECRET").expect("LIVEKIT_API_SECRET is not set");
    let rtsp_url = env::var("RTSP_URL").unwrap_or_else(|_| "rtsp://example.com/stream".to_string());

    // Generate access token
    let token = access_token::AccessToken::with_api_key(&api_key, &api_secret)
        .with_identity("rtsp-passthrough-bot")
        .with_name("RTSP Passthrough Bot")
        .with_grants(access_token::VideoGrants {
            room_join: true,
            room: "my-room".to_string(),
            ..Default::default()
        })
        .to_jwt()
        .unwrap();

    // Connect to room
    let (room, mut rx) = Room::connect(&url, &token, RoomOptions::default())
        .await
        .context("Failed to connect to room")?;

    log::info!("Connected to room: {} - {}", room.name(), String::from(room.sid().await));

    // Start publishing RTSP stream
    let publish_task = tokio::spawn({
        let room = room.clone();
        async move {
            if let Err(e) = publish_rtsp_h264_passthrough(
                &room, &rtsp_url, 1920, // width
                1080, // height
                30,   // framerate
            )
            .await
            {
                log::error!("Failed to publish RTSP stream: {}", e);
            }
        }
    });

    // Handle room events
    while let Some(msg) = rx.recv().await {
        log::info!("Room event: {:?}", msg);
    }

    let _ = publish_task.await;
    Ok(())
}
