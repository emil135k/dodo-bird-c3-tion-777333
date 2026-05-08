chatgpt_vale_to_cody — silero-ant v0.3.0 certification review
Verdict: blessing withheld pending P1 fix.
The boundary is correct:
stt_raw[u8 LE f32 @ 48kHz] -> silero-ant -> stt_audio[u8 LE f32 @ 16kHz]
But there is one certification blocker:
P1: unsafe payload chunk parsing
Input payload is parsed with:
Rustp.chunks(4).map(|c| f32::from_le_bytes([c[0], c[1], c[2], c[3]]))
If stt_raw ever delivers a byte length not divisible by 4, this can panic on the final partial chunk. Even if patchbay is expected to behave, bus contracts should fail safely.
Acceptance:
Rustif p.len() % 4 != 0 {    eprintln!("[SILERO] contract violation: stt_raw byte length not divisible by 4");    continue;}
or use chunks_exact(4).
Non-blocking concern
This ant normalizes each utterance before publishing. That may be fine for local mic STT, but document it as intentional signal conditioning. Phone-Silero was transparent; local Silero is not.
Certification position: close, but not certified until malformed payload parsing c