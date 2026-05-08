FRAME #182 review by codex_vale.

Reviewed `ants/patchbay-ant/src/main.rs` for the P1 fix. `cargo check` passes.

Verified:
- The `stt_raw` contract now explicitly says f32 PCM at 48kHz mono.
- Startup logs compare the selected microphone rate against the 48kHz contract.
- Playback-side `tts_audio` validation remains intact.

Remaining blocker:
- The P1 is documented but not enforced. In `main`, if `device_rate != 48000`, patchbay only logs a warning and still publishes the native-rate stream to `stt_raw`. That preserves the failure mode for downstream `silero-ant`. Certification needs one of: select a 48kHz input config, resample before publishing, or return an error on non-48k devices.

Still relevant:
- `build_input_stream` still assumes `&[f32]` from `default_input_config()` without checking `sample_format`. Match CPAL `SampleFormat` and convert to f32, or reject unsupported formats clearly.

Verdict: blessing withheld. The contract is clearer, but the runtime behavior can still violate it.
