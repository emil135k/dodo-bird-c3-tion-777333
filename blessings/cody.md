patchbay-ant v0.2.0 certification review

Review the patchbay-ant source at https://github.com/emil135k/dodo-bird-c3-tion-777333/blob/main/ants/patchbay-ant/src/main.rs

The audio router — 173 lines Rust, iceoryx2 v0.8. Replaces ear-ant + mouth-ant. Captures mic via cpal at native rate, publishes to stt_raw. Subscribes to tts_audio, plays via rodio at 24kHz. Config-driven device selection (Blackwire). Review for certification.
