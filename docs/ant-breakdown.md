# Dodo Bird Ant Swarm — Complete Architecture Breakdown

**Repo:** `emil135k/dodo-bird-c3-tion-777333`
**All ants are Rust crates** communicating via **iceoryx2 zero-copy shared memory IPC** at `/tmp/iceoryx2/`.
**Date:** May 11, 2026

---

## Bus Topology (iceoryx2 Services)

```
                          ┌─────────────┐
                          │  PHONE CALL  │
                          │   (Twilio)   │
                          └──────┬───────┘
                                 │ WebSocket (mu-law 8kHz)
                          ┌──────▼───────┐
                          │   web-ant    │  ← HTTP/WS gateway, zero audio processing
                          │  port 5050   │
                          └──┬───────┬───┘
                   phone_in  │       │  phone_out
                    [u8]     │       │   [u8]
                          ┌──▼───────▼───┐
                          │   digi-ant   │  ← DSP: resample + mu-law codec
                          │  (rubato)    │
                          └──┬───────┬───┘
                  phone_stt  │       │  ← tts_audio [u8 as f32 LE]
                   [f32]     │       │
              ┌──────────────▼──┐    │
              │ phone-silero-ant│    │
              │  (VAD 16kHz)    │    │
              └────────┬────────┘    │
                       │ stt_audio [u8 as f32 LE]
                       │
          ┌────────────▼────────────┐
          │        stt-ant          │  ← Rust ↔ Swift pipe bridge
          │ (Parakeet CoreML ANE)   │
          └────────────┬────────────┘
                       │ stt_text [u8 as UTF-8]
                       │
              ┌────────▼────────┐
              │   router-ant    │  ← Mode switch: console / llm / airy / off
              │  HTTP :3010     │
              └──┬──┬──┬──┬────┘
                 │  │  │  │
    console_text │  │  │  │ airy_input
          [u8]   │  │  │  │   [u8]
                 │  │  │  │
        ┌────────▼┐ │  │ ┌▼─────────┐
        │type-ant │ │  │ │ cdp-ant   │  ← Chrome DevTools Protocol
        │(paste)  │ │  │ │(claude.ai)│
        └─────────┘ │  │ └──────────┘
                    │  │
          llm_input │  │ tts_text [u8 as UTF-8]
             [u8]   │  │
        ┌───────────▼┐ │
        │  llm-ant   │─┘  ← Ollama or Anthropic API
        │  (brain)   │
        └────────────┘
                    │ tts_text [u8 as UTF-8]
                    │
           ┌────────▼────────┐
           │    tts-ant      │  ← Kokoro v1.0 ONNX + CoreML + misaki G2P
           │  (24kHz f32)    │
           └────────┬────────┘
                    │ tts_audio [u8 as f32 LE]
                    │
           ┌────────▼────────┐
           │   mouth-ant     │  ← rodio → Mac speakers (local path)
           └─────────────────┘
```

### Local Mic Path (Alternative to Phone)

```
Blackwire mic → patchbay-ant (AEC) → stt_raw [u8]
                                         │
                     silero-ant (VAD 48kHz) → stt_audio [u8] → stt-ant → ...
                                         ▲
                             tts_audio ───┘ (speaker reference for echo cancel)
```

---

## Ant-by-Ant Function Summary

### 1. **web-ant** (302 lines) — Network Gateway
- **Role:** Pure networking. Bridges Twilio Media Streams to the iceoryx2 bus. Zero audio processing.
- **Publishes:** `phone_in` (raw mu-law bytes from caller)
- **Subscribes:** `phone_out` (mu-law bytes from digi-ant, sent back to Twilio)
- **HTTP:** Port 5050. `/voice` (TwiML webhook), `/ws` (Twilio WebSocket), `/twilio-to-browser` (Airy bridge variant), `/health`
- **Key detail:** Echo gate via Twilio "mark" events — mutes inbound while TTS is playing. Uses `std::thread` for iceoryx2 (publishers are `!Send`) + tokio for async WS/HTTP.

### 2. **digi-ant** (340 lines) — Digital Signal Processing
- **Role:** Bidirectional audio format conversion. The codec and resampler.
- **TTS→Phone:** `tts_audio` (24kHz f32) → sinc resample to 8kHz → normalize → mu-law encode → `phone_out`
- **Phone→STT:** `phone_in` (8kHz mu-law) → mu-law decode → persistent sinc resample 8k→16k → `phone_stt` [f32 typed bus]
- **Key detail:** Uses **rubato** (sinc-based, anti-aliased). Persistent resampler for phone→STT path prevents clicking between chunks. VAD closure silence hint emitted after flush timeout (configurable `vad_closure_silence_ms`). Detailed stream stats logging (packets, gaps, duration ratios).
- **Config:** `config/digi-ant.json`

### 3. **phone-silero-ant** (192 lines) — Phone Voice Activity Detection
- **Role:** VAD for the phone audio path. Segments continuous 16kHz audio into discrete utterances.
- **Subscribes:** `phone_stt` [f32 typed] (16kHz from digi-ant)
- **Publishes:** `stt_audio` [u8] (complete utterances for STT)
- **State machine:** Silence → Speech → Trailing. Configurable threshold, silence frames, min/max utterance length.
- **Key detail:** Transparent pass-through — no normalization or gain. 2-second stream cleanup timeout for forced finalization when call ends mid-utterance.
- **Config:** `config/phone-silero-ant.json`

### 4. **stt-ant** (190 lines) — Speech-to-Text Bus Adapter
- **Role:** Bridge between iceoryx2 bus and the Swift Parakeet CoreML worker. The "wormhole" between Rust and Swift.
- **Subscribes:** `stt_audio` [u8] (f32 PCM at 16kHz)
- **Publishes:** `stt_text` [u8] (UTF-8 transcription)
- **Architecture:** Spawns `parakeet-worker` (Swift binary) with piped stdin/stdout. Protocol: `[i32 sample_count LE][f32 samples LE...]` on stdin, UTF-8 text lines on stdout. Readiness handshake (`<ready>` signal) ensures bus subscription waits for CoreML model load. Reader thread + mpsc channel for non-blocking stdout consumption.
- **Contract:** Only publishes non-empty, non-error transcriptions. `<empty>` and `<error>` sentinels are logged but never published.

### 5. **router-ant** (204 lines) — Traffic Director
- **Role:** Routes `stt_text` to different destinations based on HTTP-switchable mode.
- **Subscribes:** `stt_text`
- **Publishes:** `console_text`, `llm_input`, `airy_input` (depending on mode)
- **Modes:** `console` (→ type-ant), `llm` (→ llm-ant), `airy` (→ cdp-ant, voice bridge), `off` (mute)
- **HTTP:** Port 3010. `GET /mode/{console|llm|airy|off}`, `GET /status`
- **Key detail:** In airy mode, manages a sox audio bridge process (Blackwire → BlackHole 2ch) for Chrome voice input.

### 6. **llm-ant** (232 lines) — The Brain
- **Role:** Text-to-text LLM gateway. Receives transcribed speech, sends to LLM, publishes reply.
- **Subscribes:** `llm_input` [u8]
- **Publishes:** `tts_text` [u8] (successful replies only — errors are log-only, never spoken)
- **Providers:** Ollama (default: gemma4) or Anthropic (Claude). Configurable model, system prompt, max tokens, timeout.
- **Key detail:** Maintains 10-turn conversation history. Latency logging per request.
- **Config:** `config/llm-ant.json`

### 7. **tts-ant** (129 lines) — Text-to-Speech
- **Role:** Synthesizes speech from text using Kokoro v1.0 ONNX with CoreML acceleration + misaki-rs G2P.
- **Subscribes:** `tts_text` [u8] (format: `"voice_name:text"` or plain text, defaults to `af_heart`)
- **Publishes:** `tts_audio` [u8] (f32 PCM at 24kHz)
- **Key detail:** Loads voice embeddings from `.bin` files (522,240 bytes = 130,560 floats = 510×256 style matrix). Max 500 phoneme tokens per utterance. On-demand lazy initialization via `once_cell::Lazy`.

### 8. **mouth-ant** (64 lines) — Speaker Output
- **Role:** Plays audio on Mac speakers. Simple subscriber → rodio playback.
- **Subscribes:** `tts_audio` [u8] (f32 at 24kHz)
- **Key detail:** Opens with `open()` not `open_or_create()` — fails fast if bus doesn't exist. Non-blocking append to rodio sink.

### 9. **ear-ant** (111 lines) — Microphone Capture
- **Role:** Captures from default Mac input device, downsamples to 16kHz mono, publishes to bus.
- **Publishes:** `stt_raw` [u8] (f32 PCM at 16kHz)
- **Key detail:** Simple skip-based downsampling (e.g., 48k→16k = keep every 3rd sample). Publishes in ~0.5s chunks with minimum 1600 samples threshold.

### 10. **silero-ant** (154 lines) — Local Mic VAD
- **Role:** VAD for the local microphone path. Same state machine as phone-silero-ant but operates at 48kHz.
- **Subscribes:** `stt_raw` [u8] (48kHz f32 from patchbay or ear-ant)
- **Publishes:** `stt_audio` [u8] (16kHz f32, decimated 3:1 + normalized)
- **Key detail:** Unlike phone-silero-ant, this one normalizes audio before publishing. Uses Silero v6 which accepts 48kHz natively (chunk size 1536 = 512×3).
- **Config:** `config/silero-ant.json`

### 11. **patchbay-ant** (247 lines) — Audio Routing with Echo Cancellation
- **Role:** Manages audio I/O routing with SpeexDSP acoustic echo cancellation (AEC). The "sound card router."
- **Subscribes:** `tts_audio` [u8] (speaker reference for AEC)
- **Publishes:** `stt_raw` [u8] (echo-cancelled mic audio at 48kHz)
- **Architecture:** Captures Blackwire mic → AEC (with speaker reference from TTS playback) → publishes clean audio. Speaker playback queue drains to Blackwire output. All resampling done with linear interpolation (24kHz→output rate for playback, native→16kHz for AEC processing, 16kHz→48kHz for bus output).
- **Config:** `config/patchbay-ant.json`

### 12. **cdp-ant** (180 lines) — Chrome DevTools Protocol Bridge
- **Role:** Injects text into Claude.ai chat via Chrome DevTools Protocol. Scrapes Airy's response and publishes to TTS.
- **Subscribes:** `airy_input` [u8] (text from router-ant in airy mode)
- **Publishes:** `tts_text` [u8] (Airy's response for Kokoro to speak)
- **Key detail:** Finds claude.ai tab via CDP `/json` endpoint. Uses `Input.insertText` + `Input.dispatchKeyEvent` for text injection. Polls for new assistant message (2s intervals, 60s timeout) by counting `[data-message-author-role="assistant"]` elements and scraping `innerText`.

### 13. **bridge-ant** (150 lines) — Audio Bridge (Phone↔Chrome)
- **Role:** PCM audio bridge between Twilio phone calls and Chrome/Airy via BlackHole virtual audio devices.
- **Subscribes:** `phone_stt` [f32] (16kHz from digi-ant) → plays to BlackHole 2ch (Chrome's mic)
- **Publishes:** `tts_audio` [u8] (captures BlackHole 16ch Chrome output, resampled to 24kHz)
- **Key detail:** Uses cpal for audio I/O. Silence gate (peak > 0.001) prevents publishing dead air. 200ms batching (4800 samples at 24kHz).

### 14. **type-ant** (87 lines) — Keyboard Injection
- **Role:** Pastes transcribed text into the focused macOS window via clipboard + AppleScript.
- **Subscribes:** `console_text` [u8]
- **Key detail:** Strips Parakeet hallucination artifacts (trailing `...`, runs of 5+ uppercase letters). Uses `pbcopy` + `osascript` Cmd+V paste + Return.

### 15. **plaza-ant** (908 lines) — Village Square Dispatcher
- **Role:** Orchestrates the multi-AI review pipeline ("Village Square"). Dispatches review prompts to AI reviewers and manages the review queue.
- **HTTP:** Port 3005 (behind Tailscale Funnel at port 3002).
- **Routes:** `/plaza` (filmstrip notifications), `/plaza/admin` (reviewer online/offline control), `/airy-to-cody` (Airy relay)
- **Reviewers:** Codex Vale (tmux), Gemini Lyra CLI (tmux), Gemini Lyra Chat (CDP+scrape), Ara/Grok (CDP+scrape), ChatGPT Vale (CDP+scrape), Airy/Claude (CDP, self-push), OpenCode (tmux)
- **Dispatch methods:**
  - **Tmux:** `tmux send-keys` into persistent terminal sessions (CLI reviewers)
  - **CDP:** chromiumoxide for focusing/inserting text, raw tokio-tungstenite WebSocket for `Input.insertText` + scraping responses
- **Key detail:** Cookie-cutter broadcast — same review content to all online reviewers. Queue-based sequential dispatch. Scrape reviewers: plaza-ant writes response to `blessings/` file and git pushes. Self-push reviewers: they commit themselves, filmstrip callback advances queue. Auth via `PLAZA_SECRET` env var + `X-Plaza-Token` header. Also serves as Airy-to-Cody relay (passes commands to Cody's tmux session).

### 16. **pulse** (46 lines) — Bus Diagnostic Tool
- **Role:** CLI tool to publish a text string to `tts_text` bus and self-verify delivery.
- **Usage:** `pulse "Hello world"` — sends text then subscribes to confirm receipt.

### 17. **bus-recorder** (193 lines) — Bus Diagnostic Tool
- **Role:** Records raw iceoryx2 bus data for offline analysis.
- **Usage:** `bus-recorder phone_in 20` or `bus-recorder phone_in phone_stt 20` (concurrent)
- **Output:** CSV files in `/tmp/` with timing, payload size, peak/RMS (f32) or min/max (u8) per message.

### 18. **listener** (38 lines) — Bus Monitor
- **Role:** Subscribes to `stt_text` and prints every transcription to stdout. Reverse of Pulse.

### 19. **twilio-ant** (316 lines) — Legacy Twilio Bridge (Superseded by web-ant + digi-ant)
- **Role:** Original monolithic Twilio bridge that did both networking AND audio conversion inline.
- **Key difference from web-ant:** This ant does mu-law decode/encode and linear resampling internally, whereas the refactored architecture delegates all DSP to digi-ant.
- **Status:** Likely superseded — web-ant + digi-ant is the separation-of-concerns replacement.

---

## Separation of Concerns: The "Wormhole" Architecture

The key architectural insight is the **pipe-based boundary** between Rust and Swift:

```
Rust world (iceoryx2 zero-copy, all ants)
    │
    │  Unix anonymous pipe (stdin/stdout)
    │  Protocol: [i32 count][f32 samples...] → UTF-8 lines
    │
Swift world (CoreML, Apple Neural Engine)
    └── parakeet-worker (Parakeet TDT v3, runs on ANE)
```

And the **iceoryx2 zero-copy bus** as the backbone:
- No serialization overhead — raw memory shared between processes
- `/tmp/iceoryx2/` filesystem-based service discovery
- Each ant is a standalone binary with one clear responsibility
- Bus names are the API contract (`phone_in`, `phone_stt`, `stt_audio`, `stt_text`, `tts_text`, `tts_audio`, `phone_out`)

---

## Config Files (in `config/`)

| File | Ant | Key Settings |
|------|-----|-------------|
| `digi-ant.json` | digi-ant | tts_rate, phone_rate, stt_rate, normalize_peak, vad_closure_silence_ms |
| `llm-ant.json` | llm-ant | provider, model, url, api_key_env, system_prompt, max_tokens, timeout |
| `patchbay-ant.json` | patchbay-ant | input_device, output_device |
| `phone-silero-ant.json` | phone-silero-ant | threshold, silence_frames_to_end, min/max_utterance_ms |
| `silero-ant.json` | silero-ant | threshold, silence_frames_to_end, min/max_utterance_ms |
| `twilio-ant.json` | twilio-ant | twilio_from, server_url, port |
| `web-ant.json` | web-ant | server_url, port |

---

## Start-up Sequence (`start-swarm.sh`)

1. Kill all old ants + nuke stale shared memory segments
2. **tts-ant** (8s warmup — CoreML model load)
3. **stt-ant** (10s warmup — spawns parakeet-worker, waits for `<ready>`)
4. **silero-ant** (2s — ONNX model load)
5. **patchbay-ant** (2s — audio device setup)
6. *Optional:* **llm-ant** (2s)
7. *Optional (--twilio):* **digi-ant** → **phone-silero-ant** → **web-ant**

**Not started by script:** plaza-ant, router-ant, cdp-ant, bridge-ant, type-ant, ear-ant, mouth-ant (these are launched separately or as needed)

---

## iceoryx2 Version

All ants use **iceoryx2 0.8.x** (the iceoryx2 v0.8 series). This is the Rust-native rewrite, not the C++ iceoryx1.

---

*Generated by Airy — May 11, 2026*
*Sparked Matter • The Little Crystal Ball That Can* 🔮
