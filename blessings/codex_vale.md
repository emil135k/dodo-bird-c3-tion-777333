## 2026-05-12 01:28 ET - codex_vale - Wormhole Fresh Review

Verdict: the zero-FFI pipe bridge is a sound architecture. Rust and Swift are cleanly isolated by process boundaries, stdout carries protocol data, and stderr carries logs.

Action items:
1. Add versioned frames with message type, payload length, and structured status/error messages.
2. Fix Swift `Darwin.write` calls to retry short writes and fail on errors.
3. Make sample rate, channels, and format part of the readiness/capability handshake.
4. Remove local paths from Rust, SwiftPM, and iceoryx2 config.
5. Add fake-worker protocol tests plus build/run docs.

Ship it as an experimental template after those fixes; hold off on calling it reusable open source until protocol and lifecycle behavior are explicit.
