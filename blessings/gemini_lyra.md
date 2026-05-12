## 2026-05-11 21:05 ET - gemini_lyra - Architecture & Signal Integrity Audit

**Verdict:** The Wormhole pattern successfully isolates Apple's unstable framework surface from the sovereign Rust bus, but the current implementation compromises real-time performance and signal fidelity.

**Actionable Improvements:**

1. **Eliminate Busy-Wait Latency:** Both `audio-example` and `stt-example` use `try_recv()` followed by `std::thread::sleep(5-10ms)`. This introduces a fixed latency floor that is unacceptable for low-latency voice interactions.
   - **Action:** Transition to blocking reads or use an async runtime (tokio/monoio) to handle pipe I/O reactively.

2. **Fix High-Frequency Aliasing:** The Swift audio worker uses a primitive "nearest neighbor" decimation for resampling (e.g., native to 48kHz). This introduces significant aliasing artifacts.
   - **Action:** Implement proper resampling using `AVAudioConverter` in the Swift worker to ensure signal integrity when hardware rates differ from bus contracts.

3. **Normalize Signal Levels:** The hardcoded cumulative 5.0x volume boost (2.0x in Rust, 2.5x in Swift) is a fragile workaround for "ducking" or AGC issues.
   - **Action:** Audit `AVAudioSession` and `AVAudioEngine` configurations. Specifically, ensure `voiceProcessingOtherAudioDuckingConfiguration` is actually effective and that the bus contract specifies a normalized peak amplitude (e.g., -3dBFS).

4. **Protocol Self-Description:** The current protocol assumes fixed sample rates and formats known at compile time.
   - **Action:** Add a 16-byte header to the pipe stream containing `[Magic: 4B][Version: 2B][Format: 2B][Rate: 4B][PayloadLen: 4B]`. This allows the Rust side to adapt to hardware-specific rates reported by the Swift worker.

5. **Robust Lifecycle:** `_Exit(1)` on protocol desync or CoreML failure prevents graceful cleanup of iceoryx2 resources or temporary files.
   - **Action:** Implement a structured shutdown signal and allow the Rust "Ant" to attempt worker restarts with exponential backoff.

**Overall Assessment:** A brilliant isolation strategy currently hampered by "prototype-grade" signal processing. Solve the latency and aliasing issues before promoting this as a reference architecture.
