# OBS Integration Architecture

## Purpose

This document records the OBS recording integration work for the atomic ant stack. The goal was to make OBS a reliable recorder for voice sessions without forcing OBS to own the realtime audio path.

The design principle is:

- ants own the realtime capture and playback path
- OBS consumes a mirrored recording path
- playback gain is controlled in one place, not scattered across scripts and worker code

## Architecture Flow

### Realtime voice path

```text
Mac mic / headset
  -> patchbay-ant / Swift worker
  -> stt_raw
  -> silero-ant / stt-ant / tts-ant
  -> patchbay-ant playback
  -> speakers or headset
```

### OBS recording path

```text
stt_raw and/or tts_audio
  -> obs-mirror-ant
  -> BlackHole virtual device
  -> OBS audio source
  -> OBS recording
```

OBS should observe the audio, not negotiate the hardware path. That keeps the capture side stable and makes the recording layer optional.

## What Was Improved

### 1. Speakerphone and headset modes are explicit

The stack now has two supported operating modes:

- speakerphone mode
  - Voice Processing enabled
  - built-in Mac mic and speakers
  - suitable for normal desk use

- headset mode
  - Voice Processing disabled
  - USB headset / Blackwire
  - suitable for clean mic capture without Apple DSP

### 2. Playback gain was normalized

The loudness issue came from hardcoded gain values being baked into the launch path. That made playback too hot during speakerphone testing.

The gain configuration was moved into a single file:

```text
~/.atomic-audio.conf
```

Current values:

```bash
AUDIO_TTS_GAIN_SPEAKERPHONE=1.0
AUDIO_TTS_GAIN_HEADSET=0.8
```

This keeps the defaults in one place and avoids scattering magic numbers through launch scripts and worker code.

### 3. Launch scripts now read config instead of hardcoding

The following tools were updated to consume the shared gain config:

- `~/.local/bin/fix-audio`
- `~/.local/bin/audio-mode`
- `patchbay-ant` Swift worker default gain path

The launcher now determines the active mode, loads the shared config, and passes the right gain into patchbay at startup.

## Atomic Audio Config

The audio config file is intended to be the single source of truth for loudness defaults.

```bash
# ~/.atomic-audio.conf
AUDIO_TTS_GAIN_SPEAKERPHONE=1.0
AUDIO_TTS_GAIN_HEADSET=0.8
```

Why this matters:

- prevents ad hoc hardcoded gains
- keeps speakerphone and headset behavior predictable
- makes future tuning a config edit instead of a code edit

## Functional Tests Performed

### OBS direct capture tests

Verified that OBS can record voice directly:

- mic-only scene recorded successfully
- mic + desktop scene recorded successfully
- audio levels were present in the resulting files

Observed file examples:

- `2026-05-19 22-51-13.mov`
  - mic-only recording
  - non-silent

- `2026-05-19 22-47-44.mov`
  - mic + desktop recording
  - non-silent

### BlackHole mirror tests

Verified that the BlackHole path is functional in isolation:

- created a clean BlackHole-only OBS scene
- recorded through `BlackHole 2ch`
- confirmed the recording contained real audio

Observed file example:

- `2026-05-19 22-54-40.mov`
  - BlackHole-only recording
  - non-silent

### Speakerphone mode tests

Verified the live voice stack in speakerphone mode:

- `audio-mode speakerphone`
- Voice Processing ON
- built-in mic and speakers selected
- patchbay restarted cleanly
- playback gain reported as `1.0x`

### Headset mode tests

Verified the live voice stack in headset mode:

- `audio-mode headset`
- Voice Processing OFF
- Blackwire headset selected
- patchbay restarted cleanly
- headset-safe playback gain used

## OBS Integration Notes

The OBS work confirmed two separate truths:

1. OBS can record voice correctly when fed a sane source.
2. The earlier silence problem was not a general OBS failure; it was a source/routing/config interaction.

That led to the cleaner approach:

- keep the ants as the source of truth
- mirror audio into BlackHole for OBS
- avoid Apple Audio MIDI multi-output complexity
- keep OBS as a consumer, not a participant in the realtime chain

## Practical Result

Current state:

- realtime voice path remains intact
- OBS direct mic capture works
- BlackHole mirror path works in isolation
- playback loudness is now controlled from one config file

## Follow-Up Work

The remaining work is integration cleanup, not foundational repair:

- fold the working BlackHole scene back into the normal OBS scene
- remove diagnostic duplicates once the final scene is settled
- keep `~/.atomic-audio.conf` as the single place for playback defaults
- if volume needs more refinement, adjust config values only

## Rule Going Forward

Do not hardcode gain values in worker code or launcher scripts unless there is no alternative. Put tunable audio behavior in the shared config file first.
