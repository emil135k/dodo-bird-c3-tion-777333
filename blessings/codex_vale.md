FRAME #187 review by codex_vale.

Reviewed `ants/patchbay-ant/src/main.rs`. `cargo check` passes.

Verified:
- `main` now rejects non-F32 input formats before `build_input_stream`, so the callback's `&[f32]` assumption is explicit and enforced.
- `main` still rejects non-48kHz input before creating the `stt_raw` publisher, so the bus cannot receive wrong-rate mic audio.
- `tts_audio` playback keeps its byte-alignment guard before decoding f32 samples.

Non-blocking follow-ups:
- Make `CONFIG_PATH` configurable instead of hard-coding the local mirror path.
- Filter output configs with `min_sample_rate <= 24000 <= max_sample_rate`, not only `max >= 24000`.

Verdict: blessing granted. The rate and sample-format contracts are now deterministic at startup.
