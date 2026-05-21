# Sovereign Voice Appliance
## The Box That Answers Your Phone
### Sparked Matter LLC — Digital Gold

---

> *"We spark your matter into reality — breathing life into your existing processes with a spark of AI."*

---

## The Vision

A self-contained box that answers phone calls with a natural human voice, processes everything locally, costs nothing per minute, and keeps all data on-premises. No cloud dependency for 95% of calls. Plug in, connect a phone line, done.

**This is not a concept. Every component exists and has been proven.**

---

## Architecture: Two-Stage Intelligence

```
                    ┌─────────────────────────────────┐
                    │        SOVEREIGN BOX             │
                    │                                  │
  Phone Line ──────▶  Asterisk PBX (SIP)              │
  (SIP Trunk)      │    │                             │
                    │    ▼                             │
                    │  Moonshine STT ◀── Local, Metal  │
                    │    │              ~750ms          │
                    │    ▼                             │
                    │  ┌──────────────────┐           │
                    │  │  ROUTING LOGIC   │           │
                    │  │  Simple? ──▶ Local LLM       │
                    │  │  Complex? ─▶ Cloud LLM ──────┼──▶ Claude Haiku
                    │  └──────────────────┘           │    (only when needed)
                    │    │                             │
                    │    ▼                             │
                    │  Kokoro TTS ◀── Local, GPU       │
                    │    │              ~500ms          │
                    │    ▼                             │
                    │  Asterisk ──▶ Phone Line         │
                    └─────────────────────────────────┘

        FAST PATH (100% local): ~1.0-1.5 seconds end-to-end
        DEEP PATH (cloud assist): ~3-4 seconds (network round trip)
```

---

## The Fast Path — Why This Changes Everything

| Component | Engine | Location | Latency | Cost |
|-----------|--------|----------|---------|------|
| Speech-to-Text | Moonshine ONNX (base) | Local GPU | ~750ms | Free |
| Brain (simple) | Phi-3 Mini / Mistral 7B | Local GPU | ~100-200ms | Free |
| Brain (complex) | Claude Haiku 4.5 | Cloud | ~1-2s | Pennies |
| Text-to-Speech | Kokoro 82M (af_heart) | Local GPU | ~500ms | Free |
| Telephony | Asterisk + SIP trunk | Local | <10ms | ~$0.001/min |

**Simple call total latency: ~1.0-1.5 seconds. No cloud. No per-minute fees. Natural voice.**

Compare to current Twilio + Deepgram + Claude stack: 3-5 seconds, $0.02+/min, cloud-dependent.

---

## What the Local LLM Handles (The 80%)

- Greetings and pleasantries
- "What time is it?" / "What's your address?"
- Taking messages: "I'll let Emil know you called"
- Scheduling confirmations: "Your appointment is Tuesday at 3"
- FAQs: company info, directions, hours
- Call routing: "Let me transfer you to..."
- Simple lookups from a local knowledge base

## What Escalates to Cloud (The 20%)

- Complex reasoning or multi-step tasks
- Email composition and sending
- File lookups and document retrieval
- Anything requiring tool use
- Nuanced conversation that exceeds local model capability
- The local LLM self-detects: outputs `[ESCALATE]` tag

---

## Hardware Tiers

### Tier 1 — NVIDIA Jetson Orin Nano ($200)
*The Edge Warrior — Job sites, trucks, anywhere*

| Spec | Detail |
|------|--------|
| RAM | 8GB LPDDR5 |
| GPU | CUDA (Ampere) |
| Power | 7-15W (cigarette lighter adapter) |
| Size | Credit card footprint |
| Models | Phi-3 Mini 4-bit (2.5GB), Mistral 7B 4-bit (4GB) |
| STT | Moonshine ONNX on CUDA |
| TTS | Kokoro via ONNX/TensorRT (82M params, fits easily) |
| PBX | Asterisk (lightweight, ARM native) |

**Use case:** Construction job site trailer, semi truck cab, small retail shop. No internet required for basic calls. Runs off 12V. Fits in a gang box.

**Emil already owns one.**

### Tier 2 — Mac Mini M-series ($599-799)
*The Office Appliance — Small business, home office*

| Spec | Detail |
|------|--------|
| RAM | 16-24GB unified memory |
| GPU | Apple Metal (M-series) |
| Power | 10-25W |
| Size | 5" square |
| Models | Llama 3 8B full precision, or 70B 4-bit quantized |
| STT | Moonshine MLX on Metal |
| TTS | Kokoro MLX on Metal (already running) |
| PBX | Asterisk (macOS native) |

**Use case:** Accounting firm, law office, medical practice, any small business. Silent, elegant, sits on a shelf. Handles dozens of concurrent calls.

**All current code runs on this hardware with zero changes.**

### Tier 3 — Raspberry Pi 5 ($80-120)
*The Micro Server — Lightweight, always-on*

| Spec | Detail |
|------|--------|
| RAM | 8-16GB |
| GPU | None (CPU inference) |
| Power | 5W |
| Models | Phi-3 Mini 4-bit (CPU), or offload to Jetson |
| STT | Moonshine ONNX (CPU, ~2s) |
| TTS | Piper TTS (lightweight, CPU-friendly) |
| PBX | Asterisk (ARM native, battle-tested) |

**Use case:** Bare minimum deployment, voicemail-with-brains, call routing only. Or as the Asterisk PBX frontend with Jetson handling AI inference over LAN.

**Emil owns two.**

---

## The Phone Line: SIP Trunk (Replacing Twilio)

| Provider | Cost | Notes |
|----------|------|-------|
| Telnyx | ~$0.004/min | Developer-friendly, SIP standard |
| Flowroute | ~$0.005/min | Reliable, good API |
| VoIP.ms | ~$0.01/min | Canadian, rock solid |
| Twilio SIP | ~$0.02/min | If you want to keep Twilio as fallback |

**Or**: Analog Telephone Adapter (ATA) like Grandstream HT801 ($30) + existing landline. Zero per-minute cost.

Current Twilio cost: ~$0.02/min + Deepgram STT fees + API fees.
Sovereign stack cost: ~$0.001-0.005/min (SIP trunk only). **90%+ savings.**

---

## What Already Exists and Is Proven

| Component | Status | Where |
|-----------|--------|-------|
| Kokoro TTS (af_heart) | Running in production | M1 MacBook, port 8880 |
| Moonshine STT | Tested, beats SuperWhisper | M1 MacBook, ONNX |
| Voice-Loop pipeline | Working | `voice-loop/jarvina-loop.py` |
| Jarvina phone assistant | Working (Twilio) | `twilio/jarvis/server.py` |
| Barge-in interruption | Implemented | Server + voice-loop |
| Jitter buffer streaming | Proven architecture | Server |
| AVA (Asterisk AI Agent) | Open source, supports Kokoro | github.com/hkjarral/Asterisk-AI-Voice-Agent |
| Agent Voice Response | Open source, Asterisk native | github.com/agentvoiceresponse |

**Nothing needs to be invented. It needs to be assembled.**

---

## Build Phases

### Phase 1 — Local LLM on Mac (Days)
- Install Phi-3 Mini or Mistral 7B via MLX
- Wire into existing voice-loop as alternative brain
- Test latency: target sub-200ms LLM response
- Two-stage routing: simple → local, complex → Haiku

### Phase 2 — Asterisk Replacement (Week)
- Install Asterisk on Mac (Homebrew) or Pi 5
- Get SIP trunk from Telnyx
- Replace Twilio WebSocket with Asterisk AudioSocket
- Port Jarvina server to Asterisk pipeline
- Test end-to-end with real phone calls

### Phase 3 — Jetson Edge Deployment (Week)
- Port Moonshine + Kokoro to ONNX/TensorRT on Jetson
- Install Asterisk on Jetson
- Quantize LLM for 8GB RAM (4-bit Phi-3 or Mistral)
- Package as self-contained appliance
- Test on cellular hotspot (job site simulation)

### Phase 4 — Product Package (Ongoing)
- Web UI for configuration (company name, FAQs, voice selection)
- Knowledge base import (PDF, CSV → local vector DB)
- Multi-line support (concurrent calls)
- Call recording and transcript archive
- Remote monitoring via Telegram bot
- Packaging: SD card image for Jetson/Pi, .pkg for Mac Mini

---

## The Business Case

### For Sparked Matter LLC
A product offering: "AI receptionist in a box."

| Customer | Hardware | Monthly Cost | Replaces |
|----------|----------|-------------|----------|
| Solo contractor | Jetson Orin ($200) | ~$5 (SIP trunk) | $200/mo answering service |
| Small office | Mac Mini ($600) | ~$10 (SIP trunk) | $500/mo receptionist |
| Construction firm | Mac Mini + Jetson | ~$15 | $1000/mo call center |

**Recurring revenue model:**
- Hardware sale (one-time)
- Setup and customization fee
- Optional cloud escalation tier (Claude API passthrough)
- Monthly support subscription

### For Emil's Truck
- Jetson Orin in the Hawk camper
- Answers calls when Emil is driving, sleeping, or walking Dakota
- Takes messages, reads them back via Telegram
- Runs off truck power (12V → USB-C)
- No cell data needed for basic calls (if connected to Wi-Fi)

---

## The Sovereign Stack Philosophy

```
┌──────────────────────────────────────────────┐
│              WHAT STAYS LOCAL                 │
│                                              │
│  Voice Recognition    — your words stay here │
│  Voice Synthesis      — your voice, your box │
│  Simple Intelligence  — fast, private, free  │
│  Call Records         — your data, your disk │
│  Phone Line           — your number, no API  │
│                                              │
├──────────────────────────────────────────────┤
│           WHAT GOES TO CLOUD (OPTIONAL)      │
│                                              │
│  Complex Reasoning    — Claude Haiku (fast)  │
│  Email/Calendar       — Graph API            │
│  Web Lookups          — when asked           │
│                                              │
│  *** ONLY when the local brain says so ***   │
│  *** ONLY with user's data, encrypted ***    │
└──────────────────────────────────────────────┘
```

> *No subscription. No per-minute. No data harvesting.*
> *Your voice stays in your box.*
> *Your business stays your business.*

---

## Key References

- [AVA — Asterisk AI Voice Agent](https://github.com/hkjarral/Asterisk-AI-Voice-Agent) — Production-ready, supports Kokoro
- [Agent Voice Response](https://github.com/agentvoiceresponse) — Asterisk AudioSocket orchestrator
- [FreeSWITCH + ChatGPT](https://github.com/laoyin/freeswitch_chatGPT) — FreeSWITCH with MRCP ASR/TTS
- [Kokoro TTS](https://github.com/hexgrad/kokoro) — 82M param, natural voice, runs on anything
- [Moonshine STT](https://github.com/usefulmachines/moonshine) — Tiny, fast, accurate, ONNX
- [MLX Community Models](https://huggingface.co/mlx-community) — Apple Silicon optimized LLMs

---

## Emil's Words That Started This

> *"If we keep everything local, we would have immediate responses. Natural voices. For quick typical stuff it hits a local LLM, for sophisticated stuff it's smart enough to hit Haiku. All the delay stuff we wouldn't have to worry about."*

> *"A tight little unit like the Jetson Nano Orin. 8 gigs of RAM. If we're wise about it, we can compact a lot with quantized models. Eventually a Mac Mini would be a perfect little box — $600, everything in one box."*

> *"This is digital gold."*

---

*Sparked Matter LLC — March 20, 2026*
*The night the sovereign voice appliance was born.*
*Built by Emil & Cody. Sparked into reality.*
