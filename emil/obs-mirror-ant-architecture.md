# OBS Mirror Ant Architecture

## Purpose

Record the full AI session directly in OBS without making OBS compete for hardware audio devices and without splicing a separate audio file after recording.

## Principle

OBS should observe virtual mirrored audio. It should not own the hardware mic, headset, speakers, or Apple Voice Processing path.

The realtime path stays owned by the ant stack:

```text
hardware mic/headset
  -> patchbay-ant + Swift worker
  -> stt_raw / silero / stt / tts_audio
  -> patchbay playback
```

The OBS path is additive:

```text
stt_raw   -> obs-mirror-ant -> BlackHole 2ch left channel
tts_audio -> obs-mirror-ant -> BlackHole 2ch right channel
OBS captures BlackHole 2ch as one stereo input
```

## MVP Signal Contract

- `stt_raw`: f32 PCM, 48kHz mono, little-endian bytes
- `tts_audio`: f32 PCM, 24kHz mono, little-endian bytes
- `obs-mirror-ant` output: 48kHz stereo
  - Left channel: mic/STT raw mirror
  - Right channel: TTS mirror, upsampled 24kHz -> 48kHz by sample duplication

This gives OBS a single virtual source with separable channels.

## Why BlackHole 2ch First

BlackHole 2ch keeps the first experiment simple:

- one OBS audio source
- no Audio MIDI Setup multi-output device
- no direct hardware contention
- no patchbay modification
- no dependency on OBS capturing the mic directly

BlackHole 16ch can come later if we want multichannel layout such as mic, TTS, system audio, phone audio, and debug tones on separate tracks.

## Risk Boundaries

This ant is not part of the critical voice loop. If it crashes, transcription and TTS should continue.

It does not publish to `stt_raw`, `stt_audio`, or `tts_audio`. It only subscribes and writes to a virtual output device.

## Operation

Build:

```bash
cd hypAiAssist/ants/obs-mirror-ant
cargo build --release
cp target/release/obs-mirror-ant ~/.local/bin/
```

Start:

```bash
hypAiAssist/scripts/obs-mirror/start-obs-mirror.sh
```

Stop:

```bash
hypAiAssist/scripts/obs-mirror/stop-obs-mirror.sh
```

OBS source:

- Add Audio Input Capture
- Select `BlackHole 2ch`
- Configure recording as stereo
- Left channel is mic, right channel is TTS

## Future

Later versions can add:

- 16-channel BlackHole layout
- per-channel gain
- sample-rate conversion with interpolation
- optional mixed mono feed
- OBS control integration
- dynamic enable/disable through `speaker_control` or a dedicated `obs_control` topic
- status reporting to Hammerspoon or Jacob's Lattice
