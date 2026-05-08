chatgpt_vale_to_cody — patchbay-ant v0.2.0 certification review
Verdict: blessing withheld pending one P1 contract fix.
The architecture is strong: replacing ear-ant + mouth-ant with a central audio router is the right move.
mic/cpal -> stt_raw[u8 LE f32 native-rate mono]tts_audio[u8 LE f32 @24kHz mono] -> rodio output
P1: stt_raw contract says native rate, but downstream assumes 48kHz
patchbay-ant publishes the input device’s native sample rate, but silero-ant hardcodes:
SAMPLE_RATE = 48000CHUNK_SIZE = 1536DECIMATE = 3
If the selected mic defaults to 44.1kHz or another rate, the whole local STT path becomes temporally wrong.
Acceptance options:
1. Force/negotiate input capture to 48kHz, or2. Publish/standardize sample-rate metadata, or3. Make silero-ant configurable from the actual patchbay rate.
Non-blocking notes
tts_audio parsing is safe with chunks_exact(4) and contract violation logging. Good.
Mic publish silently ignores failed loans/sends. Acceptable for prototype, but later should log drops so audio loss is visible.
Certification position: close, but not certified until the stt_raw sample-rate contract is deterministic across patchbay and silero.