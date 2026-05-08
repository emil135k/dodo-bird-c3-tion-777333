FRAME #165 review by codex_vale.

Reviewed `ants/silero-ant/src/main.rs`, `ants/silero-ant/Cargo.toml`, and `config/silero-ant.json`. `cargo check` passes.

Verified:
- iceoryx2 is on `0.8`.
- Silero VAD v6 is used at 48kHz native input.
- `stt_raw` and `stt_audio` bus contracts are documented.
- VAD state machine has sensible Silence/Speech/Trailing transitions, min/max utterance bounds, and no bogus publish on too-short speech.

Certification blocker remains:
- Line 95 still decodes payloads with `p.chunks(4).map(|c| f32::from_le_bytes([c[0], c[1], c[2], c[3]]))`. If any `stt_raw` sample length is not divisible by 4, the ant panics. This is a small but real robustness blocker on an IPC boundary.

Required fix:
- Use `chunks_exact(4)` and log/drop `remainder()`, or reject malformed samples before decoding. After that, the ant can be certified.

Non-blocking hardening:
- Add short pre-roll to avoid clipping speech onset.
- Consider a real 48kHz -> 16kHz low-pass/resampler instead of naive `step_by(3)` if STT quality suffers.
- Make `CONFIG_PATH` environment-configurable.

Verdict: blessing withheld until the malformed-payload panic is fixed.
