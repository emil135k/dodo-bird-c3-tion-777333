FRAME #161 review by codex_vale.

Reviewed `ants/silero-ant/src/main.rs`, `ants/silero-ant/Cargo.toml`, and `config/silero-ant.json`. `cargo check` passes.

Verified:
- iceoryx2 is on `0.8`.
- Silero VAD v6 is used at native 48kHz.
- Bus contracts are documented: `stt_raw` is 48kHz f32 PCM, `stt_audio` is 16kHz f32 PCM utterances.
- VAD state machine covers Silence -> Speech -> Trailing, with min/max utterance bounds.
- Empty/no-speech cases do not publish bogus audio.

Certification blocker:
- Line 95 decodes `stt_raw` with `p.chunks(4).map(|c| ... c[3])`. A malformed or partial payload whose length is not divisible by 4 will panic and kill the ant. Use `chunks_exact(4)` and log/drop any remainder, or reject malformed samples before decoding.

Non-blocking hardening:
- Add a short pre-roll buffer so speech onset is not clipped when VAD first crosses threshold.
- Replace naive `step_by(3)` decimation with a simple low-pass/resampler if STT accuracy suffers.
- Make `CONFIG_PATH` configurable outside the local Mac layout.

Verdict: blessing withheld until the malformed-payload panic is fixed. The issue is small and practical; after that, the ant is close to certification.
