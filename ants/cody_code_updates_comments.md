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

---

## 2026-05-02 — stt-ant Swift @main async fix

### Context
ChatGPT Vale reviewed stt-ant on dodo-bird and found the Swift init pattern still used
DispatchQueue + Task + semaphore blocking. She flagged it as the last certification blocker.

### Change
Replaced entire Swift worker init with `@main async struct ParakeetWorker`.
No semaphores, no dispatch queues, no blocking primitives.
Swift async executor handles everything naturally.

### Build
- `swift build -c release` — clean, 3.78s
- `cargo build --release` — clean, Rust side unchanged
- Both binaries installed to ~/.local/bin

### Review Status
- ChatGPT Vale: re-reviewed dodo-bird, all 4 findings FIXED, status CERTIFIABLE
- Codex Vale: pending re-review of @main async change
- Runtime test: pending

### Next Step
Live runtime test: start stt-ant with digi-ant + phone-silero-ant, inject test voice,
verify transcription flows through the full chain.

### Note
Both Vales are female voices/names (ChatGPT Vale and Codex Vale).
Vale was formerly ChatGPT, now also the name for the Codex CLI instance.

---

## 2026-05-02 — ChatGPT Vale ledger strategy note

### Context
Emil considered whether ChatGPT Vale observations should remain only in `chatgpt_vale_observations.md` or also be appended to Cody's running log.

### Decision
Append meaningful ChatGPT Vale review comments here as well when they materially affect Cody's implementation/certification flow.

`chatgpt_vale_observations.md` remains the independent reviewer/auditor ledger.
`cody_code_updates_comments.md` remains the implementation/build/runtime ledger, but can include ChatGPT Vale review checkpoints when they inform code status.

### Operating Rule
- Detailed ChatGPT review narrative → `chatgpt_vale_observations.md`
- Implementation changes, build results, runtime results → `cody_code_updates_comments.md`
- Cross-cutting certification checkpoints → append to both, with concise wording in Cody's log

### Current Certification Alignment
ChatGPT Vale has confirmed the latest `stt-ant` Swift worker uses true `@main async` initialization. The previous async init blocker is resolved.

Current status:

```text
Rust pipe fatality: FIXED
Swift sampleCount bound: FIXED
Empty/error markers: FIXED
Swift async init deadlock risk: FIXED
Build: clean per Cody
Runtime chain test: pending
Certification: CERTIFIABLE pending runtime validation
```

### Vale Note
Two ledgers are useful, but they should not become dueling diaries. Cody's log gets the operational checkpoint; ChatGPT Vale's log keeps the longer audit trail.
