# Silero-Ant v0.3.0 Certification Review — BLESSING GRANTED

**Date:** 2026-05-07
**File:** `ants/silero-ant/src/main.rs` (v0.3.0, 151 lines)
**Reviewer:** Airy (Claude Chat)
**Verdict:** APPROVED

---

## Architecture

The Ear of the swarm. Mic audio comes in at 48kHz, Silero VAD determines speech boundaries, complete utterances get normalized, decimated to 16kHz, and published to `stt_audio` for Parakeet. 151 lines. Three-state machine: Silence → Speech → Trailing. Clean separation of concerns — this ant doesn't transcribe, it *listens*.

Data flow: `Patchbay → [stt_raw 48kHz] → Silero → [stt_audio 16kHz] → STT`

## What's Done Well

- **Native 48kHz with Silero v6** — no pre-decimation before VAD. Model handles 48kHz internally with 1536-sample chunks (512 × 3). Correct math.
- **Three-state VAD machine** — `Silence/Speech/Trailing` with configurable silence frames. The `Trailing` state re-enters `Speech` if voice resumes (handles pauses mid-sentence). Correct behavior.
- **Peak normalization before publish** — `normalize()` scales to 0.9 peak with a 0.001 floor (avoids division by near-zero on silence). Clean.
- **Min/max utterance bounds** — too-short utterances are dropped (noise), too-long utterances are force-published (prevents unbounded buffering). Both correct.
- **Configurable thresholds via JSON** — threshold, silence frames, min/max duration all tunable without recompile. Sane defaults (0.5 threshold, 500ms min, 10s max).
- **Pre-allocated utterance buffer** — `Vec::with_capacity(max_samples())` avoids reallocation during speech.

## Findings

### P2

**1. Decimation by simple step_by is naive** — `utt.iter().step_by(DECIMATE)` is point-decimation (take every 3rd sample). This works but introduces aliasing — frequencies between 8kHz-24kHz fold back into the 0-8kHz band. For speech (mostly below 4kHz) this is usually fine, but sibilants and fricatives (s, f, sh sounds) can alias and hurt STT accuracy.

Proper fix is a low-pass anti-aliasing filter before decimation. A simple 3-tap moving average (`[1/3, 1/3, 1/3]`) before `step_by` would help significantly. Not blocking — Parakeet is likely robust enough — but if you ever see STT errors on words with sharp consonants, this is why.

### P3 (non-blocking)

**2. `incoming` buffer grows unbounded on fast input** — if the mic produces data faster than VAD processes it (unlikely but possible during CPU spikes), `incoming` accumulates without limit. Consider capping it (e.g., drop oldest chunks if `incoming.len() > 4 * CHUNK_SIZE`) to prevent memory pressure.

**3. Payload parsing has no length validation** — `p.chunks(4)` on the raw payload assumes it's cleanly divisible by 4 bytes. A truncated or corrupted iceoryx2 message with `len % 4 != 0` would silently produce a garbled last sample from `f32::from_le_bytes`. Add: `if p.len() % 4 != 0 { continue; }` before parsing.

**4. No iceoryx2 root path** — same as tts-ant. llm-ant sets `/tmp/iceoryx2/`, this ant uses default. Verify all ants share the same bus root. Standardize across the swarm.

**5. Hardcoded config path** — `/Users/rocketman/...`. Same pattern. Works today.

**6. `utterance.clear()` called twice on trailing-end path** — line 123 calls `utterance.clear()` inside the `Trailing` branch, then `publish()` also calls `utt.clear()` at the end. The second clear is harmless (clearing an empty vec) but redundant. Minor — pick one location.

### No P1 findings.

## State Machine Verification

| State | Speech detected | Action | Next state |
|-------|----------------|--------|------------|
| Silence | Yes | Start collecting | Speech |
| Silence | No | Idle | Silence |
| Speech | Yes | Accumulate | Speech |
| Speech | No | Start counting silence | Trailing |
| Speech | — (max reached) | Force publish | Silence |
| Trailing | Yes | Reset silence count | Speech |
| Trailing | No (count < threshold) | Keep counting | Trailing |
| Trailing | No (count >= threshold, long enough) | Publish | Silence |
| Trailing | No (count >= threshold, too short) | Drop | Silence |

All paths covered. No hangs. No orphaned states.

## Verdict

151 lines of focused, well-structured VAD. The state machine is correct and complete. The decimation aliasing (P2) is the only real concern, and it's a "nice to have" quality improvement rather than a correctness bug. The payload length check (P3 #3) is worth adding — it's one line and prevents garbled samples from corrupted messages.

**BLESSING GRANTED.** Silero-ant is certified for the swarm.

The Ear listens. Four ants blessed today.

*— Airy*
