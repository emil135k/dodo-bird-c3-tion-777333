# Headset Recording Guide — Blackwire / USB Headset Mode

## Overview

The atomic ant audio stack supports two modes:

- **Speakerphone**: Built-in MacBook mic + speakers, Apple Voice Processing (AEC) ON
- **Headset**: USB headset (Poly Blackwire 3210, etc.), Apple AEC OFF, clean raw capture

Headset mode bypasses Apple's Voice Processing entirely. No AEC, no AGC, no Voice Isolation. The USB headset provides physical isolation between mic and earpiece, so echo cancellation is unnecessary. This produces cleaner, more natural voice recordings without the "through a pillow" compression that Apple VP applies.

## How It Works

The same patchbay-worker handles both modes. The `PATCHBAY_VP` env var controls the switch:

```
Speakerphone (VP ON):
  AVAudioEngine → Voice Processing enabled → mic at 96kHz → downsample to 48kHz → stt_raw

Headset (VP OFF):
  AVAudioEngine → Voice Processing skipped → mic at 8kHz (device native) → upsample to 48kHz → stt_raw
```

The stt_raw bus contract stays at 48kHz in both modes. Downstream ants (silero, stt, session-recorder) don't know or care which mode is active.

## Quick Start — Headset Recording Session

### 1. Plug in the USB headset

### 2. Select headset in System Settings
- System Settings > Sound > Input: select "Plantronics Blackwire 3210 Series" (or your headset)
- System Settings > Sound > Output: select "Plantronics Blackwire 3210 Series"
- Input volume: 50% is fine

### 3. Switch to headset mode
```bash
audio-mode headset
```
This sets `PATCHBAY_VP=off`, `PATCHBAY_TTS_GAIN=0.8`, restarts the voice stack.

### 4. Verify voice works
Speak into the headset. You should see transcriptions in the Claude Code terminal. Test with a simple phrase.

### 5. Start recording
```bash
atomic-rec-start
```
This launches:
- `session-recorder-ant` — captures stt_raw (mic) + tts_audio (TTS) to stereo WAV
- OBS with `AtomicDisplayOnly` profile — screen-only video, no audio

Say "SYNC MARK ONE TWO THREE" for the audio sync point.

### 6. Do your session
Work normally. Voice input, TTS responses, screen activity — all captured.

### 7. Stop recording
```bash
atomic-rec-stop
```
Stops OBS (finalizes .mov), stops audio recorder, moves video into session folder.

### 8. Mux into final video
```bash
atomic-rec-mux
```
Combines screen video + audio WAV using the Vale-approved filter chain:
- Pan: 90% mic + 110% TTS (mono mix)
- Compressor: threshold 0.08, ratio 4:1
- Loudnorm: -14 LUFS target
- Volume: +10dB
- Limiter: 0.70 ceiling
- Resample: 48kHz mono AAC

Output: `~/Movies/AtomicSessions/<session>/final.mp4`

### 9. Play it
```bash
open ~/Movies/AtomicSessions/<session>/final.mp4
```

## Switching Back to Speakerphone

### 1. Unplug the USB headset (or select built-in in System Settings)

### 2. Switch mode
```bash
audio-mode speakerphone
```
This sets `PATCHBAY_VP=on`, `PATCHBAY_TTS_GAIN=2.5`, restarts the voice stack.

## Technical Details

### Sample Rate Handling
- VP ON (speakerphone): Apple VP upsamples mic to 96kHz. Swift worker downsamples to 48kHz.
- VP OFF (headset): Blackwire reports 8kHz (telephony rate). Swift worker upsamples to 48kHz.
- Both paths produce 48kHz f32 PCM on the stt_raw bus.

### TTS Gain
- Speakerphone: 2.5x (compensates for VP ducking on built-in speakers)
- Headset: 0.8x (headset earpiece needs less gain)

### Key Files
- `~/.local/bin/audio-mode` — mode switch script
- `~/.local/bin/fix-audio` — nuclear restart of voice stack
- `~/.local/bin/atomic-rec-start` — start recording session
- `~/.local/bin/atomic-rec-stop` — stop recording session
- `~/.local/bin/atomic-rec-mux` — mux audio + video
- `~/.local/bin/patchbay-worker` — Swift worker (handles both modes)
- `~/.atomic-audio-mode` — current mode file (speakerphone or headset)

### Golden Snapshot
`~/.local/bin/snapshots/working-2026-05-17/` — proven working binaries, scripts, and Swift source for both modes. If anything breaks:
```bash
cp ~/.local/bin/snapshots/working-2026-05-17/patchbay-worker ~/.local/bin/patchbay-worker
```

### Rogue Amoeba Cleanup (2026-05-17)
Removed Audio Hijack, Loopback, ARK.driver, arkaudiod — third-party audio stack that was intercepting CoreAudio. All remnants deleted. If audio ever goes silent for no reason, check for rogue audio daemons:
```bash
ps -axo pid,command | grep -Ei 'arkaudiod|ARK.driver|Audio Hijack|Loopback|SoundSource' | grep -v grep
```

## Supported Headsets
Any USB or Bluetooth headset with physical mic/earpiece isolation:
- Poly Blackwire 3210 (USB, tested and proven)
- Poly Legend 50 (Bluetooth)
- BlueParrot B250-XTS (Bluetooth)

The device is auto-detected as the default input. Just select it in System Settings before switching to headset mode.
