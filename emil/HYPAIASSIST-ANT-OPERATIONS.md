# hypAiAssist — Ant Operations Guide

## The Swarm

| Ant | Binary | Job |
|-----|--------|-----|
| TTS Ant | `tts-ant` | Kokoro v1.0 ONNX + misaki-rs → f32 audio via iceoryx2 |
| Mouth Ant | `mouth-ant` | Subscribes to audio bus → rodio → speakers |
| Pulse | `pulse` | CLI tool to send text to the swarm |

## Source Files

| File | Path |
|------|------|
| TTS Ant | `hypAiAssist/ants/tts-ant/src/main.rs` |
| Mouth Ant | `hypAiAssist/ants/mouth-ant/src/main.rs` |
| Pulse | `hypAiAssist/ants/pulse/src/main.rs` |
| Sovereign Manifest | `hypAiAssist/Sovereign.toml` |

## Installed Binaries

All at `/Users/rocketman/.local/bin/`:
- `tts-ant`
- `mouth-ant`
- `pulse`
- `larynx` (standalone TTS CLI)

## Starting the Swarm

```bash
# 1. Nuke stale shared memory (REQUIRED every restart)
rm -f /tmp/iox2_*.shm_state
rm -rf /tmp/iceoryx2

# 2. Start TTS Ant first (creates the iceoryx2 services)
tts-ant > /tmp/tts-ant-stdout.log 2>&1 &

# 3. Wait for Kokoro model to load on CoreML (~8 seconds cold start)
sleep 8

# 4. Start Mouth Ant (opens existing services)
mouth-ant > /tmp/sink-stdout.log 2>&1 &

# 5. Wait for connection
sleep 3

# 6. Test
pulse "The ants are alive"
```

## Stopping the Swarm

```bash
pkill -9 -f tts-ant
pkill -9 -f mouth-ant
```

## Monitoring

```bash
# Watch TTS Ant in real time
tail -f /tmp/tts-ant-stdout.log

# Watch Audio Sink in real time
tail -f /tmp/sink-stdout.log

# Both side by side (two terminals)
```

## Sending Speech

```bash
# Default voice (af_heart)
pulse "Hello Emil"

# Specific voice
pulse "af_sky:Hello with Sky voice"
```

## Data Flow

```
pulse (CLI)
  │ writes text bytes to shared memory
  ▼
[iceoryx2: tts_text]  ← POSIX shared memory, zero-copy
  │
  ▼
TTS Ant (daemon)
  │ misaki-rs G2P → Kokoro v1.0 ONNX CoreML → f32 samples
  │ writes audio to shared memory
  ▼
[iceoryx2: tts_audio]  ← POSIX shared memory, zero-copy
  │
  ▼
Mouth Ant (daemon)
  │ rodio SamplesBuffer → Mac speakers
  ▼
Your ears
```

## Troubleshooting

### Ants won't start
```bash
# Always nuke stale segments first
rm -f /tmp/iox2_*.shm_state
rm -rf /tmp/iceoryx2
```

### Audio Sink says "DoesNotExist"
TTS Ant must start FIRST — it creates the services. Audio Sink opens them.

### No sound but logs show "Playing"
Check audio output device:
```bash
SwitchAudioSource -c
```
Should say "MacBook Pro Speakers" or your headphones.

### TTS Ant shows no "GOT DATA"
The pulse message isn't reaching the subscriber. Restart both ants (nuke segments first).

## Architecture

- **IPC**: iceoryx2 v0.6.1, POSIX shared memory
- **Config**: Hardcoded root path `/tmp/iceoryx2/` in all binaries
- **TTS Model**: Kokoro v1.0 fp16, CoreML Neural Engine
- **G2P**: misaki-rs v0.3, pure Rust
- **Audio**: rodio, zero-disk, RAM-to-speaker
- **Voices**: Individual .bin files at `crystalballmini/voices-v1.0/`

## Zero Contamination

- Zero morsel
- Zero Python
- Zero sherpa
- Zero WAV files
- Zero FluidAudio
- Zero disk I/O for audio

---

*Built by Emil, Cody & Lyra — Sparked Matter LLC, April 28, 2026*
*Hooks Not Hacks. The hive is sovereign.*
