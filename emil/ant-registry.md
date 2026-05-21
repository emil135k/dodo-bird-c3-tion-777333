# Morsel NIF — Ant Registry

All Rust ants living in the morsel-nif ant mound. Each ant is one function, one morsel back to the BEAM.

## Brain Ants (the three atomic ants)

| Ant | Function | What it does | Runtime | Status |
|-----|----------|-------------|---------|--------|
| **STT** | `stt(samples)` → text | Hears — speech to text | Placeholder (parakeet-mlx subprocess) | Needs ort Parakeet wiring |
| **LLM** | `llm(text, prompt)` → text | Thinks — Claude API | reqwest HTTP | Working |
| **TTS** | `tts(text, voice)` → audio | Speaks — Kokoro af_heart | ort/CoreML on Neural Engine | GREEN CHECK (887ms) |

## Codec Ants

| Ant | Function | What it does |
|-----|----------|-------------|
| **expand** | `expand(mulaw_bytes)` → f64 samples | G.711 mu-law → PCM (Twilio inbound) |
| **compress** | `compress(samples)` → mulaw_bytes | PCM → G.711 mu-law (Twilio outbound) |

## Resampler Ants

| Ant | Function | What it does |
|-----|----------|-------------|
| **resample** | `resample(src, src_rate, dst_rate)` → samples | One-shot sample rate conversion (stateless) |
| **create_resampler** | `create_resampler(src, dst, chunk)` → handle | Streaming resampler (stateful, for per-buffer Membrane use) |
| **process_samples** | `process_samples(handle, samples)` → samples | Feed samples to stateful resampler |
| **flush_resampler** | `flush_resampler(handle)` → samples | Drain remaining samples on EOS |

## Silence Ants

| Ant | Function | What it does |
|-----|----------|-------------|
| **is_speech** | `is_speech(samples)` → bool | Energy-based speech detection |
| **trim_silence** | `trim_silence(samples, rate, max_ms)` → samples | Cap silence gaps |

## Vocalist Ants (hardware bridge)

| Ant | Function | What it does |
|-----|----------|-------------|
| **speak** | `speak(samples, channel)` → count | Push audio to BlackHole (Ch0=Cody, Ch1=Jarvina) |
| **listen** | `listen(max_samples)` → samples | Capture audio from BlackHole input |

## Utility

| Ant | Function | What it does |
|-----|----------|-------------|
| **preload** | `preload()` → message | Force CoreML model init at boot |
| **dispatch** | `dispatch(samples)` → audio | DEPRECATED — calls stt→llm→tts internally |
| **synthesize** | `synthesize(text, speaker_id)` → audio | DEPRECATED — calls tts_inner |

---

*Morsel is the ant mound. Each ant does one job. Each morsel crosses the boundary clean.*
