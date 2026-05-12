## 2026-05-11 20:29 ET - codex_vale - Continued Wormhole Review

Verdict: the wormhole pattern is sound. The reviewed source keeps Rust and Swift in separate processes, uses anonymous pipes instead of FFI, and correctly reserves stdout for protocol data while sending worker logs to stderr. The architecture is credible as a Swift/Apple-framework adapter for an iceoryx2 Rust bus.

Actionable release blockers:
1. Treat stdout as a strict wire protocol. Add a versioned frame header with magic, version, message type, length, and explicit error/status frames.
2. Fix the audio worker's `Darwin.write` calls to retry short writes and fail on errors. Ignoring return values can corrupt the stream.
3. Make worker paths, Swift package paths, and iceoryx2 root configurable. Public examples cannot depend on `/Users/rocketman/...`.
4. Add a fake Swift worker and fake Rust host test so the pipe contract can be validated without CoreML, AVAudioEngine, microphone permissions, or Apple Silicon.
5. Clarify sample-rate contracts. `audio-example` comments mention 16 kHz, Rust publishes `stt_raw` as 48 kHz, and playback expects 24 kHz.
6. Document lifecycle behavior: startup timeout, worker crash restart policy, pipe EOF, corrupt frame, graceful shutdown, and backpressure.

Open-source readiness: publishable as an experimental template after the docs are cleaned up, but not yet as a reusable library pattern. The core win is isolation and debuggability, not throughput; frame the README around that claim.
