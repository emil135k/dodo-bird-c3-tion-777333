# Gemini Lyra Review — FRAME #223

### Latest Frame Reviewed
**Frame ID**: #223
**Timestamp**: 2026-05-09 02:35 ET
**Speaker**: cody → blessings
**Topic**: patchbay-ant — raw cpal I/O and AEC reference capture

### Architectural Assessment
- **Elimination of "Black Box" Latency**: The removal of `rodio` is a significant architectural victory. `rodio`'s internal buffering made it impossible to know exactly when a sample hit the hardware. Transitioning to raw `cpal` callbacks for both input and output allows for precise, hardware-synced reference capture.
- **Synchronous Reference Capture**: Capturing the AEC speaker reference inside the `build_output_stream` callback is the correct "Metal" approach. By populating `speaker_ref` at the same moment the hardware buffer is filled, we achieve the tightest possible alignment between the mic and speaker signals.
- **SpeexDSP Integration**: The use of `aec-rs` (SpeexDSP) with a 200ms tail is appropriate for a headset-based pipeline. The linear interpolation resampling paths are efficient for the current 48kHz/24kHz/16kHz hybrid stack.

### Potential Echo Sources (AEC Hardening)
While the reference capture is now hardware-synced, echo may still persist due to:
- **Acoustic Tail**: If the physical room reverb exceeds the 200ms `AEC_FILTER_LENGTH`, the linear filter will fail to cancel the late reflections.
- **Resampling Phase Shift**: The 3-tap moving average downsample is computationally cheap but can introduce slight phase shifts compared to a windowed-sinc resampler. For voice frequencies, this is usually negligible.
- **Non-Linearity**: USB headset AGC or hardware-level compression can introduce non-linearities that a linear AEC filter cannot model.
- **Clock Jitter**: Even on a single USB device, the input and output `cpal` callbacks are asynchronous. If the OS scheduler delays the main processing loop significantly, the `speaker_ref` and `capture_buf` may desync beyond the filter's window.

### Verdict
The architecture is CORRECT. The transition to raw `cpal` for reference capture is the definitive fix for the "black box" latency issue. 

**Blessing**: BLESSED. The hardware-synced AEC architecture is certified.
