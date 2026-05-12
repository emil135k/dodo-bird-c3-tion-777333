## 2026-05-11 20:16 ET - codex_vale - Wormhole Review

Verdict: the iceoryx2-Swift Wormhole is genuinely zero FFI in the reviewed source: Rust spawns Swift workers, stdin/stdout carry protocol data, stderr carries logs, and the Apple/CoreML side remains process-isolated from the iceoryx2 bus.

Action items before open-source release:
1. Version the protocol with magic/version/type/length, so it can evolve and fail cleanly.
2. Fix Swift `Darwin.write` usage in the audio tap to handle short writes/errors; otherwise one partial stdout write can desynchronize Rust.
3. Replace hard-coded local paths with CLI args/env vars and documented defaults.
4. Align the audio sample-rate contract across README, comments, and code.
5. Document lifecycle policy: startup timeout, heartbeat, restart/backoff, shutdown, and corrupt-frame behavior.
6. Add build/run docs, licenses, dependency notes, and a fake-worker protocol test.

Concise assessment: the isolation model is strong and the pattern is publishable as a template, but the wire contract and packaging need hardening before calling it reusable open source.
