# Architecture Audit — April 4, 2026
## Honest inventory of what exists, what runs, what doesn't

---

## JETSON SIDE

### Directory: `crystalballmini/jetson/`

```
jetson/
├── config/
│   ├── pipewire/                    (EMPTY — config is on Jetson system)
│   └── wireplumber/
│       ├── 51-sony-hfp-enable.lua   (loader script)
│       └── 51-sony-hfp-lock.lua     (forces Sony Bluetooth HFP profile)
├── scripts/
│   └── jarvina-launch.sh            (40 lines — starts both binaries)
├── twilio-bridge/
│   ├── Cargo.toml
│   └── src/main.rs                  (390 lines)
└── voice-agent-rust/
    ├── Cargo.toml
    └── src/main.rs                  (588 lines)
```

### Rust Crates

**voice-agent-rust/Cargo.toml:**
| Crate | Version | Purpose |
|-------|---------|---------|
| pipewire | 0.8 | Audio I/O — capture (mic) + playback (speaker) via PipeWire |
| sherpa-onnx | 1.12 (shared, CUDA) | VAD (Silero) + STT (Parakeet) + TTS (Kokoro) — ALL in-process |
| reqwest | 0.12 (blocking, json) | HTTP client for Claude/Gemini/Ollama LLM API |
| serde_json | 1 | JSON parsing |

**twilio-bridge/Cargo.toml:**
| Crate | Version | Purpose |
|-------|---------|---------|
| tokio | 1 (full) | Async runtime |
| tokio-tungstenite | 0.24 | WebSocket for Twilio Media Streams |
| futures-util | 0.3 | Async utilities |
| serde / serde_json | 1 | JSON serialization |
| base64 | 0.22 | Base64 encode/decode mulaw payloads |
| axum | 0.7 (ws) | HTTP server + WebSocket upgrade |
| tower | 0.4 | HTTP middleware |
| pipewire | 0.8 | Two PipeWire streams: jarvina-in (phone→agent) + jarvina-out (agent→phone) |
| chrono | 0.4 | Timestamps |

### Source Files

**voice-agent-rust/src/main.rs (588 lines):**
- PipeWire capture stream (16kHz F32LE mono) — reads from Sony Bluetooth mic
- PipeWire playback stream (16kHz F32LE mono, as SOURCE) — writes to Sony speaker + feeds twilio-bridge
- Silero VAD: in-process, CPU, 512-sample windows, threshold 0.3
- Parakeet STT: in-process, CUDA GPU, INT8 transducer model
- Kokoro TTS: in-process, CUDA GPU, speaker ID 3 (af_heart), streaming callback
- LLM: HTTP to Claude Haiku or local Ollama (configurable via JARVINA_LLM env var)
- Linear resampler: 24kHz TTS → 16kHz PipeWire
- Ring buffers with Arc<Mutex<>> for thread-safe audio passing

**twilio-bridge/src/main.rs (390 lines):**
- Axum HTTP server on port 5000
- `/voice` webhook returns TwiML with `<Connect><Stream>`
- `/ws` WebSocket handler: bidirectional mulaw 8kHz audio
- Mulaw encode/decode (pure Rust, G.711 standard)
- Upsample 8kHz→16kHz, downsample 16kHz→8kHz
- Two PipeWire streams: jarvina-in (SOURCE, pushes phone audio) + jarvina-out (SINK, captures agent voice)
- Pre-loaded greeting from `/tmp/jarvina-greeting.ul`
- Outbound sender: 160-byte chunks every 20ms

**/tmp/jarvina-pbx.rs (983 lines):**
- EXPERIMENTAL combined version — both voice agent + twilio bridge in ONE process
- Adds voice dialing ("call [number]"), spoken-to-digits parser
- Adds outbound Twilio REST API call initiation
- Adds Gemini LLM support
- Mode switching: LOCAL (Sony speaker) vs PHONE (Twilio)
- NOT deployed in production — in /tmp, not /home

### Model Files (on Jetson at /home/rocketman/downloads/)

| Model | File | Size | Used By |
|-------|------|------|---------|
| Silero VAD | silero_vad.onnx | ~2MB | voice-agent-rust (CPU) |
| Parakeet STT encoder | encoder.int8.onnx | ~600MB | voice-agent-rust (CUDA) |
| Parakeet STT decoder | decoder.int8.onnx | | voice-agent-rust (CUDA) |
| Parakeet STT joiner | joiner.int8.onnx | | voice-agent-rust (CUDA) |
| Parakeet tokens | tokens.txt | | voice-agent-rust |
| Kokoro TTS model | kokoro-sherpa-model.onnx | 311MB | voice-agent-rust (CUDA) |
| Kokoro TTS voices | kokoro-sherpa-voices.bin | 26MB | voice-agent-rust |
| Kokoro tokens | kokoro-tokens.txt | | voice-agent-rust |
| espeak data | espeak-ng-data/ | | voice-agent-rust (phonemes) |
| Kokoro lexicon | lexicon-us-en.txt | | voice-agent-rust |

### Scripts

**jetson/scripts/jarvina-launch.sh (40 lines):**
1. Kill existing voice-agent-rust + twilio-bridge
2. Set SHERPA_ONNX_LIB_DIR + LD_LIBRARY_PATH for CUDA
3. Set JARVINA_LLM=haiku, load API key from ~/.anthropic_key
4. Start voice-agent-rust (nohup, wait 12s for PipeWire ports)
5. Start twilio-bridge (nohup, wait 2s)
6. Show status (ps + wpctl)

### Tunnel

- Tailscale Funnel: `https://ubuntu.tail12e909.ts.net` → Jetson port 5000
- WebSocket: `wss://ubuntu.tail12e909.ts.net/ws`

### Python on Jetson

**NONE in the voice pipeline.** Everything is Rust + sherpa-onnx. No Python processes needed.

### Summary: Jetson is PURE RUST + CUDA

- VAD: in-process (Silero ONNX, CPU)
- STT: in-process (Parakeet INT8, CUDA)
- TTS: in-process (Kokoro ONNX, CUDA, streaming)
- Audio: PipeWire (pw_stream)
- Twilio: Rust WebSocket (axum + tokio-tungstenite)
- LLM: HTTP to cloud API (only external dependency)
- No Python. No HTTP to local services. No GC pauses.

---

## MACBOOK SIDE

### What is ACTUALLY RUNNING right now

```
PID 623   — Python server.py (port 3000) — HANDLES ALL TWILIO CALLS
PID 631   — Python mlx_audio.server (port 8880) — KOKORO TTS
PID 48721 — Python parakeet-server.py (port 8765) — PARAKEET STT

Rust PBX (mac-pbx) — NOT RUNNING
```

**The Mac voice system is THREE Python processes. The Rust PBX is not running.**

### Directory: `crystalballmini/mac-pbx/`

```
mac-pbx/
├── Cargo.toml
├── launch.sh                    (151 lines — orchestration script)
├── src/
│   └── main.rs                  (1469 lines)
├── test-audio/
│   ├── q1.wav, q1-8k.wav       (Kokoro test questions)
│   ├── q2.wav, q2-8k.wav
│   ├── q3.wav, q3-8k.wav
│   └── q4.wav, q4-8k.wav
├── test_blackhole.py            (Python test — BlackHole crisscross)
├── test_call.py                 (Python test — WebSocket injection)
└── test_caller.py               (Python test — VAD-based caller)
```

**Cargo.toml dependencies:**
| Crate | Version | Purpose |
|-------|---------|---------|
| cpal | 0.15 | Audio input (default mic) — DOES NOT WORK with BlackHole channels |
| coreaudio-rs | 0.14 | Audio output with channel mapping — WORKS for BlackHole Ch2 output |
| sherpa-onnx | 1.12 (shared) | VAD (Silero, CPU) + STT (Parakeet INT8, CPU) — in-process |
| reqwest | 0.12 (blocking, json) | HTTP to Kokoro TTS (port 8880) + Claude API |
| tokio | 1 (full) | Async runtime |
| tokio-tungstenite | 0.24 | WebSocket |
| axum | 0.7 (ws) | HTTP server + WebSocket |
| serde / serde_json / base64 / chrono | standard | Utilities |

### mac-pbx/src/main.rs (1469 lines) — THE RUST PBX

**What it does:**
- HTTP server on port 5050
- `/voice` webhook, `/ws` WebSocket for Twilio
- `/test-voice` + `/ws-test` for automated testing
- `/audio/:filename` serves test WAV files
- `/health` endpoint

**Audio I/O:**
- Input: cpal for MacBook mic, OR coreaudio-rs for BlackHole Ch1 (coreaudio-rs input DOES NOT WORK inside PBX process — works standalone)
- Output: cpal for MacBook speakers, OR coreaudio-rs for BlackHole Ch2 (WORKS — verified)

**VAD:** Silero via sherpa-onnx, in-process, CPU. Same config as Jetson but threshold=0.3, min_silence=0.8s, max_speech=15s.

**STT:** sherpa-onnx OfflineRecognizer in-process, CPU. Uses same Parakeet INT8 ONNX models as Jetson (downloaded to /Users/rocketman/downloads/).

**TTS:** HTTP POST to `http://localhost:8880/v1/audio/speech` — calls the PYTHON mlx_audio.server. NOT in-process. NOT Rust.

**LLM:** HTTP POST to Claude Haiku API. Same as Jetson.

**Mulaw codec:** Pure Rust, identical to Jetson (mulaw_encode/mulaw_decode).

**Resampling:** Pure Rust linear interpolation, identical to Jetson.

**Command parser:** spoken_to_digits(), parse_dial_command(), is_hangup_command() — identical to Jetson.

**Greeting:** Time-aware (morning/afternoon/evening), generated via Kokoro HTTP on call connect. Non-blocking (tokio::spawn).

**Echo prevention:** Speaking flag mutes VAD during TTS. Mark messages for precise timing.

**Twilio integration:** Mark-based echo prevention, dual-stream guard, outbound call via REST API.

### Directory: `crystalballmini/twilio/jarvis/`

```
twilio/jarvis/
├── server.py                    (1147 lines — ACTIVE, handles all calls)
├── parakeet-server.py           (291 lines — ACTIVE, STT service)
├── local_voice.py               (506 lines — NOT RUNNING, local mic mode)
├── latency_test.py              (290+ lines — instrumentation tool)
├── jarvina-channel.py           (7.3KB — optional event observer)
├── contacts.json                (contact directory)
├── .env                         (credentials)
└── audio/                       (cached TTS files, 97MB)
```

**server.py (1147 lines) — THE ACTUAL RUNNING CALL HANDLER:**
- FastAPI on port 3000
- `/voice` webhook + `/ws` WebSocket for Twilio
- ParakeetSTT class: WebSocket client to ws://localhost:8765
- Kokoro TTS: HTTP to http://localhost:8880
- Claude Haiku: streaming API with sentence-level TTS
- Barge-in detection
- Call control tokens: HANGUP, TRANSFER, INTRO, ADDCONTACT
- Caller profiles (Emil = trusted)
- JitterBuffer: 160-byte mulaw chunks at 20ms pace
- soxr VHQ resampling (24kHz→8kHz, anti-aliased)

**parakeet-server.py (291 lines) — ACTIVE STT SERVICE:**
- FastAPI + uvicorn on port 8765
- WebSocket /ws endpoint for streaming mulaw→text
- HTTP POST /transcribe for batch audio→text
- Uses parakeet_mlx Python package (MLX Metal GPU)
- Model: mlx-community/parakeet-tdt-0.6b-v3
- Warm inference: ~150-230ms
- Runs in mlx-env Python virtualenv

**local_voice.py (506 lines) — NOT RUNNING:**
- Local mic → WebRTC VAD → Parakeet STT → Claude → Kokoro TTS → speakers
- Uses sounddevice for audio
- Has spoken_to_digits parser
- Phase 2 feature, never integrated into active system

### Kokoro TTS Server (port 8880)

```
Process: /opt/homebrew/Cellar/python@3.12/.../Python -m mlx_audio.server --port 8880
Model: mlx-community/Kokoro-82M-bf16
Voice: af_heart
Output: WAV 24kHz 16-bit PCM
Latency: ~250-350ms warm
Engine: MLX (Metal GPU, M1 Pro)
```

**This is a PYTHON process. Both the Rust PBX and Python server.py call it via HTTP.**

### Parakeet STT Server (port 8765)

```
Process: /Users/rocketman/mlx-env/bin/python3 parakeet-server.py --port 8765
Model: mlx-community/parakeet-tdt-0.6b-v3
Engine: MLX (Metal GPU, M1 Pro)
WebSocket: ws://localhost:8765/ws (streaming mulaw)
HTTP: POST /transcribe (batch PCM)
Latency: ~150-230ms warm
```

**This is a PYTHON process. The Python server.py connects to it via WebSocket. The Rust PBX does NOT use it — it has sherpa-onnx in-process instead.**

### Model Files (on Mac at /Users/rocketman/downloads/)

| Model | File | Used By |
|-------|------|---------|
| Silero VAD | silero_vad.onnx (2.3MB) | Rust PBX (sherpa-onnx, CPU) |
| Parakeet STT | sherpa-onnx-nemo-parakeet-tdt-0.6b-v3-int8/ | Rust PBX (sherpa-onnx, CPU) |
| Parakeet MLX | mlx-community/parakeet-tdt-0.6b-v3 (HuggingFace cache) | Python parakeet-server.py (MLX) |
| Kokoro MLX | mlx-community/Kokoro-82M-bf16 (HuggingFace cache) | Python mlx_audio.server (MLX) |

**NOTE: Two separate Parakeet implementations exist:**
- sherpa-onnx ONNX models (for Rust PBX) — INT8, CPU
- parakeet_mlx MLX models (for Python server) — MLX, Metal GPU

### Tailscale Funnel

Currently pointing to port 5050 (Rust PBX). But Rust PBX is NOT running.
When Python server.py handles calls, funnel should point to port 3000.

### Twilio Phone Number Webhooks

| Number | Webhook | Points To |
|--------|---------|-----------|
| +18136076219 (Jarvina) | https://emils-macbook-pro.tail12e909.ts.net/voice | Mac PBX (whichever is on funnel port) |
| +17272185546 (Test Line) | https://handler.twilio.com/twiml/EH_hangup | Twilio hangup handler |

**NOTE: Webhook was previously pointing to Jetson (ubuntu.tail...). Fixed 2026-04-03 to point to Mac.**

### BlackHole 16ch

- Installed as HAL plugin: /Library/Audio/Plug-Ins/HAL/BlackHole16ch.driver
- 16 input + 16 output channels at 48kHz
- Channel isolation verified: Ch1→Ch1 loopback=0.3, Ch1→Ch2 crosstalk=0.0
- coreaudio-rs OUTPUT to Ch2: WORKS (channel map property 2002)
- coreaudio-rs INPUT from Ch1: DOES NOT WORK inside PBX process (works as standalone binary)
- cpal default input: DOES NOT route to specific BlackHole channels
- afplay/ffplay: DO NOT route to BlackHole Ch1
- sounddevice playrec with output_mapping/input_mapping: WORKS (PortAudio native channel map)
- ffmpeg avfoundation capture: WORKS for recording (captures default mix)
- Multi-Output Device: configured in Audio MIDI Setup but not actively used

---

## HONEST COMPARISON: JETSON vs MAC

| Component | Jetson | Mac (Running) | Mac (Rust, NOT running) |
|-----------|--------|---------------|------------------------|
| Language | Rust | **Python** | Rust |
| VAD | Silero in-process (CPU) | None (relies on Parakeet) | Silero in-process (CPU) |
| STT | Parakeet in-process (CUDA) | **Python parakeet_mlx (HTTP)** | Parakeet in-process (CPU) |
| TTS | Kokoro in-process (CUDA) | **Python mlx_audio (HTTP)** | **Python mlx_audio (HTTP)** |
| Audio I/O | PipeWire (pw_stream) | N/A (WebSocket only) | cpal + coreaudio-rs |
| Twilio | Rust WebSocket (axum) | **Python WebSocket (FastAPI)** | Rust WebSocket (axum) |
| LLM | HTTP to Claude API | HTTP to Claude API | HTTP to Claude API |
| GC pauses | None (Rust) | **Yes (Python GC)** | HTTP TTS still hits Python GC |
| Local mic | PipeWire Sony BT | Not active | cpal MacBook mic |

**The Mac "twin" is NOT a twin. Even the Rust version calls Python HTTP for TTS.**

---

## STRAY FILES

### Python test scripts in mac-pbx/
- test_call.py — WebSocket injection test (Python)
- test_caller.py — VAD-based test caller (Python)
- test_blackhole.py — BlackHole crisscross test (Python)

These were used during development. They are NOT part of the production system.

### Documentation in emil/
- pbx-architecture.md
- jarvina-jetson-services.md
- jarvina-architecture-overview.md
- pipewire-rs-tutorial.md
- jetson-rustdesk-notes.md
- disaster-recovery.md
- rust-voice-agent-api-map.md
- jarvina-bluetooth-hfp-discovery.md

---

*This document was generated honestly on April 4, 2026, after Emil requested a full audit of both architectures. No claims are made about functionality that has not been verified.*
