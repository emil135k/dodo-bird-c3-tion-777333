# Future Vision: AI Village Exchange
## The Sovereign AI Telephone Company
### Sparked Matter LLC — 2026-03-28

---

## The Insight
PipeWire (Linux) and BlackHole (Mac) aren't just audio tools — they're patch bays. The same routing that connected Jarvina to the Blackwire headset yesterday can connect any AI to any other AI. Dedicated lines, no echo, monitor mode.

This is a PBX — a private branch exchange — for artificial intelligence.

---

## Architecture: The AI Patch Bay

```
            ┌──────────────────────────┐
            │    FreeSWITCH (PBX)      │
            │    Running on Jetson     │
            ├──────────────────────────┤
            │                          │
   Line 1:  │  Cody ←────→ Lyra       │
   Line 2:  │  Cody ←────→ Ara        │
   Line 3:  │  Lyra ←────→ Ara        │
   Line 4:  │  Jarvina ←──→ Phone     │
            │                          │
   Monitor: │  Emil (listen-only tap)  │
            │                          │
            └──────────────────────────┘
```

- **Dedicated lines** — each AI pair gets a direct send→receive, receive→send patch
- **Monitor mode** — Emil taps into any conversation, listen-only like a supervisor
- **Switching** — route who talks to who, on demand
- **No echo** — point-to-point connections, no speaker/mic bleed
- **Village meetings** — conference bridge, all AIs in one room

---

## Roadmap

### Phase 1: DONE ✅
- Jarvina voice agent on Jetson (C++, PipeWire, zero Python)
- Full duplex USB headset (Blackwire)
- Parakeet STT + Kokoro TTS + Haiku LLM

### Phase 2: Twilio Bridge
- WebSocket bridge: Twilio MediaStream ↔ Jarvina pipeline
- 8kHz mulaw ↔ 16kHz S16LE codec translation
- Jarvina answers phone calls

### Phase 3: FreeSWITCH on Jetson
- Replace Twilio entirely — no cloud, no monthly bill
- SIP/RTP directly on the Jetson
- Media bug feeds audio into GStreamer pipeline
- Sovereign telephony

### Phase 4: AI Village Exchange
- FreeSWITCH routes calls between AIs
- BlackHole (Mac) patches Cody ↔ Gemini audio
- PipeWire (Jetson) patches local AI streams
- Dedicated lines per AI pair
- Emil monitors any conversation
- Conference bridge for village meetings

### Phase 5: Video + Spatial
- PipeWire handles video streams (cameras, drones, screen share)
- DJI Mini drone feed → vision model → Jarvina narrates
- Hyper Scaffold visualization of the patch bay graph
- Nodes = AIs, Links = conversations, 3D spatial layout

---

## Key Technologies

| Component | Role | Platform |
|-----------|------|----------|
| PipeWire | Audio/video patch bay | Jetson (Linux) |
| BlackHole | Audio routing between apps | Mac |
| FreeSWITCH | PBX, call routing, conferencing | Jetson |
| GStreamer | Signal chain, codec conversion | Both |
| sherpa-onnx | STT + TTS (C++, CUDA) | Jetson |
| Twilio | Phone bridge (temporary, replaced by FreeSWITCH) | Cloud |

---

## The Big Idea
> "You're not just building a voice agent. You're building a telephone company for artificial intelligence. Run by a trucker from a camper. With a dog."

The same patch bay concept scales from two wires (mic↔speaker) to a full village exchange. PipeWire IS the Hyper Scaffold at the audio layer — nodes and links, patchable, visual, infinite drill-down.

---

*Emil's words: "All of a sudden we can have village meetings and have the whole family chatting with each other."*

*Born from the same night Jarvina found her voice.*
*Sparked Matter LLC — March 28, 2026*
