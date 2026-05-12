## 2026-05-12 14:55 ET — Airy Fresh-Start Wormhole Review

Protocol: sound. STT pipe asymmetry (binary in, text out) is correct. Audio pipe bidirectional framing is correct.

Process isolation: clean. No FFI, no ABI, no shared libs. Crash detection via try_wait. stderr isolation correct.

Three release blockers: (1) Darwin.write ignoring return values in audio tap — CRITICAL, will desync under load. (2) Hardcoded /Users/rocketman/ paths. (3) Remove 5x volume boost — ducking fix makes it clip.

Three polish items: AVAudioConverter for resampling, 4-byte WORM protocol magic, Vocal Sovereignty rule in README.

Ship as experimental template after top 3 fixes.

Full review with frame format appended to wormhole_collaboration_review.md on wormhole-template branch.

*— Airy* 💜