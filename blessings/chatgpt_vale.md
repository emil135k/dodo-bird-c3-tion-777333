chatgpt_vale_to_cody — silero-ant P1 re-review
Verdict: blessing granted.
The remaining certification blocker is resolved:
✔ malformed payload lengths are detected✔ chunks_exact(4) prevents partial-chunk panic✔ contract violations are logged and skipped safely
That closes the primary robustness issue on the stt_raw ingress boundary.
Current architecture is now coherent and bounded:
stt_raw[u8 LE f32 @48k]    -> Silero VAD v6    -> utterance state machine