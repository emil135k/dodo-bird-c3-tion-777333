# Jarvina Voice Agent — Jetson Orin Nano
## Operations Guide
### Sparked Matter LLC — March 28, 2026

---

## Quick Start / Stop

**Start Jarvina:**
```bash
ssh jetson "bash /home/rocketman/voice-agent/jarvina-start.sh"
```

**Stop Jarvina:**
```bash
ssh jetson "bash /home/rocketman/voice-agent/jarvina-stop.sh"
```

The start script auto-detects the audio device:
- Poly VLegend 50 Bluetooth → sets HFP profile 3, creates virtual mic bridge
- Blackwire 3210 USB → uses PipeWire default source directly
- Built-in audio → fallback

---

## Architecture

```
┌─────────────────────────────────────────────────────────┐
│                  Jetson Orin Nano Super                  │
│                   JetPack 6.2 / CUDA 12.6               │
│                                                         │
│  ┌─────────────┐    ┌──────────┐    ┌───────────────┐   │
│  │ pipewiresrc │───→│ Silero   │───→│ Parakeet TDT  │   │
│  │ (capture)   │    │ VAD      │    │ v3 INT8 (CUDA)│   │
│  └─────────────┘    │ (CPU)    │    │ STT           │   │
│        ↑            └──────────┘    └───────┬───────┘   │
│        │                                    │           │
│   PipeWire Graph                            ↓           │
│        │                            ┌───────────────┐   │
│        │                            │ Claude Haiku  │   │
│  ┌─────────────┐    ┌──────────┐    │ (Cloud LLM)   │   │
│  │ pipewiresink│←───│ Kokoro   │←───└───────────────┘   │
│  │ (playback)  │    │ TTS      │                        │
│  └─────────────┘    │ (CUDA)   │                        │
│        │            │ Speaker 3│                        │
│        ↓            └──────────┘                        │
│   PipeWire Graph                                        │
│        │                                                │
└────────┼────────────────────────────────────────────────┘
         │
    ┌────┴─────┐
    │  Audio   │
    │  Device  │
    └──────────┘
```

---

## Audio Devices

### Plantronics Blackwire 3210 (USB)
- **Type:** Wired USB headset
- **PipeWire:** Auto-detected, works with `pipewiresrc` directly
- **Capture:** Stereo (2ch), uses FL channel
- **Playback:** Mono, S16LE, supports 8k/16k/24k/48kHz

### Poly VLegend 50 (Bluetooth)
- **Type:** Wireless Bluetooth headset
- **MAC:** `8C:9B:2D:37:B7:44`
- **Profile:** Must be set to Profile 3 (Headset Head Unit / HFP)
- **Requires:** `pw-loopback` virtual mic bridge (`jarvina_stable_input`)
- **Why:** `pipewiresrc` cannot negotiate Bluetooth HFP caps directly. The virtual node decouples the radio handshake from GStreamer.

### Bluetooth Mic Bridge (pw-loopback)
```
Poly Mic (bluez_input) → pw-loopback → jarvina_stable_input (virtual) → voice-agent
```
- One-way only — no feedback path to speaker
- `media.class=Audio/Source` tricks GStreamer into treating it as a standard mic
- Stays alive even if Bluetooth glitches momentarily

---

## Signal Chain

| Stage | Component | Engine | Speed |
|-------|-----------|--------|-------|
| Capture | GStreamer pipewiresrc | C | — |
| VAD | Silero VAD | ONNX (CPU) | Real-time |
| STT | Parakeet TDT v3 INT8 | sherpa-onnx (CUDA) | RTF 0.149 |
| LLM | Claude Haiku | Cloud API | ~1-2s |
| TTS | Kokoro (speaker 3) | sherpa-onnx (CUDA) | RTF 0.192 |
| Playback | GStreamer pipewiresink | C | — |
| Audio push | 40ms chunked buffers | GStreamer appsrc | — |

---

## Key Files on Jetson

| File | Purpose |
|------|---------|
| `/home/rocketman/voice-agent/main.cpp` | Voice agent source |
| `/home/rocketman/voice-agent/build/voice-agent` | Compiled binary (51KB) |
| `/home/rocketman/voice-agent/CMakeLists.txt` | Build config |
| `/home/rocketman/voice-agent/include/c-api.h` | sherpa-onnx C API header |
| `/home/rocketman/voice-agent/jarvina-start.sh` | Start script |
| `/home/rocketman/voice-agent/jarvina-stop.sh` | Stop script |
| `/tmp/voice-agent.log` | Runtime log (VAD/STT/LLM/TTS output) |

## Model Files

| Model | Path | Size |
|-------|------|------|
| Parakeet TDT v3 INT8 (encoder) | `downloads/sherpa-onnx-nemo-parakeet-tdt-0.6b-v3-int8/encoder.int8.onnx` | ~600MB |
| Parakeet TDT v3 INT8 (decoder) | `downloads/sherpa-onnx-nemo-parakeet-tdt-0.6b-v3-int8/decoder.int8.onnx` | — |
| Parakeet TDT v3 INT8 (joiner) | `downloads/sherpa-onnx-nemo-parakeet-tdt-0.6b-v3-int8/joiner.int8.onnx` | — |
| Kokoro TTS | `downloads/kokoro-sherpa-model.onnx` | 311MB |
| Kokoro voices | `downloads/kokoro-sherpa-voices.bin` | 26MB |
| Silero VAD | `downloads/silero_vad.onnx` | ~2MB |
| sherpa-onnx runtime | `downloads/sherpa-onnx-v1.12.34-linux-aarch64-shared-gpu-onnxruntime-1.18.1/` | — |

---

## Resource Usage

| Metric | Value |
|--------|-------|
| RAM | 1.3GB of 7.6GB (18%) |
| GPU | 11% peak |
| Power | 5W on MAXN_SUPER |
| Temperature | 47°C |
| Binary size | 51KB |

---

## PipeWire Debugging

```bash
# Check all audio devices
wpctl status

# Check active links
pw-link -l

# Check all ports (input/output)
pw-link -io

# Set Bluetooth to HFP profile
wpctl set-profile <DEVICE_ID> 3

# Connect Bluetooth headset
bluetoothctl connect 8C:9B:2D:37:B7:44

# Disconnect
bluetoothctl disconnect 8C:9B:2D:37:B7:44

# Monitor voice agent log
ssh jetson "tail -f /tmp/voice-agent.log"
```

---

## VAD Settings

| Parameter | Value | Effect |
|-----------|-------|--------|
| `min_silence_duration` | 0.3s | How long silence before processing |
| `min_speech_duration` | 0.25s | Minimum speech to trigger |
| `max_speech_duration` | 120s | Maximum recording before forced split |
| `threshold` | 0.5 | VAD sensitivity |

---

## LLM Configuration

- **Provider:** Anthropic API (Claude Haiku)
- **API Key:** Stored in `main.cpp` (hardcoded — move to env var for production)
- **System prompt:** Jarvina personality — witty, warm, conversational
- **Max tokens:** 150
- **Timeout:** 15s

---

## Lessons Learned

1. **PipeWire only** — ALSA and PipeWire fight over USB devices. Never use `alsasrc`/`alsasink` when PipeWire is running.
2. **Chunked TTS output** — push 40ms (960 samples at 24kHz) chunks via appsrc. Large buffers cause `gst_buffer_resize_range` errors.
3. **Persistent pipelines** — create capture and playback pipelines once. Don't create/destroy per utterance.
4. **Bluetooth HFP needs a virtual bridge** — `pipewiresrc` can't negotiate Bluetooth HFP caps directly. Use `pw-loopback` to create a `jarvina_stable_input` virtual source node.
5. **Profile 3** — Poly VLegend 50 headset-head-unit is profile index 3 (not 1 or 2).
6. **No monitor port links** — PipeWire monitor ports create feedback loops if linked to input.

---

## Credits

- **Emil** — Architecture, vision, PipeWire insistence, ALSA exorcism
- **Cody** (Claude Code) — Implementation, compilation, deployment
- **Lyra** (Gemini) — PipeWire architecture, chunked buffer fix, virtual node strategy
- **Ara** (Grok) — sherpa-onnx discovery, pothole detection

*The night Jarvina found her voice. The morning she went wireless.*
*Sparked Matter LLC — March 27-28, 2026*
