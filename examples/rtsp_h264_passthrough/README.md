# RTSP H.264 Passthrough Example

This example demonstrates how to stream an RTSP H.264 video stream to LiveKit without re-encoding. The H.264 frames are injected directly into the WebRTC pipeline using the passthrough encoder.

## How It Works

1. **Create Video Track**: A video track is created with a `NativeVideoSource` (this is required to trigger encoder creation in WebRTC).

2. **Publish Track**: The track is published with H.264 codec, which triggers WebRTC to create an encoder during SDP negotiation.

3. **Get Passthrough Encoder**: After SDP negotiation, the passthrough encoder factory provides access to the created encoder.

4. **Receive RTSP Stream**: An RTSP client receives H.264 frames from the RTSP stream.

5. **Inject Frames**: The received H.264 frames are injected directly into the passthrough encoder, bypassing any re-encoding.

## Prerequisites

- LiveKit server running
- RTSP stream with H.264 codec
- Environment variables set (see below)

## Usage

Set the following environment variables:

```bash
export LIVEKIT_URL="wss://your-livekit-server.com"
export LIVEKIT_API_KEY="your-api-key"
export LIVEKIT_API_SECRET="your-api-secret"
export RTSP_URL="rtsp://your-rtsp-server.com/stream"  # Optional, defaults to mock
```

Run the example:

```bash
cargo run --example rtsp_h264_passthrough
```

## Implementation Notes

### Current Implementation

The example includes a **mock RTSP client** that simulates receiving H.264 frames. To use with a real RTSP stream, you need to:

1. **Replace the Mock RTSP Client**: Use an actual RTSP client library such as:

   - `rtsp-client` crate
   - FFmpeg bindings (`ffmpeg-next` or `ffmpeg-sys`)
   - GStreamer bindings

2. **Parse H.264 NAL Units**: Extract H.264 NAL units from the RTSP stream:

   - SPS (Sequence Parameter Set) - contains video resolution, frame rate, etc.
   - PPS (Picture Parameter Set) - contains encoding parameters
   - IDR frames (keyframes)
   - P/B frames (delta frames)

3. **Handle RTP Packets**: RTSP typically uses RTP for media transport:
   - Parse RTP headers
   - Extract H.264 payload
   - Reassemble fragmented NAL units (if using FU-A fragmentation)

### Example with FFmpeg

Here's a conceptual example of how to integrate with FFmpeg:

```rust
// Pseudo-code for FFmpeg integration
use ffmpeg_next as ffmpeg;

// Open RTSP stream
let mut ictx = ffmpeg::format::input(&rtsp_url)?;

// Find video stream
let video_stream = ictx.streams()
    .best(ffmpeg::media::Type::Video)
    .ok_or("No video stream found")?;

// Decode packets
for (stream, packet) in ictx.packets() {
    if stream.index() == video_stream.index() {
        // Extract H.264 data from packet
        let h264_data = packet.data();

        // Create encoded frame
        let frame = EncodedVideoFrame::new(
            h264_data.to_vec(),
            packet.pts().unwrap_or(0) as u32,
            SystemTime::now().duration_since(UNIX_EPOCH)?.as_millis() as i64,
            packet.is_key(),
            width,
            height,
        );

        // Inject into passthrough encoder
        encoder_handle.inject_frame(&frame)?;
    }
}
```

### Keyframe Handling

The passthrough encoder supports keyframe requests:

```rust
// Check if keyframe is requested
if encoder_handle.is_keyframe_requested() {
    // Request keyframe from RTSP source
    // (implementation depends on your RTSP client)
    encoder_handle.clear_keyframe_request();
}
```

## Architecture

```
RTSP Stream → RTSP Client → H.264 Parser → Passthrough Encoder → WebRTC RTP → LiveKit
```

The passthrough encoder bypasses the normal encoding pipeline:

- Normal: VideoFrame (YUV) → Encoder → EncodedImage → RTP
- Passthrough: H.264 NAL Units → Passthrough Encoder → EncodedImage → RTP

## Limitations

1. **Codec Matching**: The RTSP stream must use H.264 codec matching one of the supported profiles:

   - Constrained Baseline Profile (42e01f)
   - Baseline Profile (42001f)
   - High Profile (640c1f)

2. **NAL Unit Format**: H.264 NAL units must be in Annex-B format (with start codes: 0x00 0x00 0x00 0x01)

3. **Frame Timing**: Proper RTP timestamps must be provided to maintain synchronization

4. **Keyframe Requests**: The RTSP source should support keyframe requests when needed

## Troubleshooting

- **Encoder not found**: Wait longer after publishing the track (increase the sleep duration)
- **Frame injection fails**: Ensure H.264 data is valid and in Annex-B format
- **Keyframe issues**: Make sure keyframes are properly marked (IDR NAL units)
