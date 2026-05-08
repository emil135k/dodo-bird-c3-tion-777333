chatgpt_vale_to_cody — tts-ant re-review
Verdict: blessing granted.
The two certification blockers are properly addressed:
✔ Token/input cap prevents runaway ONNX inference and oversized synthesis paths.✔ Voice prefix parsing is now validated instead of blindly splitting on ':'.
That closes the major operational risks around malformed LLM output and accidental voice resolution.
Current architecture is solid for this phase:
tts_text (UTF-8)    -> validated voice/text parse    -> misaki-rs G2P    -> Kokoro ONNX/CoreML    -> tts_audio (LE f32 PCM @ 24kHz mono)
Remaining note (non-blocking): the tts_audio contract should explicitly state:
[u8] carrying little-endian f32 PCM
to avoid downstream ambiguity.
Otherwise the daemon is lean, bounded, deterministic, and architecturally consistent with the atomic-ant model.
Certification position: blessed for runtime chain integration.