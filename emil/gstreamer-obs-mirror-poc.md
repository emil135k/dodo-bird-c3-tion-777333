# GStreamer OBS Mirror Proof of Concept

Date: 2026-05-20

## Purpose

This note captures the first GStreamer proof of concept for the Atomic Ant audio/video media path.

The goal is not to replace the working CPAL-based `obs-mirror-ant` yet. The goal is to add a higher clip on the mountain: prove that GStreamer can become the media timing and buffering layer while iceoryx2 remains the ant backplane and BEAM remains the supervisor.

## Architecture Direction

Current working lower clip:

```text
patchbay-ant / Swift worker
  -> iceoryx2 topics
       stt_raw   : mic audio, 48 kHz mono f32 bytes
       tts_audio : TTS audio, 24 kHz mono f32 bytes
  -> obs-mirror-ant using CPAL
  -> BlackHole 2ch
  -> OBS audio source
```

GStreamer proof path:

```text
patchbay-ant / Swift worker
  -> iceoryx2 topics
       stt_raw
       tts_audio
  -> obs-mirror-ant optional GStreamer backend
       Rust subscriber
       GStreamer appsrc
       queue / audioconvert / audioresample
       osxaudiosink unique-id=BlackHole2ch_UID
  -> BlackHole 2ch
  -> OBS / browser / recorder
```

Future media bridge direction:

```text
Atomic Ant Stack
  -> iceoryx2 sovereign backplane
  -> media-bridge-ant / gst-media-ant
  -> GStreamer pipelines
  -> one or more consumers:
       OBS
       browser voice mode
       local recorder
       network stream
       drone video / telemetry path
```

## Why GStreamer

GStreamer is built for media timing, clocking, buffering, resampling, conversion, and pipeline composition. Those are exactly the pieces that shell pipes and ad hoc audio routing do poorly.

For this project, GStreamer should handle media mechanics. It should not become the system brain.

Responsibilities:

```text
GStreamer:
  media clocking
  buffering
  resampling
  format conversion
  muxing/demuxing
  streaming protocols

iceoryx2:
  high-speed local ant-to-ant data movement
  shared-memory backplane
  structured internal topics

BEAM / Elixir:
  supervision
  restart policy
  state machine orchestration
  mode switching

Rust ants:
  realtime bridges
  contracts
  observability
  failure isolation
```

## Prototype Implementation

Prototype location:

```text
/Users/rocketman/crystalballmini-obs-mirror/hypAiAssist/ants/obs-mirror-ant
```

Added dependencies:

```toml
gstreamer = "0.25.2"
gstreamer-app = "0.25.2"
```

Added optional backend:

```bash
OBS_MIRROR_BACKEND=cpal        # default, current working lower clip
OBS_MIRROR_BACKEND=gstreamer   # proof backend
```

The GStreamer backend builds this pipeline:

```text
appsrc name=obs_mirror_src
  is-live=true
  format=time
  do-timestamp=true
  block=false
  caps=audio/x-raw,format=F32LE,layout=interleaved,rate=48000,channels=2
! queue max-size-time=200000000 leaky=downstream
! audioconvert
! audioresample
! osxaudiosink unique-id=BlackHole2ch_UID sync=false
```

The Rust ant still owns the iceoryx2 subscriptions. GStreamer only owns the media output path.

## Functional Proofs Performed

### 1. Raw Buffer To BlackHole

Generated a finite stereo f32 sine wave and piped it through GStreamer into BlackHole 2ch.

Result from ffmpeg readback:

```text
left peak/rms  0.129657 0.091645
right peak/rms 0.129657 0.091645
```

Conclusion: GStreamer can write valid audio into BlackHole 2ch on this machine.

### 2. Real TTS Topic To BlackHole

Started the prototype backend:

```bash
OBS_MIRROR_BACKEND=gstreamer \
OBS_MIRROR_DEVICE="BlackHole 2ch" \
OBS_MIRROR_LAYOUT=dual-mono \
OBS_MIRROR_GAIN=2.0 \
/Users/rocketman/crystalballmini-obs-mirror/hypAiAssist/ants/obs-mirror-ant/target/debug/obs-mirror-ant
```

Injected TTS through the real ant stack:

```bash
/Users/rocketman/.local/bin/inject-tts-vale \
  "GStreamer BlackHole mirror proof. Test one two three..."
```

Result from BlackHole readback:

```text
left peak/rms  0.648283 0.128664
right peak/rms 0.648283 0.128664
```

Mirror log:

```text
[OBS-MIRROR] Output backend: gstreamer
[OBS-MIRROR] Layout: dual-mono (left+right=mixed mic+tts)
[OBS-MIRROR] Bus: sub='stt_raw' + 'tts_audio' -> BlackHole mirror - READY
[OBS-MIRROR] Output stream READY: 48000Hz stereo, dual-mono, backend=gstreamer
[OBS-MIRROR] stats: mic_frames=52, tts_frames=1, mic_buf=55680 samples, tts_buf=470400 samples, underruns=0, clipped=185, output_buffers=404
```

Conclusion: real `tts_audio` from iceoryx2 can feed GStreamer `appsrc` and land in BlackHole.

### 3. Mic Topic To BlackHole

Started the same backend with higher gain and read BlackHole while live mic frames were flowing.

Result:

```text
left peak/rms  0.014125 0.001932
right peak/rms 0.014125 0.001932
```

Mirror log:

```text
mic_frames=101
tts_frames=0
underruns=0
output_buffers=798
```

Conclusion: `stt_raw` is reaching the GStreamer backend and producing output, but the live mic proof needs more deliberate testing before certification. The signal was present but low.

## Current Status

Certified:

```text
GStreamer installed and usable.
GStreamer can write to BlackHole 2ch.
GStreamer backend builds.
Real tts_audio -> GStreamer -> BlackHole works with strong levels.
Default CPAL path remains untouched.
```

Not certified yet:

```text
Live mic level through GStreamer backend.
Long-running OBS recording with backend=gstreamer.
OBS start/stop while GStreamer backend is active.
Browser voice mode through BlackHole using GStreamer backend.
obs-gstreamer plugin on Apple Silicon.
```

## OBS-GStreamer Plugin Assessment

The `obs-gstreamer` plugin may be useful later as an OBS-native GStreamer source. It should be treated as optional until proven on this Apple Silicon Mac and current OBS version.

Do not make OBS-GStreamer the center of the architecture yet.

Preferred posture:

```text
Keep obs-mirror-ant / media-bridge-ant as the ant-owned bridge.
Use GStreamer internally for media stability.
Use BlackHole as proven compatibility glue.
Test obs-gstreamer as an optional direct OBS input path.
```

## Design Principle

Do not delete the working lower clip.

```text
CPAL backend:
  current known fallback
  keep as default

GStreamer backend:
  higher clip
  explicit env flag
  promote only after real OBS/browser stability tests
```

## Next Test Plan

1. Run GStreamer backend for 5-10 minutes with OBS open.
2. Record a session with:
   - Emil mic
   - Vale/Cody TTS
   - desktop capture
3. Verify output file with ffmpeg:
   - duration
   - audio stream count
   - peaks/RMS
   - no silent segments during speech/TTS
4. Start and stop OBS recording multiple times while backend runs.
5. Watch GStreamer mirror logs:
   - `mic_frames`
   - `tts_frames`
   - `underruns`
   - `clipped`
   - `output_buffers`
6. Only after that, consider promoting GStreamer backend into the installed ant binary.

## Drone / Telemetry Connection

This same shape scales to the future drone work:

```text
camera / microphone / telemetry source
  -> GStreamer pipeline
  -> media-bridge-ant
  -> iceoryx2 topics
  -> analysis ants / recorders / streamers / UI
```

The lesson is the same: GStreamer should own media mechanics, iceoryx2 should own ant data movement, and BEAM should own orchestration.

## 2026-05-21 Follow-Up Tests

### Repeatable Test Harness

Added a worktree-local test script:

```text
/Users/rocketman/crystalballmini-obs-mirror/hypAiAssist/scripts/obs-mirror/test-gstreamer-blackhole.sh
```

Purpose:

```text
Start obs-mirror-ant with OBS_MIRROR_BACKEND=gstreamer.
Resolve the current AVFoundation index for BlackHole 2ch.
Inject two Vale TTS phrases through the real ant stack.
Capture BlackHole readback with ffmpeg.
Print peak/RMS measurements and mirror log tail.
```

Important: this script runs the worktree debug binary. It does not replace the installed CPAL mirror binary.

### BlackHole Readback Baseline

Command:

```bash
OBS_MIRROR_GAIN=1.0 \
/Users/rocketman/crystalballmini-obs-mirror/hypAiAssist/scripts/obs-mirror/test-gstreamer-blackhole.sh
```

Result:

```text
left peak/rms  0.583453 0.059972
right peak/rms 0.583453 0.059972
```

Mirror log:

```text
backend=gstreamer
layout=dual-mono
tts_frames=2
clipped=0
```

Startup note:

```text
underruns=480
```

Those underruns appeared at startup before the queues had payload, then stayed flat across later stats. Treat this as a startup priming artifact unless it grows during steady-state operation.

### OBS Recording Proof

OBS was initially unavailable on the WebSocket:

```text
obs-ctl-ant state=RECONNECTING
Reconnect failed: unable to connect to ws://127.0.0.1:4455
```

After launching OBS, `obs-ctl-ant` reconnected and returned:

```json
{"state":"READY","recording":false,"scene":"Scene","reconnect_count":1}
```

Then the GStreamer backend was started and OBS recording was controlled through `obs-ctl-ant`.

Recording output:

```text
/Users/rocketman/Movies/2026-05-21 14-23-43.mov
```

ffprobe result:

```text
video: h264, 3024x1964, 30 fps, duration 12.033333s
audio: aac, stereo, 48000 Hz, duration 11.988000s
container size: 9254906 bytes
```

Audio measured from the actual recorded file:

```text
left peak/rms  0.456939 0.037593
right peak/rms 0.456939 0.037593
```

Mirror log during OBS recording:

```text
backend=gstreamer
layout=dual-mono
mic_frames=151
tts_frames=2
underruns=0
clipped=0
output_buffers=1200
```

Conclusion:

```text
First OBS file-level proof passed.
GStreamer backend produced usable audio in the actual OBS recording.
Video and audio were both present in the .mov.
No clipping and no steady-state underruns were reported during the recording.
```

Remaining certification work:

```text
Longer recording run.
Live Emil mic plus Vale/Cody TTS in the same recording.
OBS start/stop repeated several times while mirror stays running.
CoreAudio reset / BlackHole stream break behavior under GStreamer backend.
Browser voice mode through GStreamer-backed BlackHole path.
```
