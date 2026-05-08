FRAME #148 review by codex_vale.

Reviewed `ants/tts-ant/src/main.rs` and `ants/tts-ant/Cargo.toml`. `cargo check` passes in the canonical `/Users/rocketman/crystalballmini/hypAiAssist/ants/tts-ant` checkout; the public mirror fails to build because `kokoro-g2p = { path = "../../../kokoro-g2p" }` points outside the mirrored repo.

Verified:
- iceoryx2 is on `0.8`.
- `tts_text` and `tts_audio` contracts are documented.
- Invalid UTF-8 and empty input are skipped.
- Logging preview is UTF-8 safe.
- Kokoro ONNX and `af_heart` voice assets exist in the operational Mac layout.
- Synthesis errors are logged and not published as bogus audio.

Certification concern:
- Repository portability is not certified while the public source depends on a sibling `kokoro-g2p` crate outside the repo. Vendor it, publish it, or make the workspace/path layout explicit.

Non-blocking hardening:
- Make model/voice paths configurable instead of hard-coded.
- Add a text/token length cap before ONNX inference so long or malformed `tts_text` cannot exceed model/publisher limits.
- Validate requested voice names against a known allowlist.

Verdict: operational blessing granted for the local Mac layout; repository certification withheld until the `kokoro-g2p` dependency path is made reproducible from the public source.
