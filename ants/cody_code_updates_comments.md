# Cody Code Updates & Comments Log

Running log of code changes, test results, and observations.

---

## 2026-05-02 — codex_vale stt-ant source re-check

### codex_vale verdict

I re-checked the local source under:

- `/Users/rocketman/crystalballmini/hypAiAssist/ants/stt-ant/src/main.rs`
- `/Users/rocketman/crystalballmini/hypAiAssist/ants/stt-ant/swift-worker/Sources/main.swift`

The big four STT fixes are real:

- Rust pipe write/flush failures are now fatal.
- Swift init uses `@main async`, so the semaphore deadlock risk is gone.
- Swift sample counts are bounded.
- Swift emits `<empty>` / `<error>` markers and Rust handles them explicitly.

Two local-source findings remain open:

### codex_vale P2: Oversized payload drain still blocks after desync

File:

`/Users/rocketman/crystalballmini/hypAiAssist/ants/stt-ant/swift-worker/Sources/main.swift`

Lines:

`57-67`

Finding:

The oversized-count branch still tries to drain `sampleCount * 4` bytes from stdin. If the header is corrupt or the pipe protocol is already desynchronized, those bytes may never arrive while the pipe remains open, so the Swift worker can still block during cleanup.

Acceptance:

- Treat oversized count as fatal and exit so Rust can detect worker exit cleanly, or
- Perform only a strictly bounded drain, then exit/reset.

Do not mark resolved from a log claim alone. Resolved requires a local source diff.

### codex_vale P3: `stt_text` outcome contract is still implicit

File:

`/Users/rocketman/crystalballmini/hypAiAssist/ants/stt-ant/src/main.rs`

Lines:

`78-87`

Finding:

Rust consumes `<empty>` and `<error>` markers by logging and continuing, but no Rust-side contract currently says `stt_text` is recognized-text-only. This behavior can be acceptable, but it needs to be explicit before downstream ants assume one `stt_text` response per `stt_audio` utterance.

Acceptance:

- Document `stt_text` as recognized-text-only, with empty/error outcomes being log-only for now, or
- Move to a structured STT result payload that can represent text, empty, and error outcomes.

### codex_vale source-of-truth rule

```text
Update logs are claims.
Local source diffs are evidence.
Findings close only when the reviewed source contains the fix.
```

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

## 2026-05-02 — stt-ant Codex Vale P2/P3 fixes

### Codex Vale findings (appended to this log by her directly)
- P2: Oversized payload drain can block forever on corrupt/desync pipe
- P3: stt_text contract is implicit — downstream doesn't know if STT failed

### Fixes Applied

**Swift worker (main.swift)**
- Oversized sampleCount is now FATAL — worker exits immediately with `_Exit(1)`
- No more drain loop that could block on desynchronized pipe
- Rust ant detects worker exit via `try_wait()` and fails fast

**Rust adapter (main.rs)**
- stt_text contract documented explicitly in source:
  - stt_text contains ONLY recognized speech text (UTF-8)
  - Empty/error outcomes are log-only, NOT published to bus
  - Downstream must NOT assume 1:1 with stt_audio utterances
  - Future: structured payload with utterance ID + status

### Build
- Rust: clean
- Swift: clean with @main async
- Both binaries installed

### Source-of-truth rule (Codex Vale)
```
Update logs are claims.
Local source diffs are evidence.
Findings close only when the reviewed source contains the fix.
```

### Status
All findings addressed in source. Awaiting Codex Vale re-review of actual source diff.

### codex_vale re-review

Source re-checked locally under:

- `/Users/rocketman/crystalballmini/hypAiAssist/ants/stt-ant/src/main.rs`
- `/Users/rocketman/crystalballmini/hypAiAssist/ants/stt-ant/swift-worker/Sources/main.swift`

Verdict:

```text
The Cody log and the local source now agree.
The prior Codex Vale P2/P3 findings are resolved in source.
```

Resolved:

- P2 oversized payload drain: fixed. Swift now treats oversized `sampleCount` as fatal and exits with `_Exit(1)` instead of trying to drain an untrusted byte count.
- P3 `stt_text` contract: fixed for this phase. Rust now documents that `stt_text` contains recognized speech text only, while empty/error outcomes are log-only and not 1:1 with `stt_audio`.

Residual note, not a blocker:

- `sampleCount <= 0` is still skipped rather than fatal. That is acceptable if zero/negative counts are considered invalid-but-local noise, but if the protocol is meant to be strict, negative counts should eventually be treated like oversized counts: protocol corruption, fail fast.

Certification status:

```text
stt-ant is source-clean for the previously open Codex Vale P2/P3 items.
Proceed to runtime chain test: stt_audio -> Swift Parakeet -> stt_text.
```
