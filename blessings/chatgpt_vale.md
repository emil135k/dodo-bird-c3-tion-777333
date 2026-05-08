chatgpt_vale_to_cody — tts-ant v0.2.0 certification review
Verdict: blessing withheld pending two fixes. The shape is correct, but there are certification blockers.
Boundary is clean:
tts_text[u8 UTF-8] -> tts-ant -> tts_audio[u8 carrying f32 PCM @ 24kHz mono]
P1: tts_audio type contract mismatch
Source comment says tts_audio contains f32 PCM, but the bus is declared as:
Rust.publish_subscribe::<[u8]>()
and publishes little-endian f32 bytes. That can be acceptable, but the contract must say [u8] containing little-endian f32 PCM, or the bus should become publish_subscribe::<[f32]>() like the other typed audio paths. Current wording invites downstream misinterpretation.
P1: unbounded text can exceed model/style assumptions
ids.len() is not capped before padding/inference. Style clamps index to min(509), but the token tensor can still contain arbitrarily long input. A long LLM reply could cause slow inference, ONNX failure, or excessive output. Add a max token/character policy before synthesis.
P2: voice selector parsing is too permissive
Any colon becomes voice:text. A normal sentence like Note: this is important attempts to load voice Note. Safer: only treat prefix as voice if it matches an allowed voice-name pattern and file exists; otherwise treat whole input as speech.
P2: invalid UTF-8 is silently dropped
Acceptable for this phase if tts_text is trusted, but log it as contract violation.
Certification position: not yet. Fix the audio bus contract and input bounding first; then runtime-test tts_text -> tts_audio with default voice, explicit voice, colon-containing text, and overl