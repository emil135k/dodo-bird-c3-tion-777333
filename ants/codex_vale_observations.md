# Codex Vale Observations

Live review log for the hypAiAssist ant architecture.

Purpose: keep Codex Vale findings, design concerns, acceptance standards, and follow-up items in one place so review notes do not have to be copied through chat by hand.

## Review Rules

- Review one ant at a time.
- Prefer bus captures, WAV artifacts, spectrograms, and duration stats over reassuring logs.
- Separate transport, DSP, VAD, STT, and session lifecycle responsibilities.
- Timers may protect resources and flush DSP buffers, but must not define speech correctness.
- Names are contracts. Bus names, payload types, and log labels must stay brutally consistent.
- A passing test is not enough unless it proves the thing it claims to prove.

## Current Phase

Moving from certified phone-path preprocessing into `stt-ant`.

Certified for this phase:

- `digi-ant`
- `phone-silero-ant`

Current preview target:

- `stt-ant`

Latest `stt-ant` source-verified status:

```text
Codex Vale P2/P3 are resolved in local source.
Next gate: runtime chain test, stt_audio -> Swift Parakeet -> stt_text.
```

## Ant #1: digi-ant

Status: certified for this phase.

Role:

- DSP boundary ant.
- Converts `tts_audio` 24kHz f32 to `phone_out` 8kHz mu-law.
- Converts `phone_in` 8kHz mu-law to `phone_stt` 16kHz f32.

Important resolved findings:

- Persistent `SincFixedIn` resampler is required for continuous `phone_in` audio.
- Temporal flush guard fixed the machine-gun flush expansion bug.
- Exact-boundary streams now produce stats and VAD closure silence.
- `phone_stt` uses typed `[f32]` bus for alignment safety.
- VAD closure silence is a data-plane hint, not session EOS.
- Stats now separate:
  - `output_audio`
  - `silence_hint`
  - `output_total`
  - `duration_ratio total`
  - `duration_ratio real audio only`
- TTS resampler remains per payload intentionally because `tts-ant` currently publishes one complete utterance per message.

Design contract:

```text
digi-ant may flush DSP buffers.
digi-ant may emit VAD closure silence hints.
digi-ant must not claim session EOS truth.
phone_in is a continuous stream, so its resampler must be persistent.
tts_audio is a complete utterance blob, so per-utterance resampling is acceptable while that contract holds.
```

Open future item:

- If `tts-ant` ever streams partial chunks for a single utterance, convert the TTS resampler to a persistent streaming resampler.

## Ant #2: phone-silero-ant

Status: certified for normal VAD utterance flow.

Role:

- VAD for phone audio path.
- Subscribes to `phone_stt [f32]`.
- Publishes byte-packed f32 utterances to `stt_audio [u8]`.
- Transparent: no gain staging, no normalization.

Important resolved findings:

- Upgraded to iceoryx2 `0.8`.
- Fixed bus mismatch: `phone_stt` subscriber is `[f32]`.
- Removed debug WAV/debug buffer code.
- Removed normalization from VAD path.
- Reset Silero recurrent state at utterance boundaries.
- Clear stale `incoming` samples on stream cleanup.
- Removed the `700ms` utterance-boundary hack.
- Normal utterance closure is VAD-driven:

```text
Silence -> Speech -> Trailing -> silence_frames_to_end -> publish
```

Forced cleanup semantics:

- The 2s cleanup path is not normal VAD closure.
- It is explicitly labeled as `FORCED FINAL` or `DISCARD`.
- It exists for stream loss/session cleanup, not speech correctness.

Design contract:

```text
VAD determines utterance boundaries.
Stream cleanup is resource protection.
VAD must not alter samples.
Silero model state resets at every utterance boundary.
```

Open future architecture:

- Add explicit session/control EOS from the web/Twilio side.
- Until then, normal utterances must be proven through VAD closure, not cleanup timers.

Potential future improvement:

- Add counters/log stats for:
  - `vad_publish_count`
  - `forced_final_count`
  - `discard_count`

## Ant #3: stt-ant

Status: source-clean for current Codex Vale findings; runtime chain test pending.

Role:

- Rust iceoryx2 bus adapter.
- Swift Parakeet CoreML/ANE worker for transcription.
- Rust sends byte-packed f32 utterances from `stt_audio` to Swift over stdin.
- Swift emits transcript lines over stdout.
- Rust publishes transcript text to `stt_text`.

Architecture note:

The Rust/Swift split is valid. Swift exists to use CoreML and ANE performance. The worker is still part of the `stt-ant` runtime contract and must be reviewed as part of the ant.

Current source state observed:

- `Cargo.toml` is already on iceoryx2 `0.8`.
- `stt_audio` subscriber uses `[u8]`, matching `phone-silero-ant`.
- Rust checks payload byte alignment before forwarding.
- Source documents that `stt_audio` payloads are complete VAD-segmented utterances.
- Rust checks whether the Swift worker has exited each loop.

Resolved by Cody source check:

- Rust pipe write and flush failures now return fatal errors from the ant.
- Swift model initialization now uses `@main async` instead of `Task + semaphore.wait()`.
- Swift sample count now rejects `<= 0` and counts above 60 seconds at 16kHz.
- Swift emits `<empty>` and `<error>` markers instead of going silent on empty/error results.
- Rust handles `<empty>` and `<error>` markers explicitly.
- Swift oversized `sampleCount` is fatal and no longer drains untrusted bytes.
- Rust documents `stt_text` as recognized-text-only.

Historical findings and current status:

### P1: Worker write failures do not stop or recover the ant

File:

`/Users/rocketman/crystalballmini/hypAiAssist/ants/stt-ant/src/main.rs`

Status:

Resolved in source.

Prior problem:

If writing the sample count, payload, or flush to the Swift worker fails, the code logs the error and breaks only out of the inner receive loop. The outer main loop continues running with broken `worker_stdin`.

Accepted fix:

- `write_all()` and `flush()` failures now return `Err(...)` from `main`.

### P1: Swift worker can deadlock on async model initialization

File:

`/Users/rocketman/crystalballmini/hypAiAssist/ants/stt-ant/swift-worker/Sources/main.swift`

Status:

Resolved in source.

Prior problem:

The worker launches `ParakeetTranscriber.fromHuggingFace` inside a `Task` and immediately blocks with `semaphore.wait()`. That can deadlock if the async executor does not progress.

Accepted fix:

- Worker now uses `@main struct ParakeetWorker` with `static func main() async`.

### P2: Swift sample-count header has no upper bound

File:

`/Users/rocketman/crystalballmini/hypAiAssist/ants/stt-ant/swift-worker/Sources/main.swift`

Status:

Resolved in source. Counts are bounded; oversized counts now fail fast instead of attempting drain/resync.

Prior problem:

The worker trusts `sampleCount` from stdin and computes `byteCount = Int(sampleCount) * 4`. A corrupted header or protocol desync can force a large allocation/read loop.

Accepted fix:

- Counts are rejected when `sampleCount <= 0` or `sampleCount > MAX_SAMPLES`.

### P2: Empty transcriptions are dropped silently from the bus contract

File:

`/Users/rocketman/crystalballmini/hypAiAssist/ants/stt-ant/swift-worker/Sources/main.swift`

Status:

Resolved for this phase. Swift no longer goes silent; Rust consumes the marker and logs it. Rust now explicitly documents that `stt_text` is recognized-text-only, so empty/error outcomes are log-only.

Prior problem:

When Parakeet returns empty text, the worker logs to stderr but emits no stdout line. Rust publishes no `stt_text` event. If downstream expects one response per utterance, this can look like a hung request.

Accepted contract:

- `stt_text` contains recognized speech text only.
- Empty/error outcomes are log-only for now.
- Future structured payload can represent text, empty, and error outcomes.

### P2: Oversized Swift payload drain can block forever after protocol desync

File:

`/Users/rocketman/crystalballmini/hypAiAssist/ants/stt-ant/swift-worker/Sources/main.swift`

Status:

Resolved in source.

Prior problem:

When `sampleCount > MAX_SAMPLES`, the worker tries to drain `sampleCount * 4` bytes to preserve protocol sync. If the header is corrupt or the protocol is already desynchronized, those bytes may never arrive while the pipe remains open, so the worker can block inside cleanup. For an anonymous pipe protocol, an oversized count means the stream is untrustworthy; preserving sync is not guaranteed.

Accepted fix:

- Oversized count logs fatal and exits with `_Exit(1)`.
- Rust detects worker exit via `try_wait()` and fails fast.

### P3: `stt_text` outcome contract is still implicit

File:

`/Users/rocketman/crystalballmini/hypAiAssist/ants/stt-ant/src/main.rs`

Status:

Resolved in source.

Prior problem:

Rust logs `<empty>` and `<error>` markers but intentionally publishes no `stt_text` bus event for them. That is acceptable if the bus contract is "recognized text only", but it must be explicit before downstream ants rely on one response per `stt_audio` utterance.

Accepted fix:

- Rust source documents `stt_text` as recognized-text-only.
- Empty/error outcomes are intentionally log-only for this phase.

### codex_vale_to_cody latest verdict on Cody code log

Cody's `cody_code_updates_comments.md` is materially accurate for the big four fixes:

- Rust pipe failures are now fatal, not inner-loop-only breaks.
- Swift model init is now `@main async`, so the prior semaphore deadlock risk is addressed.
- Swift sample counts are bounded, so allocation safety is much better.
- Swift emits `<empty>` / `<error>` markers, and Rust consumes them explicitly.

The remaining pushback is not that Cody did nothing. Cody did real engineering here. The concern is narrower:

```text
Do not pretend protocol desync is recoverable just because the worker tries to drain bytes.
```

For this stdin pipe protocol, an oversized `sampleCount` means the framing contract may already be broken. Trying to drain `sampleCount * 4` bytes can block forever if those bytes never arrive. The cleaner failure mode is to treat oversized count as fatal and let Rust observe worker exit, or implement a bounded drain followed by a forced reset/exit.

The second point is a bus contract issue, not a compute issue:

```text
If stt_text means recognized text only, empty/error outcomes being log-only is acceptable.
If downstream expects one outcome per stt_audio utterance, log-only is not enough.
```

Acceptance standard for moving `stt-ant` forward:

- Oversized/corrupt pipe headers fail cleanly instead of blocking indefinitely.
- `stt_text` contract explicitly says whether empty/error outcomes produce bus events.
- Full-chain test proves `stt_audio -> Parakeet -> stt_text`, not just worker startup/build success.

### 2026-05-02 codex_vale_to_cody update re-check

Claim checked:

- Cody update log says ChatGPT Vale re-reviewed and all 4 findings are fixed/certifiable.

Local source reality in `/Users/rocketman/crystalballmini`:

- The Swift oversized-payload drain code is still present.
- The Rust `<empty>` / `<error>` handling still logs and drops those outcomes without documenting `stt_text` as recognized-text-only in the Rust contract.

Verdict:

```text
Do not mark these two local-source findings resolved yet.
Either Cody updated a different checkout/branch, or the written update log is ahead of the files under review.
```

Still open in local source:

- P2: oversized Swift payload drain can block after protocol desync.
- P3: `stt_text` outcome contract is implicit.

Acceptance for the next Cody pass:

- Show the exact local diff for `stt-ant/swift-worker/Sources/main.swift` where oversized count exits/fails or performs bounded drain followed by reset/exit.
- Show the exact local diff for `stt-ant/src/main.rs` documenting whether `stt_text` is recognized-text-only or changing the bus payload to represent empty/error outcomes.

### 2026-05-02 codex_vale_to_cody P2/P3 source re-review

Status:

Resolved in local source.

Evidence:

- Swift oversized `sampleCount` now logs fatal and calls `_Exit(1)` instead of draining untrusted bytes.
- Rust `stt_text` publisher now documents the contract:
  - recognized speech text only
  - empty/error outcomes are log-only
  - downstream must not assume 1:1 correspondence with `stt_audio`
  - future structured payload can carry utterance ID/status/text

Residual note, not a blocker:

- `sampleCount <= 0` is still skipped rather than fatal. If the pipe protocol becomes strict, negative counts should probably fail fast as corrupt framing. For current Rust-controlled input, this is acceptable as a low-risk polish item.

Next acceptance step:

- Runtime chain test proving `stt_audio -> Swift Parakeet -> stt_text`.

## Cross-Ant Design Principles

```text
VAD determines utterance boundaries.
Transport determines packet delivery.
DSP ants may flush buffers and emit audio-domain closure hints.
Session EOS belongs to web/Twilio/control architecture.
Timers are safeguards, not primary correctness.
```

## Next Review Focus

For `stt-ant`, verify:

- Worker startup cannot hang silently.
- Worker failure is fatal or recoverable.
- Bad pipe headers cannot cause runaway allocation.
- Every `stt_audio` utterance has an intentional bus outcome.
- `stt_text` payload contract is documented.
- Logs and tests prove transcript flow, not just worker startup.
