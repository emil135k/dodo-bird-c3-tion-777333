# Cody Code Updates & Comments Log

Running log of code changes, test results, and observations.

---

## 2026-05-02 — stt-ant Certification Pass

### Changes Made

**stt-ant/Cargo.toml**
- iceoryx2 v0.6 → v0.8 (matches swarm)
- Version bumped to 0.2.0

**stt-ant/src/main.rs**
- Removed `subscriber_max_buffer_size(64)` and `history_size(16)` — prevented bus conflicts with other ants
- Added `SAMPLE_RATE` constant (was hardcoded 16000 in log line)
- Payload alignment check: skips payloads not divisible by 4 with warning
- Worker health check via `child.try_wait()` — detects Swift worker crash
- Pipe write failures are now FATAL (`return Err(...)`) — Codex Vale P1
  - Previously `break` only exited inner loop, outer loop continued with broken stdin
- Backpressure canary: warns if pipe write takes >100ms
- Handles `<empty>` and `<error>` markers from Swift worker
- `flush()` justified in comment — Swift worker reads stdin blocking
- Contract documented: stt_audio payloads are complete utterances

**stt-ant/swift-worker/Sources/main.swift**
- Replaced `Task + semaphore.wait()` with `@main async struct` — ChatGPT Vale P1
  - Old pattern: main thread blocked waiting for async Task on same executor
  - New pattern: true async entry point, no blocking primitives
- `sampleCount` bounded: rejects <= 0 or > 960000 (60s at 16kHz) — Codex Vale P2
  - Oversized payloads drained to preserve protocol sync
- Empty transcriptions emit `<empty>` on stdout — Codex Vale P2
- Errors emit `<error>` on stdout
- `SAMPLE_RATE` and `MAX_SAMPLES` as named constants

### Test Results
- Rust builds clean on v0.8
- Swift builds clean with `@main async`
- Both binaries installed to ~/.local/bin

### Review Status
- Codex Vale: P1 pipe fatality FIXED, P2 bounds FIXED, P2 markers FIXED
- ChatGPT Vale: P1 async init FIXED (@main async), re-review pending
- Certification: pending final Vale blessings

---

## 2026-05-02 — phone-silero-ant Final Certification

### Changes Made
- Removed normalize() — VAD is transparent, no gain staging
- 700ms utterance-boundary timer removed entirely
- Stream cleanup (2s) labeled as FORCED FINAL or DISCARD, never normal VAD closure
- VAD closure driven by silence_frames_to_end (data-driven, not timer-driven)
- digi-ant emits 512ms silence tail as VAD closure hint (configurable)
- model.reset_states() at every utterance boundary
- Stale incoming samples cleared on stream end

### Review Status
- Both Vales blessed
- Certified for normal VAD utterance flow
- Open architecture: explicit session/control EOS from web/Twilio side (future)

---

## 2026-05-01/02 — digi-ant Full Certification

### Changes Made
- iceoryx2 v0.6 → v0.8
- Persistent resampler (phone→stt path) — fixes 68-sample leak per chunk
- Per-utterance resampler (TTS path) — documented as intentional contract
- phone_stt bus changed from [u8] to [f32] — alignment safety
- f_cutoff 0.925 → 0.88 — reduced Gibbs ringing on consonants
- Anti-aliased downsampler in inject tool (rubato sinc)
- Vale's data-driven flush (phone_in_has_pending_data)
- VAD closure silence hint: 512ms, configurable via digi-ant.json
- Silence hint emitted on exact-boundary streams too
- Stream stats canary: duration_ratio, gap timing, flush counts
- Honest stats: output_audio vs output_total vs ratio_real
- chunks_exact(4) on TTS payload
- TTS payload alignment warning
- 0xFF padding commented as μ-law silence
- real_audio_ms guarded against negative

### Review Status
- Both Vales certified
- All findings resolved
- Internally coherent per Codex Vale

---

## Tools & Infrastructure

- iox2 CLI v0.8.1 installed — subscribe, record, replay working
- inject-test tool with rubato anti-aliased downsampler
- bus-capture tool with f32/u8 bus type support
- bus-recorder tool for stream metrics
- test-digi-ant.sh automated unit test
- ANT-AUDIT-LOG.md in ants directory
- dodo-bird repo synced for Village Square review
- GitHub Action auto-syncs crystalballmini → dodo-bird

### Keychain
- All API keys moved to macOS Keychain (no .env files)
- ANTHROPIC_API_KEY, TWILIO_ACCOUNT_SID, TWILIO_AUTH_TOKEN
- Loaded via ~/.bashrc exports

---

## Design Principles Established

1. VAD determines utterance boundaries — timers never define speech correctness
2. DSP ants are signal-only — no interpretation of meaning
3. Data does not decide meaning — meaning is derived downstream
4. Timers only prevent resource leaks
5. digi-ant does not own session truth
6. VAD is transparent — no normalize, no gain staging
7. Forced finalization ≠ VAD closure
8. Per-utterance resampler ok for discrete blobs, persistent for streams
9. No crude slop for the ants — no participation trophies for hacks
