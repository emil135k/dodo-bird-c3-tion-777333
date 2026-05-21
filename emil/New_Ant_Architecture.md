# New Atomic Ant Architecture — Sovereign Voice Pipeline

**Date:** 2026-04-12
**Status:** STT + TTS verified on Apple Neural Engine via FluidAudio CoreML
**Replaces:** morsel-nif monolith with sherpa-onnx

---

## Old Architecture: morsel-nif Monolith

| Function | What | Engine | Problem |
|----------|------|--------|---------|
| `dispatch()` | STT → LLM → TTS all in one call | sherpa-onnx (CPU) | One failure kills all three; can't replace STT without touching TTS |
| `synthesize()` | TTS only | sherpa-onnx Kokoro | Coupled to same sherpa-onnx singleton |
| `expand/compress` | G.711 codec | Pure math | Fine — no change needed |
| `resample/process/flush` | 8k↔16k↔24k | rubato (Rust) | Fine — independent |
| `is_speech/trim_silence` | Energy-based VAD | Pure math | Fine — independent |
| `speak/listen` | BlackHole audio I/O | CoreAudio | Fine — independent |

**Single dependency:** `sherpa-onnx = "1.12"` — ONNX Runtime, int8 quantized models, CoreML attempts that silently fall back to CPU. One rodeo clown doing everything.

---

## New Architecture: Atomic Ants on Neural Engine

### ParakeetAnt (STT)

| Attribute | Value |
|-----------|-------|
| **Engine** | FluidAudio CoreML → Apple Neural Engine |
| **Model** | Parakeet TDT 0.6B v3 (full precision) |
| **Speed** | 89–130ms for 5s audio |
| **Input** | WAV file path (16kHz mono) |
| **Output** | Text string |
| **Bridge** | Rust → Swift FFI (`@_cdecl`) → FluidAudio SDK → CoreML → ANE |
| **Init** | 0.2s cached / 30s first-run (CoreML compilation) |
| **Dependency** | `fluidaudio-rs` (local patched) |

### KokoroAnt (TTS)

| Attribute | Value |
|-----------|-------|
| **Engine** | FluidAudio CoreML → Apple Neural Engine |
| **Model** | Kokoro 82M (CoreML optimized) |
| **Speed** | ~2s for 5s of speech |
| **Input** | Text string + voice name + speed |
| **Output** | WAV file (24kHz PCM) |
| **Voices** | Runtime switchable by string — no recompilation |
| **Bridge** | Same as STT — Rust → Swift FFI → CoreML → ANE |
| **Init** | ~8s cached / ~14min first-run (model download) |
| **Dependency** | Same `fluidaudio-rs` crate |

### Voice Cast (Runtime Switchable)

| Character | Voice ID | Role |
|-----------|----------|------|
| **Jarvina** | `af_heart` | Primary assistant (default) |
| **Cody** | `af_heart` or `am_adam` | Code narrator |
| **Lyra** | `af_bella` | Dream Queen |
| **Available** | `af_alloy`, `af_aoede`, `af_jessica`, `af_kore`, `af_nicole`, `af_nova`, `af_river`, `af_sarah`, `af_sky` | Female voices |
| **Available** | `am_adam`, `am_echo`, `am_eric`, `am_fenrir`, `am_liam`, `am_michael`, `am_onyx`, `am_puck`, `am_santa` | Male voices |

### ClaudeAnt (LLM) — Unchanged

| Attribute | Value |
|-----------|-------|
| **Engine** | Anthropic API (HTTP) |
| **Model** | Claude |
| **Input** | Text + conversation history |
| **Output** | Text reply |
| **Change** | None — stays exactly as-is |

### Codec & Signal Ants — Unchanged

| Ant | Function | Engine | Change |
|-----|----------|--------|--------|
| **FlipperAnt** | RTP endianness | Pure math | None |
| **ExpanderAnt** | μ-law → PCM | Pure math (G.711) | None |
| **CompressorAnt** | PCM → μ-law | Pure math (G.711) | None |
| **ResamplerAnt** | 8k↔16k↔24k | rubato (Rust) | None |
| **LowpassAnt** | Anti-alias filter | Butterworth (Rust) | None |
| **VADAnt** | Utterance detection | Peak energy (Elixir) | None |
| **MixerAnt** | Source mixing | Membrane (Elixir) | None |

---

## Side-by-Side Comparison

| | Old Monolith | New Atomic Ants |
|---|---|---|
| **STT engine** | sherpa-onnx Parakeet int8 (ONNX, CPU) | FluidAudio Parakeet (CoreML, Neural Engine) |
| **STT speed** | ~1200ms | **89–130ms** |
| **STT accuracy** | "Hey Darvana" (J phoneme mangled) | **"Hey Jarvana" (perfect)** |
| **TTS engine** | sherpa-onnx Kokoro (ONNX, CPU) | FluidAudio Kokoro (CoreML, Neural Engine) |
| **TTS speed** | ~3–4s | **~2s** |
| **Voice switching** | speaker_id integer, limited | String name, 20+ voices, runtime swap |
| **Hardware** | CPU only (CoreML silently failed) | **Apple Neural Engine (dedicated silicon)** |
| **Dependencies** | sherpa-onnx + ONNX Runtime (C++) | fluidaudio-rs + Swift bridge (static linked) |
| **Replaceability** | All-or-nothing monolith | Each ant independently replaceable |
| **Python in runtime** | No | No |
| **ONNX in runtime** | Yes (evicted) | **No** |
| **NIF signature change** | N/A | **None** — same `dispatch/1`, same `synthesize/2` |
| **Elixir code change** | N/A | **None** |

---

## The New Signal Chain

```
Twilio 8kHz μ-law
  → FlipperAnt        (endianness)
  → ExpanderAnt       (μ-law → PCM f64)
  → ResamplerAnt      (8k → 16k)
  → VADAnt            (utterance detection, pure Elixir)
  ┌─────────────────────────────────────────────┐
  │  → ParakeetAnt    (STT, 89ms, Neural Engine)│  ← NEW
  │  → ClaudeAnt      (LLM, Anthropic API)      │  ← unchanged
  │  → KokoroAnt      (TTS, 2s, Neural Engine)  │  ← NEW
  └─────────────────────────────────────────────┘
  → MixerAnt          (source mixing)
  → LowpassAnt        (anti-alias 4kHz)
  → ResamplerAnt      (24k → 8k)
  → CompressorAnt     (PCM → μ-law)
  → Twilio 8kHz μ-law
```

Each ant is its own BEAM process (Membrane element). Each can be probed, replaced, or bypassed independently.

---

## Replacement Map (morsel-nif/src/lib.rs)

| Lines | Old (sherpa-onnx) | New (FluidAudio) |
|-------|-------------------|-------------------|
| 288–327 | STT singleton init (`OfflineRecognizer`) | `fluidaudio_initialize_asr()` |
| 329–354 | TTS singleton init (`OfflineTts`) | `fluidaudio_initialize_tts("af_heart")` |
| 361–378 | STT transcribe (`stream.accept_waveform`) | `fluidaudio_transcribe_file(path)` |
| 422–433 | TTS synthesize (`tts.generate_with_config`) | `fluidaudio_synthesize(text, voice, speed, path)` |
| 436–451 | `synthesize()` NIF function | Same pattern with FluidAudio TTS |

**Everything else stays untouched.** Resampler, codec, silence, VAD, BlackHole — all independent of sherpa-onnx.

---

## Key Files

| File | Purpose |
|------|---------|
| `spikes/fluidaudio-spike/src/main.rs` | STT + TTS spike binary (verified) |
| `spikes/fluidaudio-spike/.cargo/config.toml` | rpath fix + espeak-ng linker flags |
| `spikes/fluidaudio-spike/fluidaudio-rs-local/swift/FluidAudioBridge.swift` | Patched bridge: deadlock fix + TTS extension |
| `spikes/fluidaudio-spike/fluidaudio-rs-local/Package.swift` | FluidAudio + FluidAudioTTS dependencies |
| `~/Library/Application Support/FluidAudio/Models/` | Cached CoreML models (permanent) |

---

## Three Bugs Fixed to Get Here

| Bug | Root Cause | Fix |
|-----|-----------|-----|
| **Swift SIGSEGV** | Duplicate Swift runtime symbols (system + Xcode toolchain) | `.cargo/config.toml` rpath → `/usr/lib/swift` only |
| **Concurrency deadlock** | `Task + DispatchSemaphore` with no Swift executor | `Thread + RunLoop + Task.detached` bootstrap |
| **Static audio output** | `synthesize()` returns WAV Data, not raw PCM floats | Write Data directly to file, don't double-wrap |

---

## Remaining Work

1. **ESpeakNG.framework bundling** — OOV words (numbers, foreign names) need espeak G2P
2. **Rustler NIF wrapper** — bridge fluidaudio-rs into Elixir
3. **Wire into Sovereign Pipeline** — replace sherpa-onnx singletons in morsel-nif
4. **Flying probe test** — full pipeline: audio → STT → LLM → TTS → audio
5. **Benchmark** — end-to-end latency with Claude LLM in the loop

---

*The sherpa-onnx rodeo clown is dead. Two Neural Engine ants took its place.*
*94ms ears. 2-second voice. Sovereign.*

*Code with Soul and Spirit, Powered by Joy.*
