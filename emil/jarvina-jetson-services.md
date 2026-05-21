# Jarvina Jetson Services — Architecture Reference
## Sparked Matter LLC — March 30, 2026

---

## Services (Rust Binaries)

### 1. twilio-bridge
- **Location**: `/home/rocketman/twilio-bridge/`
- **Purpose**: Twilio ↔ PipeWire translator
- **Port**: 5000 (WebSocket + HTTP)
- **What it does**:
  - HTTP webhook `/voice` — returns TwiML to Twilio
  - WebSocket `/ws` — bidirectional Twilio Media Stream
  - Inbound: base64 decode → mu-law decode (Rust) → upsample 8kHz→16kHz → PipeWire source node
  - Outbound: PipeWire sink node → downsample 16kHz→8kHz → mu-law encode (Rust) → base64 → Twilio
  - Plays pre-generated Kokoro greeting on call pickup
- **PipeWire nodes created** (during call):
  - `jarvina-in` (Audio/Source, Direction::Output) — pushes phone audio INTO PipeWire
  - `jarvina-out` (Audio/Sink, Direction::Input) — captures Jarvina's voice FROM PipeWire
- **Launch**:
  ```bash
  cd /home/rocketman/twilio-bridge
  XDG_RUNTIME_DIR=/run/user/1000 ./target/debug/twilio-bridge
  ```

### 2. voice-agent-rust (Jarvina)
- **Location**: `/home/rocketman/voice-agent-rust/`
- **Purpose**: The AI brain — listen, think, speak
- **What it does**:
  - PipeWire capture stream (ear) → VAD → STT → LLM → TTS → PipeWire playback stream (mouth)
  - VAD: Silero (512-sample window, CPU)
  - STT: Parakeet TDT v3 INT8 (CUDA)
  - LLM: Haiku API (cloud) or Nemotron (local Ollama)
  - TTS: Kokoro af_heart speaker 3 (CUDA)
- **PipeWire nodes** (persistent):
  - `jarvina-rust:input_MONO` — capture stream (her ear)
  - `jarvina-rust-out:capture_MONO` — playback stream (her mouth)
- **Environment variables**:
  - `JARVINA_LLM` — `haiku` or `local`
  - `JARVINA_API_KEY` — Anthropic API key (for Haiku)
  - `JARVINA_PLAYBACK` — `pw-stream` (default)
  - `SHERPA_ONNX_LIB_DIR` — path to GPU libraries
  - `LD_LIBRARY_PATH` — same as above
- **Launch**:
  ```bash
  cd /home/rocketman/voice-agent-rust
  SHERPA_ONNX_LIB_DIR=/home/rocketman/downloads/sherpa-onnx-v1.12.34-linux-aarch64-shared-gpu-onnxruntime-1.18.1/lib \
  LD_LIBRARY_PATH=/home/rocketman/downloads/sherpa-onnx-v1.12.34-linux-aarch64-shared-gpu-onnxruntime-1.18.1/lib \
  JARVINA_LLM=haiku \
  JARVINA_API_KEY=$(cat /home/rocketman/.anthropic_key) \
  ./target/debug/voice-agent-rust
  ```

---

## PipeWire Link Map (during a call)

```
INBOUND (phone → Jarvina):
  jarvina-in:output_MONO → jarvina-rust:input_MONO

OUTBOUND (Jarvina → phone):
  jarvina-rust-out:capture_MONO → jarvina-out:input_MONO
```

---

## External Dependencies

| Component | Location | Purpose |
|-----------|----------|---------|
| Tailscale Funnel | `https://ubuntu.tail12e909.ts.net` | Exposes port 5000 to internet |
| Twilio | Cloud | Phone number +18136076219, Media Streams |
| Anthropic API | Cloud | Haiku LLM for fast responses |
| PipeWire | System service | Audio routing (the matrix) |
| WirePlumber | System service | Session manager / auto-linking |

---

## Key Files on Jetson

| File | Purpose |
|------|---------|
| `/home/rocketman/.anthropic_key` | Haiku API key |
| `/tmp/jarvina-greeting.ul` | Pre-generated Kokoro greeting (mu-law 8kHz) |
| `/tmp/jarvina.log` | Jarvina runtime log |
| `/tmp/twilio-bridge.log` | Bridge runtime log |
| `/usr/share/wireplumber/scripts/51-sony-hfp-lock.lua` | Sony HFP auto-lock |
| `/etc/wireplumber/main.lua.d/51-sony-hfp-enable.lua` | Loads the HFP lock script |

---

## No GStreamer

GStreamer has been completely removed from the Jetson (`apt-get remove gstreamer1.0-*`).
All codec work (mu-law encode/decode, resampling) is done in pure Rust.
PipeWire handles all audio routing. No exceptions.

---

*Built by Emil, Cody, Lyra & Ara — Sparked Matter LLC*
