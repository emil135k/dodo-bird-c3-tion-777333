FRAME #177 review by codex_vale.

Reviewed `ants/patchbay-ant/src/main.rs`, `ants/patchbay-ant/Cargo.toml`, and `config/patchbay-ant.json`. `cargo check` passes.

Verified:
- patchbay-ant is on iceoryx2 `0.8` and Cargo version `0.2.0`.
- The router keeps the bus split clear: mic capture publishes `stt_raw`, playback consumes `tts_audio`.
- `tts_audio` payloads are byte-alignment checked before f32 decode.
- Playback is explicitly configured for 24kHz mono buffers.

Certification blockers:
- `main` builds the input stream as `&[f32]` from `default_input_config()` without checking the CPAL `sample_format`. A default i16/u16/etc. microphone can fail at runtime. Match `SampleFormat` and convert to f32, or reject non-f32 input with a clear startup error.
- `stt_raw` is documented as device-native rate while downstream Silero expects 48kHz. `device_rate` is logged but not enforced, resampled, or carried in metadata. Require a 48k input config, resample to 48k, or change the bus contract so consumers know the actual rate.

Non-blocking:
- Make `CONFIG_PATH` configurable instead of hard-coding Jacob's local mirror path.
- In `find_output_config`, filter with `min_sample_rate <= 24000 <= max_sample_rate`, not only `max >= 24000`.
- Consider bounded `stt_raw` publish chunks so callback backlog cannot create arbitrarily large frames.

Verdict: blessing withheld until input sample-format and sample-rate handling are deterministic. The routing shape is sound, but certification needs a stable audio contract.
