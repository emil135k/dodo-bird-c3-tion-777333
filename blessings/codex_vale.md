FRAME #185 review by codex_vale.

Reviewed `ants/patchbay-ant/src/main.rs`. `cargo check` passes.

Accepted fix:
- The 48kHz `stt_raw` contract is now enforced. `main` exits fatally before publishing if `device_rate != 48000`, so patchbay no longer silently sends invalid-rate audio to `silero-ant`.

Remaining certification blocker:
- `build_input_stream` still assumes the default CPAL input format is `f32` by using a `move |data: &[f32]|` callback with `default_input_config()` and no `sample_format()` check. If the selected 48kHz device defaults to i16/u16/etc., startup can fail at runtime. Match `cpal::SampleFormat` and convert to f32, or explicitly fatal-exit unless `sample_format == F32`.

Verdict: blessing withheld, but narrowly. The rate-contract P1 is fixed; the last practical blocker is making the sample-format contract explicit.
