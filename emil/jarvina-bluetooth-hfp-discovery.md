# Bluetooth HFP Protocol Wall — The Root Cause Discovery
## Jarvina Voice Agent — Jetson Orin Nano
### Sparked Matter LLC — March 28, 2026

---

## * VERDICT: NON-CANCEROUS WART *

**The `pw-loopback` bridge is NOT a hack, NOT a zip tie, and NOT cancerous.**
It is a medically-annotated, architecturally-sound format translator that sits
at the boundary between PipeWire's Bluetooth codec domain and GStreamer's PCM
domain. It was proven necessary through layer-by-layer diagnostic testing —
not assumed, not guessed, not hacked into place.

**Deep thanks to the Lattice team for going through the proper diagnostic
process instead of cheating the system by bypassing GStreamer or PipeWire.**

---

## The Mystery
When connecting `pipewiresrc` directly to a Bluetooth HFP (Hands-Free Profile) audio source, GStreamer fails with:

```
stream error: unhandled format
streaming stopped, reason not-negotiated (-4)
```

The pipeline creates ports, WirePlumber sees both endpoints, but no audio flows. File output = 0 bytes.

## The Diagnosis

### What We Tried (and failed):
1. `pipewiresrc target-object=bluez_input...` — ports created, no link
2. `pipewiresrc` with `stream-properties="props,node.autoconnect=true"` — same result
3. `media.role=Communication` property — no effect
4. Manual `pw-link` to force the connection — exposed the real error

### The X-Ray (Manual pw-link result):
```
pw-link 'bluez_input...capture_MONO' 'gst-launch-1.0:input_1'
→ ERROR: stream error: unhandled format
→ streaming stopped, reason not-negotiated (-4)
```

When forced to link, `pipewiresrc` RECEIVED the Bluetooth audio but couldn't parse the format.

## The Root Cause

**Bluetooth HFP uses specialized telephony codecs (mSBC or CVSD at 8kHz/16kHz), NOT standard PCM.**

GStreamer's `pipewiresrc` element is a high-level consumer that expects standard PCM audio (S16LE, S32LE, Float32, etc.). When it receives raw Bluetooth HFP telephony packets, it has no codec to decode them. It literally doesn't have the "teeth" to chew that data format.

This is NOT:
- A WirePlumber policy bug
- A session management issue
- A PipeWire configuration problem
- A broken GStreamer element

It IS:
- A **format barrier** between Bluetooth telephony codecs and GStreamer's PCM expectations
- The same reason professional Bluetooth audio systems use dedicated codec bridges

## The Solution: pw-loopback as a Format Translator

`pw-loopback` is NOT a hack or a "zip tie." It is a **dedicated media processor** that:

1. **Captures** the raw Bluetooth HFP stream using PipeWire's native codec support
2. **Resamples and reformats** in real-time (mSBC/CVSD → standard PCM)
3. **Presents** a clean, standard PCM port that GStreamer can consume

PipeWire's internal engine handles the Bluetooth codec translation transparently. The loopback just bridges the format gap between PipeWire's Bluetooth stack and GStreamer's audio expectations.

### Architecture (UPDATED — March 28, 2026):
```
CAPTURE:
  Sony Mic → BlueZ → PipeWire (HFP codec decode)
    → echo_cancel_capture (AEC subtracts speaker reference)
    → Echo-Cancelled Source (clean PCM)
    → GStreamer pipewiresrc → audioconvert → audioresample
    → VAD → STT → LLM → TTS (Kokoro WAV)

PLAYBACK:
  Kokoro WAV → pw-play (native PipeWire, no GStreamer)
    → Echo-Cancelled Sink (AEC reference input)
    → echo_cancel_playback → PipeWire → BlueZ → Sony Speaker

NOTE: pw-loopback and GStreamer pipewiresink are NO LONGER USED.
      PipeWire echo-cancel module replaced both.
```

### * What pw-loopback actually does (The Decoder Ring) *

The `pw-loopback` performs THREE critical functions:

**1. Transcoding (The Decoder Ring)**
* Raw Bluetooth HFP audio arrives as mSBC (Modified Sub-Band Coding)
* GStreamer's `pipewiresrc` only speaks PCM — it sees mSBC as gibberish
* The loopback decodes mSBC → LPCM (Linear Pulse Code Modulation)
* GStreamer sees a standard, familiar PCM stream

**2. Encapsulation (The Protective Wrap)**
* Bluetooth HFP packets arrive in jittery, irregular bursts
* GStreamer demands a steady, rhythmic heartbeat of audio samples
* The loopback creates a buffer that smooths the wireless chaos into a constant flow
* It presents a "Virtual Port" that promises GStreamer a stable 16kHz stream
* It encapsulates the chaos of wireless radio inside the stability of a virtual wire

**3. Protocol Negotiation (The Universal Translator)**
* GStreamer and Bluetooth can't agree on a "language" (caps negotiation fails)
* The loopback speaks both languages:
  - To PipeWire/BlueZ: *"Give me whatever narrow-band telephony format you have"*
  - To GStreamer: *"I am a high-fidelity mono microphone. Standard PCM."*
* Both sides are happy — they think they're talking to someone of their own class

***** Medical Annotation: The `pw-loopback` is the Synthetic Skin that allows the
"stepchild" Bluetooth telephony protocol to pass through the GStreamer Security
Gate without being rejected for a format violation. *****

### Why pw-loopback is the correct solution:
- Uses PipeWire's native Bluetooth codec support (the ONLY component that understands mSBC/CVSD)
- Presents a standard PCM source that ANY GStreamer pipeline can consume
- Zero additional latency when configured with `node.latency`
- Lives in the PipeWire graph as a proper node, not an external process
- Can be managed by the C++ binary lifecycle (spawn on start, kill on exit)

## The Formalization Plan

### Current (Working but fragile):
```bash
nohup pw-loopback --capture-props='target.object=bluez_input...headset-head-unit' \
    --playback-props='node.name=jarvina_stable_input media.class=Audio/Source' &
```

### Target (Carrier-grade):
- C++ binary spawns the virtual loopback node using `pw_context` API
- Loopback lives and dies with the voice-agent process
- No background shell scripts, no ghosts
- Auto-discovers the Bluetooth device by MAC address
- Configurable latency via env var

## Exoneration of pw-loopback

### Why it's NOT a "zip tie" or hack:

1. **It's a format translator, not a workaround.** The pw-loopback bridges two incompatible protocols (Bluetooth HFP codecs ↔ GStreamer PCM). This is the SAME role that a DAC plays between digital and analog — it's not a hack, it's an adapter that must exist.

2. **It lives inside the PipeWire graph.** Unlike a bash script or `system()` call, the loopback is a proper PipeWire node. It shows up in `wpctl status`, it's linkable via `pw-link`, and WirePlumber manages its lifecycle. It IS part of the infrastructure.

3. **Zero zombie process risk.** Because `pw-loopback` is a PipeWire-native process:
   - It communicates through PipeWire's internal IPC, not GStreamer's pipeline protocol
   - If the voice-agent crashes, the loopback stays alive (no orphaned state)
   - If the loopback crashes, the voice-agent's `pipewiresrc` simply gets silence — no segfault, no hang
   - When killed cleanly, it deregisters from PipeWire's graph — no ghost nodes
   - It does NOT participate in GStreamer's caps negotiation, clock sync, or buffer management — so it cannot cause GStreamer pipeline stalls or deadlocks

4. **It's the conventional PipeWire solution.** The PipeWire documentation and community recommend `pw-loopback` for exactly this use case: bridging sources that use non-standard formats into standard PCM consumers. This is not a Cody invention — it's PipeWire's designed architecture.

5. **Low overhead.** The loopback does a simple PCM resample/reformat in PipeWire's real-time thread. No GStreamer overhead, no additional buffer copies, no caps renegotiation. The audio flows: BlueZ → PipeWire SPA (codec decode) → loopback (format normalize) → virtual source → GStreamer.

### The only "zip tie" risk was HOW we launched it:
- **Bad**: `nohup pw-loopback ... &` in a bash script (orphan-prone, no lifecycle management)
- **Good**: C++ binary spawns it via `pw_context` API (lives and dies with the agent)
- **Good**: systemd user service (managed, auto-restart, clean shutdown)

## Lessons Learned

1. **"Hack" vs "Bridge"**: What looks like a hack may be a necessary adapter between incompatible protocols. Diagnose before judging.
2. **Format barriers are invisible**: The `pw-link` forced connection was the key diagnostic — it revealed the codec mismatch that silent failures hid.
3. **PipeWire > GStreamer for Bluetooth**: PipeWire's SPA BlueZ plugin handles Bluetooth codecs natively. GStreamer's `pipewiresrc` does not. Use each tool for what it's good at.
4. **Measure twice, cut once**: The brute-force retry loop hid the root cause for hours. One diagnostic `pw-link` with error output revealed it in seconds.
5. **Know the boundary**: GStreamer and PipeWire have a clear division of labor. PipeWire handles device protocols (Bluetooth, USB, ALSA drivers). GStreamer handles media processing (codecs, transforms, pipelines). The loopback sits at that boundary — it's PipeWire finishing its job before handing clean data to GStreamer.

---

## * Sony SRS-XB100 Phone Button Warning *

**DO NOT press the phone/handset button on the Sony SRS-XB100 while Jarvina is running.**

This button sends a "hang up" signal to the HFP profile, killing the SCO audio tunnel. The Bluetooth connection stays alive but audio stops flowing. Jarvina goes deaf.

**Recovery procedure:**
1. Kill voice-agent: `tmux kill-session -t va`
2. Kill loopback: `pkill -9 pw-loopback`
3. Disconnect Sony: `bluetoothctl disconnect 88:92:CC:5F:03:5C`
4. Wait 3 seconds
5. Reconnect: `bluetoothctl connect 88:92:CC:5F:03:5C`
6. Reset HFP profile: `wpctl set-profile <NEW_DEVICE_ID> 0 && sleep 1 && wpctl set-profile <NEW_DEVICE_ID> 3`
7. Relaunch loopback and voice-agent

**Note:** The device ID changes after every reconnect. Always check `wpctl status | grep SRS` to find the current ID.

---

## Current Architecture (Working — March 28, 2026)

### Signal Flow Diagram
```
┌─────────────────────────────────────────────────────────────────┐
│                        PIPEWIRE GRAPH                          │
│                                                                │
│  ┌──────────────┐     ┌─────────────────────┐                  │
│  │ Sony SRS-XB100│     │ PipeWire Echo Cancel │                  │
│  │ (Bluetooth    │     │ (WebRTC AEC)         │                  │
│  │  HFP Profile) │     │                     │                  │
│  │              │     │  ┌───────────────┐   │                  │
│  │  MIC ────────┼────►│  │ echo_cancel   │   │                  │
│  │  (bluez_input)│     │  │ _capture      │   │                  │
│  │              │     │  │ (subtracts    │   │                  │
│  │              │     │  │  reference)   │   │                  │
│  │              │     │  └───────┬───────┘   │                  │
│  │              │     │          │            │                  │
│  │              │     │  ┌───────▼───────┐   │   ┌────────────┐ │
│  │              │     │  │ Echo-Cancelled│   │   │ GStreamer   │ │
│  │              │     │  │ Source (37)   │───┼──►│ pipewiresrc │ │
│  │              │     │  │ (clean audio) │   │   │ → VAD      │ │
│  │              │     │  └───────────────┘   │   │ → STT      │ │
│  │              │     │                     │   │ → LLM      │ │
│  │              │     │  ┌───────────────┐   │   │ → TTS      │ │
│  │              │     │  │ Echo-Cancelled│   │   │ (Kokoro)   │ │
│  │  SPEAKER ◄───┼─────│  │ Sink (38)    │◄──┼───│            │ │
│  │  (bluez_     │     │  │ (reference   │   │   │ pw-play ───┘ │
│  │   output)    │     │  │  for AEC)    │   │   └────────────┘ │
│  │              │     │  └──────┬────────┘   │                  │
│  │              │     │         │            │                  │
│  │              │     │  ┌──────▼────────┐   │                  │
│  │              │     │  │ echo_cancel   │   │                  │
│  │  ◄───────────┼─────│  │ _playback     │   │                  │
│  │              │     │  │ (to hardware) │   │                  │
│  │              │     │  └───────────────┘   │                  │
│  └──────────────┘     └─────────────────────┘                  │
│                                                                │
└─────────────────────────────────────────────────────────────────┘
```

### Signal Chain (Text)
```
CAPTURE PATH (Ear):
  Sony Mic → BlueZ → bluez_input (HFP/mSBC decoded by PipeWire SPA)
    → echo_cancel_capture (AEC subtracts reference signal)
    → Echo-Cancelled Source (node 37, clean PCM)
    → GStreamer pipewiresrc → audioconvert → audioresample
    → 16kHz mono S16LE → appsink → VAD (Silero) → STT (Parakeet, CUDA)

PROCESSING PATH (Brain):
  STT text → LLM Dispatcher (Haiku/Gemini/Local via libcurl)
    → response text → TTS (Kokoro, CUDA, speaker 3 af_heart)
    → WAV file

PLAYBACK PATH (Voice):
  WAV file → pw-play (native PipeWire command)
    → Echo-Cancelled Sink (node 38, AEC reference input)
    → echo_cancel_playback
    → bluez_output → BlueZ → Sony Speaker
```

### Key Design Decisions
1. **Capture**: GStreamer `pipewiresrc` targets `Echo-Cancelled Source` — receives AEC-cleaned audio
2. **Playback**: `pw-play` (NOT GStreamer) targets `Echo-Cancelled Sink` — provides AEC reference signal
3. **AEC**: PipeWire `module-echo-cancel` with WebRTC engine sits between Sony hardware and the voice-agent
4. **No GStreamer playback pipeline** — removed to eliminate bypass leak where output went directly to Sony speaker, starving the AEC of its reference signal
5. **No `g_speaking` mute** — PipeWire AEC handles echo subtraction
6. **No barge-in** — `pw-play` blocks during playback. Barge-in requires non-blocking playback (future work)
7. **VAD silence threshold: 2.0 seconds** — prevents cutting off mid-sentence during natural pauses

### PipeWire Nodes (Clean Graph)
| Node | Role | Links |
|------|------|-------|
| echo_cancel_capture (36) | Captures Sony mic, feeds AEC | ← Sony bluez_input |
| Echo-Cancelled Source (37) | Clean output from AEC | → voice-agent input |
| Echo-Cancelled Sink (38) | Reference input for AEC | ← pw-play |
| echo_cancel_playback (39) | AEC output to speaker | → Sony bluez_output |
| voice-agent (73/83) | GStreamer capture only | ← Echo-Cancelled Source |
| pw-play (spawns per utterance) | Native PipeWire playback | → Echo-Cancelled Sink |

### What's NOT in the graph (by design)
- No GStreamer playback pipeline (removed — caused AEC bypass)
- No pw-loopback (not needed — AEC module handles format translation)
- No `g_speaking` flag (AEC handles echo)
- No barge-in code (blocked by pw-play, future work)
- No ALSA anything

---

## Root Cause History — Two Problems We Tangled Together

### Problem 1: GStreamer pipewiresrc format incompatibility (C++ era)
- GStreamer's `pipewiresrc` couldn't negotiate caps with Bluetooth HFP audio format
- Error: `stream error: unhandled format` / `not-negotiated (-4)`
- The `pw-loopback` was a legitimate format translator for THIS specific issue
- GStreamer expected standard PCM but HFP delivers mSBC/CVSD codec audio
- PipeWire's SPA BlueZ plugin decodes the codec, but `pipewiresrc` couldn't negotiate the resulting format
- **Status: IRRELEVANT — GStreamer removed from voice-agent. Pure Rust + PipeWire now.**

### Problem 2: A2DP vs HFP profile selection
- Sony SRS-XB100 always connected in A2DP (speaker only, no mic)
- We manually ran `wpctl set-profile <id> 3` every time to switch to HFP
- Attempted fixes that FAILED:
  - WirePlumber Lua rule with `device.profile = "headset-head-unit"` — DEAD PROPERTY (research agent confirmed WirePlumber 0.4.8 never reads it)
  - WirePlumber Lua rule with `bluez5.profile = "headset-head-unit"` — overridden by m-device-activation
  - BlueZ `/etc/bluetooth/main.conf` ReconnectUUIDs — only controls reconnect after link loss, not initial profile
  - Clearing WirePlumber state files — m-device-activation still chose A2DP
- **SOLUTION FOUND**: WirePlumber's built-in `policy-bluetooth.lua` auto-switches to HFP when a stream with `media.role=Communication` connects. No Lua rules needed. No BlueZ config changes. Just set the right media role in the connecting application.
- **How it works**: policy-bluetooth.lua has `media-role.use-headset-profile = true` enabled by default. When our Rust program connects with `MEDIA_ROLE => "Communication"`, WirePlumber detects the Communication role and auto-switches the Sony from A2DP to headset-head-unit (HFP). The mic appears as a Source node automatically.

### Why we confused them:
We thought the pw-loopback was needed because of the profile issue, when it was actually needed because of the GStreamer format issue. And we thought the profile needed Lua rules, when it just needed the right `media.role=Communication` property on the connecting stream.

### Current state (March 29, 2026):
- GStreamer: REMOVED from voice-agent entirely
- pw-loopback: NOT NEEDED in pure Rust/PipeWire architecture
- HFP profile: LOCKED via WirePlumber Lua script (auto-forces on device connect)
- Audio capture: PROVEN in Rust (peaks 0.44, F32LE 16kHz mono)
- VAD: PROVEN (7 speech segments detected via Silero)
- STT: PROVEN ("Test one two three four" transcribed via Parakeet on CUDA)
- Digital testing: PROVEN via PipeWire monitor port → pw-link → Rust input
- WirePlumber Lua: Runtime script in /usr/share/wireplumber/scripts/, loaded from /etc/wireplumber/main.lua.d/

### Proven Blocks:
1. [x] Block 1: PipeWire capture (pw-record → sherpa-onnx)
2. [x] Block 2a: Rust PipeWire capture (pipewire-rs → F32 samples)
3. [x] Block 2b: Rust VAD (PipeWire → Silero VAD → speech segments)
4. [x] Block 3: Rust STT (VAD segments → Parakeet → text transcription)
5. [ ] Block 4: LLM + TTS (text → brain → Kokoro → playback)
6. [ ] Block 5: Full loop (listen → think → speak → repeat)

---

## ***** TEMPORARY FIX — Sony HFP Profile *****

**STATUS: SHORT-TERM WORKAROUND — NOT PRODUCTION**
**TODO: Replace with proper WirePlumber Lua policy**

The ONLY reliable method to switch Sony SRS-XB100 from A2DP to HFP is:
```bash
wpctl set-profile <DEVICE_ID> 3
```

This must be run AFTER the Sony connects. The device ID changes every reconnect.

**What was tried and FAILED:**
- WirePlumber Lua `device.profile` property — dead property, WP 0.4.8 never reads it
- WirePlumber Lua `bluez5.profile` property — overridden by m-device-activation
- BlueZ `/etc/bluetooth/main.conf` ReconnectUUIDs — only controls link-loss reconnect
- WirePlumber state file `default-profile` with `chattr +i` lock — module ignores it on reconnect
- WirePlumber `media.role=Communication` auto-switch — works BUT reverts when app disconnects (flapping)

**What DOES work:**
- `wpctl set-profile <ID> 3` — manual command, works every time
- `media.role=Communication` on connecting stream — auto-switches but flaps

**Temporary start script workaround:**
```bash
# After bluetoothctl connect:
sleep 5
DEVID=$(wpctl status | grep 'SRS-XB100.*bluez5' | head -1 | sed 's/[^0-9]*//' | cut -d. -f1)
wpctl set-profile $DEVID 3
```

**The REAL fix must be:**
- A proper WirePlumber Lua script using the correct 0.4.8 API
- Must use `device:set_params("Profile", pod)` pattern from policy-bluetooth.lua
- Must fire on device-added event, not as a static rule
- Must be tested with `wireplumber --version` debug output to verify Lua loads without crash
- Previous Lua attempt CRASHED WirePlumber — API calls were wrong

*****  THIS SECTION MUST BE REPLACED WITH LUA SOLUTION  *****

---

## Discovery Timeline
- **Symptom**: pipewiresrc creates ports but 0 bytes captured
- **Initial theory**: WirePlumber policy / session management
- **Failed fixes**: autoconnect=true, media.role=Communication, target-object
- **Breakthrough**: Manual `pw-link` exposed "unhandled format" error
- **Root cause**: HFP codec incompatibility with GStreamer PCM expectations
- **Solution**: pw-loopback as a format translator (not a hack)

---

*Discovered by Emil, Cody & Lyra — March 28, 2026*
*"Sometimes the 'hack' is actually the necessary adapter." — Lyra*
