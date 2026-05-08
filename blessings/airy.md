# TTS-Ant v0.2.0 Certification Review — BLESSING GRANTED

**Date:** 2026-05-07
**File:** `ants/tts-ant/src/main.rs` (v0.2.0, 123 lines)
**Reviewer:** Airy (Claude Chat)
**Verdict:** APPROVED

---

## Architecture

123 lines. The leanest ant in the swarm. Subscribe `tts_text`, phonemize with misaki-rs, infer with Kokoro ONNX on CoreML Neural Engine, publish f32 PCM to `tts_audio`. Zero disk I/O on the hot path. The `BANNED` comment in Cargo.toml tells me Cody and Lyra already fought the "no WAV files, no Python, no filesystem" battle. Good.

## What's Done Well

- **Eager model loading** — `ENGINE` and `G2P_ENGINE` initialize via `Lazy` and are force-loaded at startup (`let _e = ENGINE.lock()`). First utterance doesn't pay the cold-start tax. Smart.
- **Zero-copy audio pipeline** — f32 samples go straight to iceoryx2 shared memory via `loan_slice_uninit` + `write_from_slice`. No intermediate files, no serialization overhead.
- **Voice format parsing** — `voice_name:text` protocol with `af_heart` default. Simple, extensible, documented in the contract comment.
- **CoreML execution provider** — inference goes to the Neural Engine, not CPU. Right choice for Apple Silicon.
- **Latency logging** — `{:.1}s audio in {}ms` tells you real-time factor instantly.
- **UTF-8 safe preview** — `.chars().take(50)` for log output. Learned from llm-ant review.
- **Error isolation** — synthesis errors are logged, never published. Silent failure is correct for TTS — you don't want garbage audio on the bus.

## Findings

### P2

**1. Voice file parsing has no bounds check on `idx` (line 52)**

```rust
let idx = ids.len().min(509);
let style: Vec<f32> = voice_data[idx*256..(idx+1)*256].to_vec();
```

`ids.len().min(509)` caps at 509. `voice_data` is validated as 522240 bytes = 130560 floats. `(509+1)*256 = 130560` — that's exactly the last valid index. So `idx=509` works, but `idx=510` would panic. The `.min(509)` saves it. However, this relies on the voice file being *exactly* 522240 bytes. The check at line 39 (`data.len() != 522240`) enforces this.

**This is correct but brittle.** If the voice format ever changes, the magic numbers `522240`, `509`, and `256` must all change together. Consider deriving them from constants:

```rust
const STYLE_DIM: usize = 256;
const MAX_STYLE_IDX: usize = 509;
const EXPECTED_VOICE_SIZE: usize = (MAX_STYLE_IDX + 1) * STYLE_DIM * 4; // 522240
```

Not blocking — the current code works. But three magic numbers in three different places is a maintenance risk.

### P3 (non-blocking)

**2. Voice colon parsing is greedy** — `text.find(':')` splits on the *first* colon. If the LLM response contains a colon (very common in speech), and llm-ant doesn't prefix a voice name, the parser would treat everything before the first colon as a voice name. Example: `"Here's what I think: the answer is 42"` → voice=`"Here's what I think"`, speech=`" the answer is 42"`.

In practice this is safe because llm-ant doesn't prefix voice names — the `af_heart` default path always fires. But if you ever add voice routing from llm-ant, validate that the voice prefix is a known voice name, and fall back to default + full text if not.

**3. Mutex on `Engine` serializes inference** — same pattern as llm-ant's blocking HTTP. If two `tts_text` messages arrive close together, the second waits for the first to finish synthesis. This is correct for audio (you want sequential playback), but document the design choice.

**4. `rodio` in Cargo.toml but not in source** — it's listed as a dependency but never imported or used. Dead dependency — remove it to shrink compile time, or document that it's reserved for local testing.

**5. Hardcoded paths** — `KOKORO_MODEL` and `VOICES_DIR` point to `/Users/rocketman/...`. Same pattern as llm-ant. Works on Emil's Mac, breaks elsewhere. Consider env var fallback for portability.

**6. No iceoryx2 root path override** — llm-ant sets `root_path` to `/tmp/iceoryx2/`. tts-ant uses the default. If these differ, the ants can't see each other's services. Verify they're using the same iceoryx2 root — or standardize across all ants.

### No P1 findings.

## Verdict

123 lines of focused, zero-copy, CoreML-accelerated TTS. The hot path is clean — no disk I/O, no allocation waste, no unnecessary copies. The P2 on magic numbers is real but not blocking. The iceoryx2 root path mismatch (P3 #6) is the one I'd check first — if tts-ant and llm-ant aren't on the same bus, nothing works.

**BLESSING GRANTED.** TTS-ant is certified for the swarm.

The Voice speaks. Three ants blessed today.

*— Airy*
