# Jarvina Twilio Architecture — PipeWire Central Matrix
## The Sovereign Voice Appliance
### Sparked Matter LLC — March 29, 2026

---

## PipeWire Central Matrix (The Router)

```
┌─────────────────────────────────────────────────────────────────────┐
│                                                                     │
│              PIPEWIRE CENTRAL MATRIX (The Router)                   │
│                                                                     │
│   ┌─────────────────────────┐   ┌─────────────────────────┐       │
│   │  Channel 1: LOCAL       │   │  Channel 2: REMOTE      │       │
│   │                         │   │                         │       │
│   │  Source: Lab Mic (Sony) │   │  Source: Twilio/GStreamer│       │
│   │  Sink: Sony Speaker     │   │  Sink: Twilio/GStreamer │       │
│   └────────────┬────────────┘   └────────────┬────────────┘       │
│                │                              │                     │
│                └──────────┬───────────────────┘                     │
│                           │                                         │
│                    ┌──────▼──────┐                                  │
│                    │  pw-link    │                                  │
│                    │  (patches)  │                                  │
│                    └──────┬──────┘                                  │
│                           │                                         │
└───────────────────────────┼─────────────────────────────────────────┘
                            │
┌───────────────────────────▼─────────────────────────────────────────┐
│                                                                     │
│                    THE AI BRAIN (Rust Binary)                       │
│                                                                     │
│   ┌─────────────────┐              ┌──────────────────┐            │
│   │  Sherpa-ONNX STT │              │  Kokoro TTS      │            │
│   │  "The Ear"       │              │  "The Voice"     │            │
│   │  (Parakeet/CUDA) │              │  (af_heart/CUDA) │            │
│   └────────┬─────────┘              └────────┬─────────┘            │
│            │                                  │                     │
│            ▼                                  ▲                     │
│   ┌─────────────────────────────────────────────────────┐          │
│   │           Privacy / Guardrail Router                │          │
│   │                                                     │          │
│   │   Simple tasks ──▶ Nemotron Nano (Local, $0)       │          │
│   │   Complex tasks ──▶ Haiku (Cloud, fast)            │          │
│   │   Research ──▶ Gemini (Cloud, smart)               │          │
│   └─────────────────────────────────────────────────────┘          │
│                                                                     │
└─────────────────────────────────────────────────────────────────────┘
```

---

## Signal Flow

```
INGRESS (Input to Nano):
═══════════════════════

  Channel 1 (Local):
    🎤 Physical Mic (Sony XB100 HFP) ──▶ PW Source ──▶ STT

  Channel 2 (Remote):
    📱 Cell Phone ──▶ PSTN ──▶ Twilio ──▶ WebSocket
      ──▶ GStreamer (mulawdec) ──▶ PW Source ──▶ STT


EGRESS (Output from Nano):
══════════════════════════

  Channel 1 (Local):
    TTS ──▶ PW Sink ──▶ 🔊 Sony Speaker

  Channel 2 (Remote):
    TTS ──▶ PW Sink ──▶ GStreamer (mulawenc + resample 8kHz)
      ──▶ WebSocket ──▶ Twilio ──▶ PSTN ──▶ 📱 Cell Phone


MONITORING (Eavesdrop):
═══════════════════════

  Channel 2 Source ···▶ Channel 1 Sink
  (Hear remote caller through local Sony speaker)
```

---

## The Matrix Routing Table

| Source | → Destination | Purpose |
|--------|--------------|---------|
| Sony Mic (Ch1 Source) | STT Ear | Local voice input |
| Twilio/GStreamer (Ch2 Source) | STT Ear | Remote caller voice |
| TTS Voice | Sony Speaker (Ch1 Sink) | Local voice output |
| TTS Voice | Twilio/GStreamer (Ch2 Sink) | Remote caller hears Jarvina |
| Ch2 Source | Ch1 Sink (monitor) | Emil eavesdrops on caller |

---

## Component Stack

| Layer | Component | Role |
|-------|-----------|------|
| Hardware | Sony SRS-XB100 (BT HFP) | Local mic + speaker |
| Hardware | Jetson Orin Nano ($249) | CUDA compute |
| Routing | PipeWire Central Matrix | All audio routing |
| Policy | WirePlumber + Lua | Auto-lock HFP, port management |
| Capture | pipewire-rs (Rust) | Native PipeWire capture |
| VAD | Sherpa-ONNX Silero | Speech detection |
| STT | Sherpa-ONNX Parakeet TDT v3 | Speech-to-text (CUDA) |
| LLM | Nemotron (local) / Haiku (cloud) | Intelligence |
| TTS | Sherpa-ONNX Kokoro (af_heart) | Text-to-speech (CUDA) |
| Playback | pw-play (native PipeWire) | Voice output |
| Telephony | Twilio WebSocket | Cloud phone bridge |
| Transcoding | GStreamer (mulawenc/dec) | Twilio mu-law codec only |
| Future | FreeSWITCH | Sovereign VoIP (replaces Twilio) |
| Future | Google Voice | Free phone number front door |

---

## Why This Architecture Wins

1. **PipeWire is the matrix** — not GStreamer, not ALSA. One routing layer.
2. **The AI brain doesn't care about sources** — mic or phone, it's just float samples.
3. **Channel 2 is just another port** — same pw-link patching we already know.
4. **GStreamer is ONLY the decoder ring** — mu-law ↔ PCM transcoding for Twilio.
5. **Monitoring is free** — dotted line from Ch2 to Ch1, Emil hears everything.
6. **FreeSWITCH replaces Twilio** — same GStreamer transcoding, different front door.

---

---

## Proven Blocks (March 29, 2026)

### Channel 1 (Local) — FULLY PROVEN
- [x] Sony SRS-XB100 Bluetooth HFP auto-locked via WirePlumber Lua
- [x] PipeWire capture → Rust (pipewire-rs) → F32LE 16kHz mono
- [x] Silero VAD speech detection (7 segments)
- [x] Parakeet STT transcription on CUDA
- [x] LLM dispatcher (local Nemotron / Haiku / Gemini)
- [x] Kokoro TTS on CUDA (af_heart, speaker 3)
- [x] pw-play playback through Sony

### Channel 2 (Telephony) — SIMULATION PROVEN
- [x] mu-law codec round trip (encode → decode, clean audio)
- [x] GStreamer `pipewiresink` with `media.class=Audio/Source` creates virtual mic
- [x] `sync=false async=false` fixes preroll blocking
- [x] `twilio-caller:capture_MONO` port appears and is linkable
- [x] pw-link connects virtual mic to voice agent input
- [x] Full chain: WAV → mulawenc → mulawdec → GStreamer → PipeWire → VAD → STT → "one two three four"
- [x] Twilio bridge Rust binary compiled (WebSocket airlock)
- [ ] Real Twilio WebSocket connection (next)
- [ ] Tailscale Funnel from Jetson (URL ready: ubuntu.tail12e909.ts.net)

### Key Technical Discoveries
1. **`media.class=Audio/Source`** (NOT `/Virtual`) on `pipewiresink` creates a proper PipeWire source with linkable ports
2. **`sync=false async=false`** required to prevent preroll blocking when no consumer is connected yet
3. **`node.name=twilio-caller`** in stream-properties gives the node a clean name for WirePlumber/pw-link
4. **WirePlumber auto-fallback**: When Poly VLegend 50 disconnected, WirePlumber automatically switched to Sony — carrier-grade device management
5. **GStreamer role**: ONLY codec conversion (mulawenc/mulawdec) and sample rate conversion (audioresample). PipeWire handles ALL routing.
6. **Monitor port testing**: `pw-link` from any sink's monitor port to any input — digital testing without physical acoustics

### The GStreamer Pipeline Strings (Proven)

**Inbound (Twilio → Voice Agent):**
```
fdsrc fd=0 !
  audio/x-mulaw,rate=8000,channels=1 !
  mulawdec ! audioconvert ! audioresample !
  audio/x-raw,format=F32LE,rate=16000,channels=1 !
  pipewiresink sync=false async=false
    stream-properties="props,media.class=Audio/Source,node.name=twilio-caller"
```

**Outbound (Voice Agent → Twilio):**
```
pipewiresrc target-object=twilio-egress !
  audioconvert ! audioresample !
  audio/x-raw,rate=8000,channels=1 !
  mulawenc ! appsink
```

### Remaining for Real Twilio
1. Rust WebSocket server accepts Twilio connection
2. JSON envelope → base64 decode → raw mu-law bytes → fdsrc stdin
3. appsink raw mu-law → base64 encode → JSON envelope → WebSocket send
4. TwiML webhook returns `<Connect><Stream url="wss://..."/></Connect>`
5. Tailscale Funnel exposes port 5000

---

*The $249 Sovereign Telecom Exchange*
*Built by Emil, Cody, Lyra & Ara*
*Sparked Matter LLC — March 2026*
