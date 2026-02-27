# Project: Live Stream Relay Backend

## Overview
A lightweight Rust backend that receives live screen streams from Android/iOS clients and relays them to web clients via WebSockets. This backend is purely a **stream relay** — it does not perform any content filtering, porn blocking, or client-side enforcement. That responsibility belongs entirely to the mobile client.

## Core Responsibilities
- Accept incoming WebSocket connections from Android/iOS clients streaming live screen data
- Relay received stream frames in real time to connected web clients (viewers)
- Manage session/room mapping between a mobile client stream and its corresponding web viewer(s)
- Stay lightweight — no heavy processing, no video transcoding, no content analysis

## Tech Stack
- **Language:** Rust
- **WebSocket library:** `tokio-tungstenite` (async, lightweight)
- **Async runtime:** `tokio`
- **HTTP layer (for upgrade + health check):** `axum` or `hyper`
- **Serialization:** `serde` + `serde_json` for signaling messages
- Keep dependencies minimal — avoid anything that adds unnecessary weight

## Architecture

### Connection Types
1. **Streamer (mobile client):** Android or iOS device that connects and pushes raw video frame data
2. **Viewer (web client):** Browser-based frontend that connects and receives the relayed stream

### Stream Flow
```
[Android/iOS Client] --WS--> [Rust Backend] --WS--> [Web Client]
```
- Each streamer is assigned a unique **session/stream ID** on connection
- Viewers subscribe to a stream by referencing the stream ID
- The backend simply forwards binary frame data from streamer → viewer(s)
- No frame buffering beyond what's needed for relay; keep memory footprint low

## WebSocket Endpoints
- `GET /ws/stream` — Streamer connects here to push live frames
- `GET /ws/view/{stream_id}` — Viewer connects here to receive a specific stream
- `GET /health` — Simple health check endpoint

## Signaling Protocol (JSON over WebSocket text frames)

### Streamer → Server (on connect)
```json
{ "type": "register", "stream_id": "optional-custom-id-or-null" }
```
Server responds:
```json
{ "type": "registered", "stream_id": "abc123" }
```

### Viewer → Server (on connect)
```json
{ "type": "subscribe", "stream_id": "abc123" }
```
Server responds with either:
```json
{ "type": "subscribed", "stream_id": "abc123" }
```
or:
```json
{ "type": "error", "message": "stream not found" }
```

### Video Frames
- Sent as **binary WebSocket frames** (not JSON) for efficiency
- Frame format is defined by the mobile client (e.g., raw H.264 NAL units, JPEG frames, or raw RGBA) — the backend treats them as opaque binary blobs and forwards as-is
- No transcoding or processing on the backend

## Session Management
- Use a shared `Arc<RwLock<HashMap<StreamId, StreamerHandle>>>` to track active streams
- When a streamer disconnects, notify any active viewers and clean up the session
- Support multiple viewers per stream (broadcast)
- Stream IDs: short random alphanumeric strings (e.g., 8 chars), generated server-side if not provided by client

## Configuration (via environment variables)
| Variable | Default | Description |
|---|---|---|
| `HOST` | `0.0.0.0` | Bind address |
| `PORT` | `8080` | Bind port |
| `MAX_STREAMS` | `100` | Max concurrent active streams |
| `MAX_VIEWERS_PER_STREAM` | `10` | Max viewers per stream |

## Non-Goals (explicitly out of scope)
- Content filtering or porn detection — handled entirely by the mobile client
- Video storage or recording
- Authentication (can be added later by the separate frontend layer)
- Video transcoding or format conversion
- TURN/STUN or WebRTC — plain WebSockets only

## Project Structure
```
src/
├── main.rs          # Entry point, server setup
├── config.rs        # Environment config
├── session.rs       # Session/stream state management
├── handlers/
│   ├── mod.rs
│   ├── stream.rs    # Streamer WS handler
│   └── view.rs      # Viewer WS handler
└── relay.rs         # Frame relay logic (streamer → viewers broadcast)
```

## Performance Notes
- Avoid cloning frame data where possible — use `Bytes` from the `bytes` crate for cheap reference-counted binary data
- Use `tokio::sync::broadcast` channel per stream for fan-out to multiple viewers
- Keep frame relay on the async task level — no blocking operations in the hot path