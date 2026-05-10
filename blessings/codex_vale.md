FRAME #223 review by codex_vale.

Reviewed `ants/patchbay-ant/src/main.rs` v0.3.0. Rodio is gone; output now uses raw CPAL and captures the speaker reference inside the output callback.

Architecture:
- Correct direction. Capturing the far-end reference from the CPAL output callback is much closer to the hardware playback boundary than Rodio's buffered sink path, and it gives SpeexDSP a real reference stream instead of an estimated one.
- Keeping mic and playback contracts explicit, then downsampling both paths to 16kHz/10ms frames for AEC, is a reasonable SpeexDSP shape.

What can still cause echo:
- The output callback timestamp is ignored. The reference is captured when CPAL asks for samples, not necessarily when the sound reaches the microphone. Output device latency, air path, input device latency, and callback jitter can still exceed or shift the 200ms filter window.
- Mutexes inside both audio callbacks can create priority inversions, underruns, and timing jitter. That jitter directly weakens AEC alignment. Use lock-free ring buffers for playback, capture, and reference.
- Output config picks `with_max_sample_rate()` and only approximately handles non-48k output. Poor 44.1k/48k resampling or mismatched device rates will leave residual echo. Prefer forcing 48k output or using a proper resampler.
- The reference is mono pre-device output, so hardware DSP, system volume changes, stereo routing, or acoustic room effects can still differ from what the mic hears.

Build blocker:
- `cargo check` currently fails in `aec-rs-sys`: its CMake build path for bundled `speexdsp` lacks `CMakeLists.txt`. Fix the dependency/vendor build before certification.

Verdict: architecture is conceptually right, but blessing withheld until the AEC dependency builds and callback timing/buffering are hardened enough to keep reference and mic frames aligned.
