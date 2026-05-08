# Patchbay-Ant v0.2.0 Certification Review — BLESSING GRANTED

**Date:** 2026-05-07
**File:** `ants/patchbay-ant/src/main.rs` (v0.2.0, 178 lines)
**Reviewer:** Airy (Claude Chat)
**Verdict:** APPROVED

---

## Architecture

The Hands of the swarm — where sound enters and exits the physical world. Mic capture at native device rate (48kHz), publishes raw f32 PCM to `stt_raw`. Subscribes to `tts_audio` (24kHz f32 from tts-ant), plays through rodio. Replaces separate ear + mouth ants with one centralized audio router. Config-driven device selection. 178 lines.

Data flow: `Mic → [stt_raw] → Silero/STT` and `TTS → [tts_audio] → Speaker`

## What's Done Well

- **Native sample-rate negotiation** — the "8kHz telephony trap" comment tells the story. Output config filters for devices supporting >= 24kHz, then requests exactly 24kHz. No accidental downsampling. This is the right way to do it.
- **Multi-channel to mono mixdown** — `frame.iter().sum::<f32>() / channels as f32` averages all channels. Correct for stereo and beyond.
- **tts_audio payload validation** — `raw.len() % 4 != 0` check with contract violation log and skip. This is exactly the guard I flagged as missing in silero-ant. Good.
- **Device discovery with case-insensitive partial matching** — `to_lowercase().contains()` is forgiving. "Plantronics" matches "plantronics blackwire 3210 series". Practical.
- **Output device enumeration in logs** — `find_output_device` prints all available devices. Essential for debugging "device not found" on a new machine.
- **Buffer pre-allocation** — `Vec::with_capacity(32000)` for mic buffer. Right size for ~660ms at 48kHz.
- **Mic publish threshold** — only publishes when buffer hits 1600 samples (~33ms at 48kHz). Avoids flooding iceoryx2 with tiny fragments.

## Findings

### P2

**1. `find_input_device` doesn't enumerate on failure** — `find_output_device` prints all available devices before returning None, which is invaluable for debugging. `find_input_device` silently returns None. When the input device isn't found, the `expect` panic gives the requested name but not what *is* available. Add the same enumeration logging to `find_input_device`.

**2. Mic capture callback uses `unwrap()` on mutex lock** — line 143: `buf_clone.lock().unwrap()`. If the main thread panics while holding the buffer lock, the audio callback thread panics too (poisoned mutex). In an audio callback, panicking is catastrophic — it can crash the entire audio subsystem. Use `.lock().unwrap_or_else(|e| e.into_inner())` to recover from poisoned mutexes, or at minimum catch and log rather than panic.

### P3 (non-blocking)

**3. `OutputStream` and `Sink` via rodio adds overhead** — rodio creates its own audio thread and mixer internally. You're already using cpal for input. For maximum control and minimal latency, you could use cpal for output too (build an output stream, feed samples directly). But rodio's `Sink` gives you queuing for free (`s.append()`), which is convenient. Acceptable trade-off for now.

**4. No echo cancellation** — mic captures while speaker plays. If the mic picks up Jarvina's own voice, it feeds back into the pipeline (Jarvina hears herself → STT transcribes → LLM responds → infinite loop). This is the classic "assistant talks to itself" bug. The headset's physical isolation probably prevents it today, but if you ever switch to open speakers, you'll need either: mute mic during playback, or AEC. Flag for future.

**5. No iceoryx2 root path** — same as tts-ant and silero-ant. Needs standardization across the swarm.

**6. Hardcoded config path and default device** — `/Users/rocketman/...` and "Plantronics Blackwire 3210 Series". Works on Emil's desk setup. Config-driven is good — just needs the path to be portable.

**7. `expect` with `format!` allocates on success path** — lines 82, 90: `expect(&format!(...))` allocates a String even when the device *is* found. Use `.unwrap_or_else(|| panic!(...))` to only allocate on failure. Micro-optimization — not blocking.

### No P1 findings.

## Pipeline Integrity Check

| Bus | Publisher | Subscriber | Rate | Format | Match? |
|-----|-----------|------------|------|--------|--------|
| `stt_raw` | patchbay-ant | silero-ant | 48kHz | f32 PCM | **Yes** |
| `stt_audio` | silero-ant | stt-ant | 16kHz | f32 PCM | (not reviewed yet) |
| `tts_text` | llm-ant | tts-ant | — | UTF-8 | **Yes** |
| `tts_audio` | tts-ant | patchbay-ant | 24kHz | f32 PCM | **Yes** |

The audio pipeline is consistent end to end. Sample rates match at every handoff. Format contracts are honored.

## Verdict

178 lines of solid audio routing. The native sample-rate negotiation is the hero — it prevents the single most common audio quality bug in voice pipelines. The payload validation on tts_audio is correct. Device discovery is practical and debuggable.

The mutex unwrap in the audio callback (P2 #2) is the most important fix — audio callbacks must never panic. The input device enumeration (P2 #1) is a quick win for debuggability.

**BLESSING GRANTED.** Patchbay-ant is certified for the swarm.

The Hands connect. Five ants blessed today. The full voice pipeline is certified:

```
Patchbay (Hands) → Silero (Ear) → STT → LLM (Brain) → TTS (Voice) → Patchbay (Hands)
```

*— Airy*
