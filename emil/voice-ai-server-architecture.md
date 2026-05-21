# Voice AI Server Architecture
## Compiled Signal Chain — The Right Way to Build It
### Sparked Matter LLC — 2026-03-27

---

## The Problem We Discovered

All the AI engines work individually:
- **Parakeet STT** — clean transcription on CUDA
- **Nemotron LLM** — 15.7 tok/s on Jetson, 25.6 tok/s on Mac
- **Kokoro ONNX TTS** — RTF 0.19, perfect WAV output on CUDA

**The glue between them is the problem.** Python passing audio bytes between components causes crackling, stuttering, and buffer underruns. When we dump Kokoro output to a file and play it back — perfect. When we pipe it live through Python — crackles and stammers.

**Root cause:** Python is too slow for real-time audio piping. No deterministic timing. No proper buffer management. Garbage collection pauses. It's a garden hose connecting Formula 1 engines.

---

## The Solution: GStreamer — Compiled Signal Chain

GStreamer is a C-based compiled media pipeline framework. Handles format conversion, buffering, routing, and timing natively. Industry standard for embedded audio/video. Cross-platform.

**Key advantages over Python audio piping:**
- Compiled C — no interpreter overhead, no GC pauses
- Built-in buffer management with configurable queue depths
- Native support for sample rate conversion, format transcoding
- Bluetooth (BlueZ), ALSA, PulseAudio, PipeWire plugins
- Zero-copy where possible
- Runs on Mac AND Jetson — same pipeline code

---

## Target Architecture

| Layer | Jetson (NVIDIA) | Mac (Apple) |
|-------|-----------------|-------------|
| **Audio server** | PipeWire | CoreAudio |
| **Signal chain** | GStreamer (C, compiled) | GStreamer (C, compiled) |
| **STT** | Parakeet (CUDA) | Parakeet (MLX) |
| **LLM** | Nemotron (CUDA, 15.7 tok/s) | Nemotron (Metal, 25.6 tok/s) |
| **TTS** | Kokoro ONNX (CUDA, RTF 0.19) | Kokoro (Metal) |
| **Bluetooth** | BlueZ + PipeWire | CoreAudio (native) |
| **Telephony** | FreeSWITCH / Asterisk | FreeSWITCH / Asterisk |

### Layer Responsibilities

**Audio Server (PipeWire / CoreAudio):**
- Switchboard — manages which app gets the mic, which gets the speaker
- Bluetooth device management and codec negotiation
- System-level audio routing
- Think: the patch panel in a rack

**Signal Chain (GStreamer):**
- Media processing pipeline between patch points
- Format conversion (24kHz Kokoro → 8kHz HFP, etc.)
- Buffer management with proper queue depths
- Timing — deterministic, no jitter
- Think: the signal processing gear between the patch points

**Python (Orchestration ONLY):**
- API calls to Ollama (LLM)
- Conversation history management
- Application logic (greeting, routing, tool use)
- NEVER touches audio bytes directly

---

## Signal Flow

```
  Bluetooth Mic (HFP 8/16kHz)
       │
       ▼
  PipeWire/CoreAudio (audio server)
       │
       ▼
  GStreamer Pipeline (compiled C)
       │
       ├── Resample → 16kHz mono WAV
       │
       ▼
  Parakeet STT (CUDA/MLX)
       │
       ▼ (text)
  Python Orchestrator
       │
       ├── Nemotron LLM (local, 15.7 tok/s)
       │   OR Claude Haiku (cloud, complex queries)
       │
       ▼ (text)
  GStreamer Pipeline (compiled C)
       │
       ├── Kokoro ONNX TTS → 24kHz WAV
       ├── Resample → 8/16kHz for HFP
       ├── Buffer (ensure complete before playback)
       │
       ▼
  PipeWire/CoreAudio (audio server)
       │
       ▼
  Bluetooth Speaker (HFP)
```

---

## Installation

**Mac:**
```bash
brew install gstreamer
```

**Jetson:**
```bash
sudo apt install gstreamer1.0-tools gstreamer1.0-plugins-base \
  gstreamer1.0-plugins-good gstreamer1.0-plugins-bad \
  gstreamer1.0-plugins-ugly gstreamer1.0-pulseaudio \
  gstreamer1.0-alsa
```

---

## Audio Server Migration

**Jetson: PulseAudio → PipeWire**
- PulseAudio causes crackling on Bluetooth HFP (timer-based scheduling, buffer underruns on ARM)
- PipeWire has native mSBC wideband support (16kHz vs 8kHz CVSD)
- PipeWire has adaptive buffering instead of timer-based
- NVIDIA's Jetson audio stack is ALSA at kernel level — doesn't care what sits on top
- Migration steps documented in memory/project_jetson_orin_nano.md

**Mac: CoreAudio (no change needed)**
- Already the native audio server
- Handles Bluetooth natively
- GStreamer has CoreAudio plugins

---

## Diagnostic Tools

| Tool | Layer | What It Sees |
|------|-------|-------------|
| **tshark / btmon** | HCI/Bluetooth | Packet timing, jitter, retransmissions |
| **pactl / pw-top** | Audio server | Buffer underruns, sample rates, latency |
| **sox --stat** | Audio file | Peak level, RMS, clipping, spectogram |
| **gst-launch** | Signal chain | Pipeline testing, format verification |
| **hcitool rssi** | Bluetooth | Signal strength, RF interference |

### Quick Diagnostic Commands
```bash
# Check for buffer underruns (PipeWire)
pw-top   # XRUN column should stay at 0

# Check Bluetooth signal strength
hcitool rssi XX:XX:XX:XX:XX:XX

# Capture Bluetooth packets for analysis
sudo btmon -w /tmp/bt-debug.snoop

# Generate spectrogram of audio output
sox output.wav -n spectrogram -o spectrogram.png

# Test GStreamer pipeline
gst-launch-1.0 audiotestsrc ! audioconvert ! pulsesink device=bluez_sink.XX
```

---

## Proven Benchmarks (2026-03-26)

| Component | Platform | Result |
|-----------|----------|--------|
| Kokoro ONNX TTS | Jetson CUDA | RTF 0.19 (5x real-time) ✅ |
| Kokoro TTS (PyTorch) | Jetson CUDA | RTF 1.29 (too slow) ❌ |
| Nemotron 3 Nano 4B | Jetson llama.cpp CUDA | 15.7 tok/s ✅ |
| Nemotron 3 Nano 4B | Mac M1 Ollama | 25.6 tok/s ✅ |
| Parakeet STT 110M | Jetson NeMo CUDA | 3.69s warmup, working ✅ |
| Kokoro WAV → file → playback | Jetson BT | Perfect audio ✅ |
| Kokoro live stream → BT | Jetson Python pipe | Crackling ❌ |

**Key finding:** Individual engines are fast enough. The Python audio piping layer is the bottleneck. GStreamer eliminates it.

---

## Related Resources

- [AVA — Asterisk AI Voice Agent](https://github.com/hkjarral/Asterisk-AI-Voice-Agent) — Production-ready, supports Kokoro
- [Agent Voice Response](https://github.com/agentvoiceresponse) — Asterisk AudioSocket orchestrator
- [GStreamer Documentation](https://gstreamer.freedesktop.org/documentation/)
- [Sovereign Voice Appliance](emil/sovereign-voice-appliance.md) — Full product vision
- [Jetson Orin Nano Setup](memory/project_jetson_orin_nano.md) — Hardware setup log

---

## Emil's Words

> "The engines are Formula 1. The plumbing between them is garden hose."

> "We need flash style coders-decoders-digital-transcribers."

> "It's a stuttering, a stammering, like bad timing in a digital sampling world."

---

*Sparked Matter LLC — March 27, 2026*
*The night the signal chain diagnosis was made.*
*Built by Emil & Cody.*
