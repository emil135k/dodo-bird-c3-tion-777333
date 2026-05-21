# Atomic Ant Stack Current Architecture

Date: 2026-05-21

## Purpose

This document captures the current state of the Atomic Ant stack after the BEAM supervision work, OBS integration work, and the successful GStreamer OBS mirror timing fix.

The important architectural lesson is this:

```text
BEAM supervises.
Rust and Swift do realtime work.
iceoryx2 moves ant data.
GStreamer handles media timing.
OBS, browsers, and other tools are consumers.
```

The stack is moving away from fragile shell-pipe behavior and toward explicit, observable, stateful ants.

## Current High-Level Shape

```text
launchd
  -> BEAM / Elixir supervisor
    -> core Rust and Swift ants
      -> iceoryx2 shared-memory backplane
        -> realtime voice path
        -> text routing path
        -> OBS / media mirror path
        -> future browser / Twilio / drone media paths
```

The system is intentionally layered:

```text
Process lifecycle:
  launchd + BEAM

Realtime audio capture/playback:
  patchbay-ant + Swift AVAudioEngine worker

Fast ant-to-ant transport:
  iceoryx2

Speech boundary detection:
  silero-ant

Speech recognition:
  stt-ant + parakeet-worker

Text routing:
  router-ant

Console injection:
  type-ant

TTS:
  tts-ant

OBS/media mirror:
  obs-mirror-ant

OBS command/control:
  obs-ctl-ant

Mode coordination:
  ant-mode + Hammerspoon + router/type/browser control endpoints
```

## BEAM Role

BEAM is a supervisor, not the realtime audio engine.

Current rule:

```text
launchd starts BEAM.
BEAM supervises the ants.
Ants remain independent OS processes.
Rust/Swift handle realtime work.
iceoryx2 carries fast data.
```

BEAM does not use NIFs for the realtime Rust audio path. This is intentional.

Why no NIFs in this phase:

```text
NIF panic or memory misuse can take down the BEAM VM.
External ants can crash independently.
BEAM can restart external ants without corrupting its own scheduler.
Realtime data stays outside BEAM message passing.
```

Current core supervised catalog:

```text
speaker-ctl-ant
tts-ant
stt-ant
silero-ant
patchbay-ant
router-ant
type-ant
```

Optional or adjacent ants:

```text
obs-mirror-ant
obs-ctl-ant
browser-ctl-ant
future media-bridge-ant
future Twilio/browser bridge ants
future drone/video/telemetry ants
```

## iceoryx2 Role

iceoryx2 is the sovereign ant backplane.

It is not the clock owner for every media boundary. It is the high-speed local transport layer.

Current root path:

```text
/tmp/iceoryx2/
```

Important topics involved in the current audio/media path:

```text
stt_raw
  mic audio frames
  48 kHz mono f32 bytes
  published by patchbay-ant from the Swift worker capture path

tts_audio
  TTS audio frames
  24 kHz mono f32 bytes
  published by tts-ant
  consumed by patchbay-ant for playback
  consumed by obs-mirror-ant for recording mirror
```

Contract checking currently performed in `obs-mirror-ant`:

```text
payload length must be divisible by 4
payload is interpreted as little-endian f32 samples
stt_raw is treated as 48 kHz mono
tts_audio is treated as 24 kHz mono and upsampled to 48 kHz by duplicating samples
```

Current subscriber behavior:

```text
raw_sub.receive()
  -> append f32 samples to mic buffer
  -> increment mic_frames

tts_sub.receive()
  -> append each f32 sample twice to TTS buffer
  -> increment tts_frames
```

Bounded buffering:

```text
MAX_BUFFER_SAMPLES = 48000 * 10
```

Each buffer is a bounded `VecDeque<f32>`. If a buffer is full, the oldest sample is dropped before the new one is pushed.

This prevents unbounded memory growth if an output consumer stalls.

## Realtime Voice Path

```text
Mac mic / Blackwire headset
  -> patchbay-ant
    -> Swift AVAudioEngine worker
      -> frame capture
      -> patchbay-ant Rust reader
        -> iceoryx2 stt_raw
          -> silero-ant
            -> stt-ant / parakeet-worker
              -> router-ant
                -> type-ant or alternate route
```

Playback path:

```text
tts-ant
  -> iceoryx2 tts_audio
    -> patchbay-ant
      -> Swift AVAudioEngine worker
        -> Mac speakers / Blackwire headset
```

OBS mirror path:

```text
stt_raw + tts_audio
  -> obs-mirror-ant
    -> CPAL backend or GStreamer backend
      -> BlackHole 2ch
        -> OBS audio input source
```

## Why The Original OBS Path Was Fragile

The earlier OBS/BlackHole path worked, but it had too many implicit state dependencies:

```text
OBS source visibility
OBS hidden source mute state
BlackHole device state
CoreAudio reset behavior
CPAL stream survival
OBS recording start/stop behavior
ant stack process state
which process owns the BlackHole writer
```

The path was functional, but not yet hardened. It could record successfully one moment and fail after a routing or state change.

The GStreamer work exists to move from “functional if all state stays aligned” to “media pipeline with explicit buffering and clock handling.”

## OBS Mirror Ant: Current Design

Current source location for the GStreamer prototype:

```text
/Users/rocketman/crystalballmini-obs-mirror/hypAiAssist/ants/obs-mirror-ant
```

Current installed CPAL binary remains separate and is not replaced by this prototype unless explicitly promoted.

Environment controls:

```bash
OBS_MIRROR_BACKEND=cpal        # default lower clip
OBS_MIRROR_BACKEND=gstreamer   # current prototype higher clip

OBS_MIRROR_DEVICE="BlackHole 2ch"
OBS_MIRROR_LAYOUT=dual-mono
OBS_MIRROR_GAIN=1.0
OBS_MIRROR_GST_DEVICE_UID=BlackHole2ch_UID   # optional override
```

Layouts:

```text
split:
  left channel  = mic / stt_raw
  right channel = TTS / tts_audio

dual-mono:
  left channel  = mic + TTS
  right channel = mic + TTS
```

`dual-mono` is currently OBS-safe because OBS can record either side and still contain the full audible session.

## CPAL Backend

The CPAL backend is the lower clip.

It writes directly to a CoreAudio output device such as BlackHole 2ch.

It is still valuable because:

```text
it is simpler
it already proved useful
it is a fallback if GStreamer has a regression
it rebuilds the CPAL output stream if CoreAudio reports stream errors
```

CPAL callback behavior:

```text
CoreAudio asks for output frames.
The callback pops from mic_buf and tts_buf.
Missing mic sample increments underruns and writes zero.
Samples are mixed according to layout.
Gain is applied.
Samples are clipped to [-1.0, 1.0].
```

CPAL stream recovery:

```text
output stream error callback
  -> sets AtomicBool stream_broken
main loop sees stream_broken
  -> drops old stream
  -> finds output device again
  -> rebuilds stream
  -> resumes playback
```

## GStreamer Backend

The GStreamer backend is the higher clip.

Its job is to let GStreamer own media clocking and buffering while the Rust ant continues to own iceoryx2 subscriptions and system observability.

Current GStreamer pipeline:

```text
appsrc name=obs_mirror_src
  is-live=true
  format=time
  do-timestamp=true
  block=true
  max-bytes=384000
  caps=audio/x-raw,format=F32LE,layout=interleaved,rate=48000,channels=2
! queue max-size-time=500000000 max-size-buffers=0 max-size-bytes=0
! audioconvert
! audioresample
! osxaudiosink unique-id=BlackHole2ch_UID sync=true
```

Meaning of the important settings:

```text
is-live=true:
  tells GStreamer this is a live source, not a file.

format=time:
  buffers are interpreted on a time basis.

do-timestamp=true:
  GStreamer timestamps buffers according to the live pipeline clock.

block=true:
  appsrc applies backpressure instead of silently dropping when downstream is busy.

max-bytes=384000:
  bounds appsrc internal buffering.
  384000 bytes is about 1 second of 48 kHz stereo f32 audio.

queue max-size-time=500000000:
  allows up to about 500 ms downstream queue.
  this gives the sink room to absorb small scheduling jitter.

non-leaky queue:
  do not drop audio buffers as a normal operating mode.

sync=true:
  osxaudiosink synchronizes to the pipeline/device clock.
```

## The Clock Boundary Problem

The successful fix came from recognizing separate clock domains:

```text
Domain 1:
  patchbay / iceoryx2 frame arrival

Domain 2:
  Rust bridge scheduling

Domain 3:
  GStreamer pipeline clock

Domain 4:
  CoreAudio / BlackHole device clock

Domain 5:
  OBS capture and encoder timing
```

The original naive GStreamer prototype crossed these domains too casually.

### Failed Shape 1: Unsynced Nonblocking Pipeline

Earlier pipeline behavior:

```text
appsrc block=false
queue leaky=downstream
osxaudiosink sync=false
manual sleep-based pusher
```

Observed behavior:

```text
audio existed
OBS recorded audio
but playback had crackles and Max Headroom-style artifacts
```

Diagnosis:

```text
buffers were allowed to drop
sink was not synchronized
manual sleep introduced drift
levels looked acceptable but time continuity was poor
```

### Failed Shape 2: Pure need-data Pull

Experiment:

```text
GStreamer appsrc need-data callback owns buffer requests
```

Observed behavior:

```text
output became mostly silence
underruns exploded
GStreamer pulled many buffers before iceoryx2 queues had payload
```

Diagnosis:

```text
GStreamer pull cadence did not match the current upstream ant buffering model.
This approach may work later with a proper primed jitter buffer, but it was wrong for this prototype stage.
```

### Successful Shape: Absolute Monotonic 10 ms Pusher

Current successful bridge:

```text
GST_CHUNK_FRAMES = 480
TARGET_RATE = 48000
chunk duration = 10 ms
```

The Rust output thread:

```text
sets next_tick = Instant::now() + 10 ms
for each loop:
  build exactly 480 stereo frames
  push one GstBuffer
  sleep until next absolute tick
  next_tick += 10 ms
```

The key improvement is that the sleep schedule is absolute, not relative.

Bad relative schedule:

```text
do work
sleep 10 ms
do work
sleep 10 ms
```

This drifts because work time is added on top of the sleep time.

Current absolute schedule:

```text
tick at T + 10 ms
tick at T + 20 ms
tick at T + 30 ms
```

This keeps the bridge aligned to a stable cadence.

## Buffer Construction

Each GStreamer output buffer contains:

```text
480 stereo frames
960 f32 samples
3840 bytes
10 ms of audio
```

For every frame:

```text
mic sample = mic_buf.pop_front() or 0.0
tts sample = tts_buf.pop_front() or 0.0
```

In `dual-mono`:

```text
mixed = mic + tts
left = mixed
right = mixed
```

In `split`:

```text
left = mic
right = tts
```

Then:

```text
sample *= OBS_MIRROR_GAIN
sample is clamped to [-1.0, 1.0]
clipped_samples increments if clamp occurred
```

## Handshaking Improvements

The important handshakes are not formal request/response messages yet. They are explicit contracts and observable readiness points.

### iceoryx2 Topic Handshake

`obs-mirror-ant` does not assume topics are magically there. It opens or creates:

```text
stt_raw
tts_audio
```

Then it logs:

```text
[OBS-MIRROR] Bus: sub='stt_raw' + 'tts_audio' -> BlackHole mirror - READY
```

This is the current readiness boundary between ant bus and mirror ant.

### Payload Contract Handshake

Before interpreting a payload as f32 samples:

```text
payload.len() % 4 == 0
```

If not, the ant screams:

```text
stt_raw contract violation
tts_audio contract violation
```

This prevents silent reinterpretation of malformed audio.

### Backend Handshake

The ant announces:

```text
Output backend: cpal
```

or:

```text
Output backend: gstreamer
```

This matters because CPAL and GStreamer have different timing behavior and different certification status.

### GStreamer Pipeline Handshake

The ant logs the exact pipeline string:

```text
[OBS-MIRROR] GStreamer pipeline: appsrc ... ! osxaudiosink ...
```

Then:

```text
[OBS-MIRROR] Output stream READY: 48000Hz stereo, ..., backend=gstreamer
```

This is the current boundary between Rust and GStreamer.

### Runtime Health Handshake

Every 5 seconds:

```text
mic_frames
tts_frames
mic_buf samples
tts_buf samples
underruns
clipped
output_buffers
```

These counters are now the practical health interface.

Interpretation:

```text
mic_frames increasing:
  stt_raw is flowing.

tts_frames increasing:
  tts_audio is flowing.

output_buffers increasing:
  backend is actively producing audio.

underruns stable after startup:
  acceptable.

underruns growing continuously:
  output clock is outrunning available data.

clipped > 0:
  gain or mixing is too hot.

mic_buf/tts_buf growing without bound:
  output side is too slow or stalled.

mic_buf/tts_buf pinned at zero while input frames grow:
  output side may be too aggressive.
```

## Successful GStreamer + OBS Test

The smooth successful test was produced after the absolute 10 ms pusher fix.

Recording:

```text
/Users/rocketman/Movies/2026-05-21 14-33-13.mov
```

ffprobe:

```text
video:
  codec: h264
  size: 3024x1964
  fps: 30
  duration: 11.966667s

audio:
  codec: aac
  channels: 2
  sample_rate: 48000
  duration: 11.924000s
```

Audio measured from the actual recorded file:

```text
left peak/rms  0.458085 0.038453
right peak/rms 0.458085 0.038453
```

Mirror log:

```text
backend=gstreamer
layout=dual-mono
mic_frames=151
tts_frames=2
underruns=480
clipped=0
output_buffers=1500
mic_buf stable around 5280 samples
```

User listening result:

```text
smooth as butter
```

Interpretation:

```text
The crackles were caused by bad media timing settings.
The pauses/skips were caused by bridge cadence mismatch.
The absolute 10 ms pusher made the GStreamer/BlackHole/OBS path smooth.
```

## Current Recommended GStreamer Mirror Settings

For the current prototype:

```bash
OBS_MIRROR_BACKEND=gstreamer
OBS_MIRROR_DEVICE="BlackHole 2ch"
OBS_MIRROR_LAYOUT=dual-mono
OBS_MIRROR_GAIN=1.0
```

`OBS_MIRROR_GAIN=1.0` is the current clean baseline because it avoided clipping in the successful tests.

## Remaining Risks

This is not production-certified yet.

Remaining risks:

```text
long recordings not yet proven
live Emil mic + TTS together still needs a deliberate test
OBS start/stop loops need repeated testing
OBS restart while backend is running needs testing
BlackHole/CoreAudio reset behavior under GStreamer needs testing
browser voice mode through this backend is not certified
obs-gstreamer plugin is not certified on this Mac
```

## Future Architecture Direction

The likely future is to generalize `obs-mirror-ant` into a broader media bridge:

```text
media-bridge-ant / gst-media-ant
  -> subscribe to iceoryx2 media topics
  -> publish or route through GStreamer pipelines
  -> feed OBS, browser voice, recorders, network streams, or drone telemetry consumers
```

BlackHole should eventually become optional compatibility glue, not the center of the architecture.

Preferred future:

```text
iceoryx2 topics
  -> GStreamer media bridge
    -> OBS direct GStreamer source, if stable
    -> recorder
    -> browser bridge
    -> network stream
    -> drone video/telemetry path
```

Fallback compatibility:

```text
GStreamer media bridge
  -> BlackHole
    -> OBS / browser / legacy CoreAudio consumers
```

## Design Principle

Keep the lower clip secure before reaching for the higher clip.

Current lower clip:

```text
CPAL obs-mirror-ant path
```

Current higher clip:

```text
GStreamer obs-mirror-ant backend
```

Do not delete the lower clip. Promote the higher clip only after repeated tests prove it under real use.

## Summary

The Atomic Ant stack is now moving toward a clean separation of concerns:

```text
BEAM:
  lifecycle and supervision

Rust/Swift ants:
  realtime edges and strict contracts

iceoryx2:
  sovereign local data movement

GStreamer:
  media clocking, buffering, conversion, and future streaming

OBS/browser/Twilio/drone systems:
  consumers or external interfaces, not owners of the core nervous system
```

The GStreamer OBS mirror test proved that this direction is viable. The key was not simply adding GStreamer. The key was respecting the clock and buffering boundaries between iceoryx2, Rust, GStreamer, CoreAudio/BlackHole, and OBS.

