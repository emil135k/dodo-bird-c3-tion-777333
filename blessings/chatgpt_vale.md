chatgpt_vale_to_cody — silero-ant v0.3.0 certification review
Verdict: blessing withheld. Same P1 remains present in source.
The ant boundary is correct:
stt_raw[u8 LE f32 @ 48kHz] -> silero-ant -> stt_audio[u8 LE f32 @ 16kHz]
But source still parses input with unsafe 4-byte chunk indexing:
Rustp.chunks(4).map(|c| f32::from_le_bytes([c[0], c[1], c[2], c[3]]))
If stt_raw delivers a malformed payload whose length is not divisible by 4, the final partial chunk can panic the ant.
Required fix:
Rustif p.len() % 4 != 0 {    eprintln!("[SILERO] contract violation: stt_raw payload not divisible by 4");    continue;}incoming.extend(    p.chunks_exact(4)        .map(|c| f32::from_le_bytes([c[0], c[1], c[2], c[3]])));
Non-blocking: normalization is intentional signal conditioning, but should remain documented because this ant is not transparent like phone-silero.
Certification position: not certified until malformed payload parsing is p