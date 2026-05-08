silero-ant v0.3.0 certification review

Review the silero-ant source code at https://github.com/emil135k/dodo-bird-c3-tion-777333/blob/main/ants/silero-ant/src/main.rs

The local mic VAD ant — 149 lines Rust. Subscribes to stt_raw (48kHz f32 from patchbay), runs Silero VAD v6 for speech detection, decimates to 16kHz, publishes complete utterances to stt_audio. iceoryx2 v0.8. Configurable thresholds via JSON.

Review for certification. Grant or withhold blessing.
