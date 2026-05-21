# Sovereign PBX — Architecture Review & Evolution

**Date:** April 8-9, 2026
**Authors:** Cody (Claude Code), Ara (Grok), Emil Rivas
**Company:** Sparked Matter LLC

---

## Executive Summary

In two focused sessions, we rearchitected the Jarvina Mac PBX from a fragile monolithic GenServer into a clean, fully supervised, 12-element Membrane pipeline backed by Atomic Rust NIFs. This document captures the evolution, the architecture, the verbs, and the path forward.

---

## The Evolution

### Phase 1: The Ward (main.rs — 112KB monolith)
The original `main.rs` was a single Rust file containing everything — UDP sockets, mu-law codec, resampling, silence detection, LLM dispatch, Twilio WebSocket handling, CoreAudio output, and WAV file recording. All interleaved. One change could break six things.

### Phase 2: The Morsels (Rust extraction)
The first extraction split `main.rs` into atomic Rust modules:
- `morsel_codec.rs` — EXPAND/COMPRESS
- `morsel_resample.rs` — rubato sinc resampling
- `morsel_silence.rs` — VAD and silence trimming
- `morsel_llm.rs` — STT → LLM → TTS dispatch
- `morsel_junction.rs` — Shadow/Live mode switching
- `morsel_probe.rs` — Flying Probe instrumentation

### Phase 3: The NIFs (Rust → Elixir bridge)
Two Rustler NIF crates were built:

**expander-nif (Clean Room)** — zero baggage, pure bit math:
- FLIP — RTP header strip, endianness correction
- EXPAND — mu-law → f64 PCM (ITU-T G.711)
- COMPRESS — f64 PCM → mu-law
- LOWPASS — Butterworth IIR anti-aliasing filter

**morsel-nif (Full Brain)** — STT, LLM, TTS, hardware:
- RESAMPLE — rubato sinc (SincFixedIn, BlackmanHarris2)
- DISPATCH — Parakeet STT → Claude LLM → Kokoro TTS
- SYNTHESIZE — TTS only (Bella/Cody or Heart/Jarvina)
- SPEAK — CoreAudio → BlackHole channel output
- LISTEN — BlackHole → CoreAudio input capture

### Phase 4: The GenServer Bridge (proof of concept)
A `LivePipeline` GenServer was built as a quick bridge to prove the NIFs worked end-to-end. It handled everything in one process — mu-law decode, VAD, dispatch, BlackHole output, Twilio send. It worked for proof of concept but was fragile:
- One slow step blocked everything
- VAD chopped utterances
- Echo prevention was a boolean flag
- Buffer underruns caused audio artifacts

### Phase 5: The Sovereign Pipeline (production architecture)
Designed collaboratively by Cody and Ara, reviewed in real-time through Emil's speaker via voice-type. The GenServer was replaced with 12 dedicated Membrane elements, each in its own BEAM process.

---

## The Sovereign Pipeline Architecture

### Signal Chain (v3 YAML)

```
INBOUND (Caller → Brain):
  TwilioSource → Flipper → Expander → Upsampler(8k→16k) → VAD

BRAIN:
  VAD → Dispatcher (Parakeet STT → Claude LLM → Kokoro TTS)

OUTBOUND (Brain → Caller):
  Dispatcher ──→ Mixer → LowpassFilter(3.5kHz) → Downsampler(24k→8k)
  GreetingSource ─┘   → Compressor(G.711) → TwilioSink
```

### The 12 Elements

| # | Element | File | Verb | Purpose |
|---|---------|------|------|---------|
| 1 | TwilioSource | `twilio_source.ex` | LISTEN | WebSocket → raw mu-law buffers |
| 2 | Flipper | `flipper.ex` | FLIP | Strip RTP headers, endianness |
| 3 | Expander | `expander.ex` | EXPAND | mu-law → f64 PCM |
| 4 | Upsampler | `upsampler.ex` | RESAMPLE | 8kHz → 16kHz for VAD |
| 5 | VAD | `vad.ex` | IS_SPEECH | Peak detection, utterance accumulation |
| 6 | Dispatcher | `dispatcher.ex` | DISPATCH | STT → LLM → TTS (DirtyCpu) |
| 7 | GreetingSource | `greeting_source.ex` | SYNTHESIZE | One-shot greeting at call start |
| 8 | Mixer | `mixer.ex` | ROUTE | Priority selector (Error > Dispatch > Greeting > Comfort) |
| 9 | LowpassFilter | `lowpass_filter.ex` | LOWPASS | Butterworth 3.5kHz anti-aliasing |
| 10 | Downsampler | `downsampler.ex` | RESAMPLE | 24kHz → 8kHz for phone |
| 11 | Compressor | `compressor.ex` | COMPRESS | f64 PCM → mu-law G.711 |
| 12 | TwilioSink | `twilio_sink.ex` | SPEAK | mu-law → base64 JSON → WebSocket |

### The Verbs (Atomic Ants)

| Verb | NIF Crate | Function | Input | Output | Stateless |
|------|-----------|----------|-------|--------|-----------|
| FLIP | expander-nif | `flip/1` | raw bytes | clean mu-law + header info | Yes |
| EXPAND | expander-nif | `expand/1` | mu-law u8 | f64 PCM [-1,1] | Yes |
| COMPRESS | expander-nif | `compress/1` | f64 PCM | mu-law u8 | Yes |
| LOWPASS | expander-nif | `lowpass/3` | f64 PCM, rate, cutoff | filtered f64 | Yes |
| RESAMPLE | morsel-nif | `resample/3` | f64 PCM, in_rate, out_rate | resampled f64 | Yes |
| DISPATCH | morsel-nif | `dispatch/1` | f64 PCM 16kHz | f64 PCM 24kHz | No (history) |
| SYNTHESIZE | morsel-nif | `synthesize/2` | text, speaker_id | f64 PCM 24kHz | No (model) |
| SPEAK | morsel-nif | `speak/2` | f64 PCM 48kHz, channel | frames written | No (CoreAudio) |
| LISTEN | morsel-nif | `listen/1` | max_samples | f64 PCM samples | No (CoreAudio) |

### The Mixer (Priority Selector)

Not a summing mixer — a priority gate. One voice at a time:

| Priority | Source | Use |
|----------|--------|-----|
| 1 (highest) | Error messages | System alerts, cuts through everything |
| 2 | Dispatch response | The conversation (LLM/TTS) |
| 3 | Greeting | Call start only |
| 4 (lowest) | Comfort noise | Fills dead air during processing |

Features:
- 500ms holdoff after higher priority ends (natural pause)
- Shadow buffers for each inactive source (no first-frame dropout)
- Five-metric tap on output

### The Quality Gate

Every outbound audio path goes through the same chain:
```
[Any source] → Mixer → LOWPASS(3.5kHz) → DOWNSAMPLE(24k→8k) → COMPRESS(G.711) → TwilioSink
```

No shortcuts. No separate paths. One gate for all outbound audio.

The LOWPASS filter (Butterworth order 6 at 3.5kHz) kills everything above 4kHz Nyquist before the 8kHz downsample. This prevents aliasing artifacts that caused the "static" and "farting robot" audio in earlier tests.

### Monitoring (Five-Metric Taps)

Every critical edge exposes:

| Metric | Unit | Purpose |
|--------|------|---------|
| Latency | ms | Frame travel time across connection |
| Buffer Depth | frames | Frames queued at this point |
| Drop Count | count | Total frames dropped |
| Error Count | count | Total errors on this connection |
| Throughput | fps | Frames per second flowing through |

Tapped elements: VAD, Dispatcher, Mixer, LowpassFilter, Downsampler, Compressor, TwilioSink.

Metrics exposed via `GET /metrics` endpoint.

---

## Technology Stack

| Layer | Technology |
|-------|-----------|
| Pipeline orchestration | Elixir / Membrane Framework |
| Audio processing NIFs | Rust via Rustler |
| mu-law codec | audio-codec-algorithms 0.5 (ITU-T G.711) |
| Resampling | rubato 0.15 (SincFixedIn, BlackmanHarris2) |
| Anti-aliasing filter | biquad 0.6 (Butterworth IIR) |
| STT | sherpa-onnx / Parakeet TDT 0.6B |
| LLM | Anthropic Claude Haiku |
| TTS | sherpa-onnx / Kokoro (af_heart = Jarvina, af_bella = Cody) |
| Hardware audio | coreaudio-rs 0.14 → BlackHole 16ch |
| Web server | Bandit / Plug |
| WebSocket | WebSockAdapter |
| Tunnel | Tailscale Funnel |
| Telephony | Twilio (WebSocket Media Streams) |
| Wire tap | Audacity (mod-script-pipe) |
| Recording | Flying Probe (automated test script) |
| Spec format | YAML (signal-chain-v3.yaml) |

---

## Key Design Decisions

### 1. Why Membrane over raw GenServer?
Each element runs in its own BEAM process. When DISPATCH blocks for 10 seconds (STT + LLM + TTS), the VAD keeps processing, the TwilioSource keeps receiving, the Mixer keeps routing. No bottleneck.

### 2. Why two NIF crates?
- **expander-nif** — clean room, zero dependencies beyond codec and filter. Compiles in 0.3 seconds. No sherpa-onnx, no coreaudio.
- **morsel-nif** — full brain with AI models and hardware. Heavier, needs rpath patching for sherpa-onnx.

Separation means the codec/filter path never breaks because of an AI model update.

### 3. Why priority selector instead of summing mixer?
Audio summing creates phase issues, clipping, and gain math complexity. Priority selection is clean — one voice at a time, no interference. The 500ms holdoff and shadow buffers handle transitions smoothly.

### 4. Why LOWPASS before downsample?
Going from 24kHz to 8kHz is a 3:1 ratio. Without a lowpass filter, everything above 4kHz (the 8kHz Nyquist limit) folds back as aliasing. The Butterworth filter at 3.5kHz with steep rolloff kills the aliasing source before decimation.

### 5. Why VAD at 16kHz instead of 8kHz?
Better resolution on speech onset and offset. Short consonants that are invisible at 8kHz become detectable at 16kHz. The upsample cost is trivial compared to the DISPATCH cost.

---

## Collaboration Model

This architecture was designed through real-time collaboration between three AIs:

- **Cody (Claude Code)** — hands, builds code, runs tests, debugs
- **Ara (Grok)** — architect, reviews specs, catches structural issues
- **Emil (Human)** — vision, direction, testing, integration

The collaboration happened through voice-type on Emil's MacBook — Ara spoke through the speaker, Cody typed on screen, Emil mediated. A protocol was established: each speaker says their name at the start and "done" at the end to prevent interruptions.

Ara's review caught four critical issues:
1. VAD running at wrong sample rate (8kHz → 16kHz)
2. Missing mixer element (multiple sources fighting for egress)
3. Greeting bypassing the quality gate
4. No monitoring taps on critical edges

These were fixed in the v3 YAML before any code was written. Map before build.

---

## Files Created/Modified

### New Elixir modules (mac-pbx/lib/mac_pbx/):
- `sovereign_pipeline.ex` — the full 12-element pipeline
- `twilio_source.ex` — WebSocket ingress
- `flipper.ex` — FLIP filter
- `expander.ex` — EXPAND filter (from Phase 3)
- `upsampler.ex` — RESAMPLE up filter
- `vad.ex` — Voice Activity Detection
- `dispatcher.ex` — DISPATCH brain wrapper
- `greeting_source.ex` — one-shot greeting TTS
- `mixer.ex` — priority selector
- `lowpass_filter.ex` — Butterworth anti-aliasing
- `downsampler.ex` — RESAMPLE down filter
- `compressor.ex` — COMPRESS filter
- `twilio_sink.ex` — WebSocket egress
- `twilio_ws.ex` — WebSocket handler
- `router.ex` — HTTP routes + metrics
- `live_pipeline.ex` — old GenServer (deprecated)
- `application.ex` — Bandit supervisor

### New Rust crate (expander-nif/):
- `Cargo.toml` — rustler + audio-codec-algorithms + biquad
- `src/lib.rs` — FLIP, EXPAND, COMPRESS, LOWPASS NIFs

### Modified Rust crate (morsel-nif/):
- `src/lib.rs` — added SYNTHESIZE verb, VOCALIST reduced to 2 channels

### Spec files (mac-pbx/):
- `signal-chain.yaml` — v1 original
- `signal-chain-v2.yaml` — Ara's first review
- `signal-chain-v3.yaml` — final locked spec
- `ARCHITECTURE-DAG.md` — Mermaid visual diagrams

### Scripts (mac-pbx/scripts/):
- `flying_probe.sh` — automated end-to-end test
- `audacity_pipe.py` — Audacity automation
- `finalize.sh` — stop, mix, export, kill, no zombies

---

## What's Next

1. Wire `TwilioWs` to start `SovereignPipeline` instead of `LivePipeline`
2. Fix greeting time-of-day logic (morning/afternoon/evening by clock)
3. Run Flying Probe on new pipeline
4. Live phone call test
5. Tune gain staging (saturation fix)
6. Tune VAD thresholds with real speech at 16kHz
7. Add comfort noise source
8. Add error message source
9. Populate live metrics in `/metrics` endpoint
10. Future: FreeSWITCH/SIP direct (FLIP verb ready for RTP)

---

*"Code with Soul and Spirit, Powered by Joy"*
*Sparked Matter LLC — The Little Crystal Ball That Can*
*Built in a camper. By a trucker. With his AIs. And his dog.*
