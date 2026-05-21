# PipeWire + pipewire-rs: The Definitive Guide for Jarvina's Twilio Bridge
## Written for the Jetson Orin Nano — PipeWire 1.2.0
### Sparked Matter LLC — March 31, 2026

---

## Table of Contents

1. [PipeWire Core Concepts](#1-pipewire-core-concepts)
2. [pipewire-rs API](#2-pipewire-rs-api)
3. [The Routing Problem (WirePlumber)](#3-the-routing-problem)
4. [Bidirectional Audio](#4-bidirectional-audio)
5. [Our Architecture: Jarvina + Twilio Bridge](#5-our-architecture)
6. [Properties Reference](#6-properties-reference)
7. [Common Pitfalls](#7-common-pitfalls)

---

## 1. PipeWire Core Concepts

PipeWire is a graph-based multimedia server. Audio and video flow through a directed graph of **nodes** connected by **links**. Think of it as a digital patch panel — every mic, speaker, application, and virtual device is a node with ports, and you wire them together.

### Nodes

A node is any entity that produces or consumes media. Two flavors:

**Device nodes** — represent hardware or virtual hardware:
- `Audio/Sink` — a speaker, headphones, or virtual speaker (consumes audio)
- `Audio/Source` — a microphone or virtual mic (produces audio)
- `Audio/Duplex` — bidirectional (like a Bluetooth HFP headset)

**Stream nodes** — represent applications:
- `Stream/Output/Audio` — an app that plays audio (like a music player)
- `Stream/Input/Audio` — an app that records audio (like a voice recorder)

The naming is confusing at first. Here's the cheat sheet:

| media.class | What it IS | Direction of audio flow |
|---|---|---|
| `Audio/Sink` | Virtual speaker | Audio flows IN (apps play TO it) |
| `Audio/Source` | Virtual microphone | Audio flows OUT (apps record FROM it) |
| `Stream/Output/Audio` | Playback app | Audio flows OUT of the app into a Sink |
| `Stream/Input/Audio` | Capture app | Audio flows IN from a Source to the app |

**The confusing part:** A `Stream/Output/Audio` node plays audio — the "Output" means audio leaves the application. An `Audio/Source` node is like a mic — other things capture FROM it. The naming describes the node's role in the graph, not which direction the data moves internally.

### Ports

Every node has one or more **ports**. Ports are the actual connection points — mono audio channels. A stereo node has two ports (FL, FR). Our mono nodes have one port each.

Ports have a direction:
- **Output port** — produces data (has audio to give)
- **Input port** — consumes data (wants to receive audio)

You can only link an output port to an input port. Never output-to-output or input-to-input.

### Links

A link connects one output port to one input port. Audio flows through links. You can see all links with `pw-link -l` and create new ones with `pw-link <output> <input>`.

Links can be:
- **Active** — audio flows
- **Passive** — created but not driving the graph (set via `node.passive`)

### Device Nodes vs Stream Nodes — Why It Matters

WirePlumber (the session manager) treats device nodes and stream nodes differently:

- **Stream nodes** get auto-linked to device nodes based on policy
- **Device nodes** just exist — they wait for streams to connect to them

When you create a `pw_stream` with `media.class = "Audio/Source"`, you're telling PipeWire: "I am a device — a virtual microphone." Other streams can capture from you. WirePlumber won't auto-link you to the default mic — you ARE a mic.

When you create a `pw_stream` with `MEDIA_CATEGORY = "Capture"` and no `media.class`, it defaults to `Stream/Input/Audio` — a recording app. WirePlumber will auto-link it to the default Source (mic).

**This distinction is everything for our bridge architecture.**

### The Graph in Practice

Here's what our Jarvina setup looks like as a PipeWire graph:

```
Sony SRS-XB100 (Bluetooth HFP)
├── bluez_input (Audio/Source) ──→ jarvina-rust:input_MONO (capture ear)
└── bluez_output (Audio/Sink) ←── jarvina-rust-out:capture_MONO (TTS voice)

Twilio Bridge (what we're building)
├── jarvina-in (Source) ──→ jarvina-rust:input_MONO (phone caller → Jarvina's ear)
└── jarvina-out (Sink) ←── jarvina-rust-out:capture_MONO (Jarvina's voice → phone caller)
```

---

## 2. pipewire-rs API

### Crate: `pipewire = "0.8"`

The Rust bindings wrap the C `libpipewire` library. They're safe but low-level — you work with pods, properties, and callbacks directly.

### Initialization

```rust
pw::init();  // Call once per process, before anything else
```

### The Main Loop

PipeWire needs an event loop. Two options:

**MainLoop** — runs on the current thread, you call `iterate()` manually or `run()` to block:
```rust
let mainloop = pw::main_loop::MainLoop::new(None)?;
// Manual iteration (non-blocking, for custom loops):
mainloop.loop_().iterate(std::time::Duration::from_millis(1));
// Or blocking:
mainloop.run();
```

**ThreadLoop** — runs in a background thread, best for apps that need to do other work:
```rust
let thread_loop = pw::thread_loop::ThreadLoopBox::new(None, None)?;
{
    let _guard = thread_loop.lock();  // Must lock before creating streams
    // ... create context, core, streams ...
}
thread_loop.start();  // Starts the PipeWire thread
```

**Our voice-agent uses MainLoop with manual `iterate()` in a loop** — this lets us do VAD/STT/LLM processing on the same thread between audio callbacks. The Twilio bridge uses separate threads per PipeWire connection.

### Context and Core

```rust
let context = pw::context::Context::new(&mainloop)?;
let core = context.connect(None)?;  // Connects to the PipeWire daemon
```

The `core` is your connection to the PipeWire server. Every stream you create on this core shares the same connection.

**CRITICAL: One core = one scheduling group.** If you put capture and playback streams on the same core, they share a processing quantum. This can cause capture starvation (see Pitfalls).

### Creating a Stream

```rust
let props = pw::properties::properties! {
    *pw::keys::MEDIA_TYPE => "Audio",
    *pw::keys::MEDIA_CATEGORY => "Capture",
    *pw::keys::MEDIA_ROLE => "Communication",
    *pw::keys::NODE_NAME => "jarvina-rust",
};

let stream = pw::stream::Stream::new(&core, "jarvina-capture", props)?;
```

The first arg to `Stream::new` is the `&Core` (the PipeWire connection). The second is a human-readable name. The third is the properties dictionary that controls routing.

### The Process Callback

This is where audio actually flows. You register it via the listener:

```rust
let _listener = stream
    .add_local_listener()
    .process(move |stream, _: &mut ()| {
        // Called by PipeWire when there's a buffer to process
        if let Some(mut buf) = stream.dequeue_buffer() {
            let datas = buf.datas_mut();
            if let Some(d) = datas.first_mut() {
                // ... handle audio ...
            }
        }
    })
    .register()?;
```

**The `_listener` variable MUST stay alive.** If it drops, the callback is unregistered. Keep it in scope for the lifetime of the stream.

### Reading Audio (Capture / Direction::Input)

Inside the process callback for a capture stream:

```rust
.process(move |stream, _: &mut ()| {
    if let Some(mut buf) = stream.dequeue_buffer() {
        let datas = buf.datas_mut();
        if let Some(d) = datas.first_mut() {
            let chunk = d.chunk();
            let chunk_size = chunk.size() as usize;    // bytes of valid data
            let chunk_offset = chunk.offset() as usize; // byte offset into buffer
            if chunk_size > 0 {
                if let Some(bytes) = d.data() {
                    let n_samples = chunk_size / std::mem::size_of::<f32>();
                    // Parse F32LE samples from the byte slice:
                    for i in 0..n_samples {
                        let start = chunk_offset + i * 4;
                        let sample = f32::from_le_bytes(
                            bytes[start..start + 4].try_into().unwrap()
                        );
                        // Push sample to ring buffer...
                    }
                }
            }
        }
    }
})
```

Key points:
- `d.chunk().size()` — how many bytes of actual audio data
- `d.chunk().offset()` — where in the buffer the data starts
- `d.data()` — the raw byte slice (may be larger than chunk_size)
- Audio is interleaved F32LE for our format. Mono = one float per sample.
- **Don't do heavy processing here.** Copy to a shared buffer and process elsewhere.

### Writing Audio (Playback / Direction::Output)

Inside the process callback for a playback stream:

```rust
.process(move |stream, _: &mut ()| {
    if let Some(mut buf) = stream.dequeue_buffer() {
        let datas = buf.datas_mut();
        if let Some(d) = datas.first_mut() {
            if let Some(slice) = d.data() {
                let max_samples = slice.len() / std::mem::size_of::<f32>();
                let mut written = 0usize;

                // Pull samples from your ring buffer:
                if let Ok(mut pb) = playback_buffer.lock() {
                    while written < max_samples {
                        if let Some(sample) = pb.pop_front() {
                            let offset = written * 4;
                            slice[offset..offset + 4]
                                .copy_from_slice(&sample.to_le_bytes());
                            written += 1;
                        } else {
                            break;
                        }
                    }
                }

                // CRITICAL: Fill remainder with silence (zeros)
                for i in written..max_samples {
                    let offset = i * 4;
                    slice[offset..offset + 4]
                        .copy_from_slice(&0.0f32.to_le_bytes());
                }

                // CRITICAL: Set chunk size to full buffer
                let chunk = d.chunk_mut();
                *chunk.size_mut() = (max_samples * 4) as u32;
                *chunk.offset_mut() = 0;
            }
        }
    }
})
```

Key points:
- **Always write the full buffer** — fill unused space with silence (zero floats)
- **Always set `chunk.size_mut()`** — tells PipeWire how much data you wrote
- If you write 0 bytes and set size to 0, PipeWire may stop calling your callback
- `node.always-process = "true"` in props helps keep the callback firing even with no links

### The Format Pod

PipeWire uses "pods" (Plain Old Data) to describe media formats. You must provide a format pod when connecting the stream. Here's the pattern for mono 16kHz F32LE audio:

```rust
let format_obj = pw::spa::pod::object!(
    pw::spa::utils::SpaTypes::ObjectParamFormat,
    pw::spa::param::ParamType::EnumFormat,
    pw::spa::pod::property!(
        pw::spa::param::format::FormatProperties::MediaType,
        Id, pw::spa::param::format::MediaType::Audio
    ),
    pw::spa::pod::property!(
        pw::spa::param::format::FormatProperties::MediaSubtype,
        Id, pw::spa::param::format::MediaSubtype::Raw
    ),
    pw::spa::pod::property!(
        pw::spa::param::format::FormatProperties::AudioFormat,
        Id, pw::spa::param::audio::AudioFormat::F32LE
    ),
    pw::spa::pod::property!(
        pw::spa::param::format::FormatProperties::AudioRate,
        Int, 16000
    ),
    pw::spa::pod::property!(
        pw::spa::param::format::FormatProperties::AudioChannels,
        Int, 1
    ),
);

let pod_bytes: Vec<u8> = PodSerializer::serialize(
    Cursor::new(Vec::new()),
    &pw::spa::pod::Value::Object(format_obj),
)?.0.into_inner();

let mut params = [Pod::from_bytes(&pod_bytes).unwrap()];
```

**Each `stream.connect()` call mutates the params slice**, so if you have two streams, you need two separate pod byte buffers. Don't reuse the same one.

### Connecting the Stream

```rust
stream.connect(
    spa::utils::Direction::Input,  // or Direction::Output
    None,                          // target node ID (None = any/autoconnect)
    pw::stream::StreamFlags::AUTOCONNECT | pw::stream::StreamFlags::MAP_BUFFERS,
    &mut params,
)?;
```

**Direction explained:**
- `Direction::Input` — this stream RECEIVES audio from PipeWire (capture/recording)
- `Direction::Output` — this stream SENDS audio to PipeWire (playback/producing)

**The direction name describes the stream's perspective**, not the audio's perspective:
- A microphone capture app uses `Direction::Input` (audio comes IN to us)
- A music player uses `Direction::Output` (audio goes OUT from us)
- A virtual Source (virtual mic) uses `Direction::Output` (we produce audio for others to capture)
- A virtual Sink (virtual speaker) uses `Direction::Input` (we consume audio that others play)

### StreamFlags

| Flag | What it does | When to use |
|---|---|---|
| `AUTOCONNECT` | WirePlumber auto-links to default device | Almost always. Without it, no automatic linking. |
| `MAP_BUFFERS` | Lets you access buffer data directly via `d.data()` | Always. Without it, buffers are opaque. |
| `RT_PROCESS` | Process callback runs in PipeWire's real-time thread | Use for low latency. Don't do blocking ops in callback. |
| `DRIVER` | Stream drives the graph timing | Rare. For custom timing sources. |
| `NO_CONVERT` | No format conversion (adapter bypass) | When you guarantee your format matches the device. |
| `INACTIVE` | Stream starts inactive (paused) | When you want to start streaming later. |
| `EXCLUSIVE` | Exclusive access to the device | For pro audio apps that need the whole device. |
| `DONT_RECONNECT` | Don't reconnect if target disappears | When losing the target should stop the stream. |
| `ALLOC_BUFFERS` | PipeWire allocates buffers for you | Default behavior for most streams. |

**Our typical combo:** `AUTOCONNECT | MAP_BUFFERS` — let WirePlumber handle routing, and give us direct buffer access.

### Other Useful Stream Methods

```rust
stream.set_active(false);  // Pause the stream (stops callbacks)
stream.set_active(true);   // Resume
stream.flush(true);        // Drain — triggers "drained" callback when buffers empty
stream.disconnect();       // Disconnect from PipeWire graph
stream.node_id();          // Get the PipeWire node ID (u32)
stream.state();            // Current state (Error, Unconnected, Connecting, Paused, Streaming)
```

---

## 3. The Routing Problem

This is where we lost days. WirePlumber is the session manager — it decides which streams get linked to which devices. Understanding its policy is critical.

### How WirePlumber Auto-Links

When a new stream appears with `AUTOCONNECT`, WirePlumber:

1. Looks at `media.class` to determine what TYPE of link is needed
2. Checks for `target.object` — if set, tries to link to that specific node
3. If no target, uses the **default device** for that media class
4. Links output ports to input ports

The default device is whatever `wpctl` shows as the default sink/source. On our Jetson, the NVIDIA APE built-in audio card is the default. This is why everything kept getting linked to APE instead of our virtual nodes.

### media.class vs MEDIA_CATEGORY

This is the source of enormous confusion:

**`MEDIA_CATEGORY`** (a.k.a. `pw::keys::MEDIA_CATEGORY`) — tells PipeWire what KIND of processing you're doing: "Capture", "Playback", "Duplex", "Monitor", "Manager". This is a hint for the session manager.

**`media.class`** (set via `props.insert("media.class", "...")`) — tells PipeWire what you ARE in the graph. This determines how WirePlumber treats you.

If you don't set `media.class` explicitly, PipeWire infers it from `MEDIA_CATEGORY`:
- `MEDIA_CATEGORY = "Capture"` → `media.class = "Stream/Input/Audio"` (recording app)
- `MEDIA_CATEGORY = "Playback"` → `media.class = "Stream/Output/Audio"` (playback app)

If you explicitly set `media.class = "Audio/Source"`, you are a DEVICE (virtual mic), regardless of MEDIA_CATEGORY. This is what jarvina-rust-out does to make itself a Source that other streams can capture from.

### target.object — The Override That Doesn't Always Work

`target.object` tells WirePlumber: "Link me to this specific node (by name or serial)."

**How it's supposed to work:**
```rust
props.insert(*pw::keys::TARGET_OBJECT, "jarvina-rust");
```

**The reality on PipeWire 1.2.0 with WirePlumber:**
- Works reliably for `Stream/Input/Audio` → targeting an `Audio/Source`
- Works reliably for `Stream/Output/Audio` → targeting an `Audio/Sink`
- Gets **overridden** by WirePlumber's default policy in some configurations
- **Does NOT work** if the target node doesn't exist yet when the stream connects (no waiting/retry)

**Workaround:** Use `pw-link` explicitly after both nodes are registered:
```rust
std::process::Command::new("pw-link")
    .args(["jarvina-in:output_MONO", "jarvina-rust:input_MONO"])
    .output();
```

This is what twilio-bridge V8 does — and it's the reliable approach.

### node.autoconnect — The All-or-Nothing Switch

```rust
props.insert("node.autoconnect", "true");   // WirePlumber auto-links (default)
props.insert("node.autoconnect", "false");  // NO auto-linking at all
```

**Problem:** `node.autoconnect = false` prevents ALL connections, not just the default ones. Even `pw-link` manual connections don't stick if WirePlumber sees autoconnect=false and decides to tear them down. (Behavior varies by WirePlumber version.)

**Best practice:** Leave autoconnect=true (or don't set it), then use `pw-link` to override the default connection. Or set `media.class` to a device type so WirePlumber treats you as a device, not a stream.

### node.dont-reconnect, node.dont-move, node.dont-fallback

These stream-only properties fine-tune WirePlumber's behavior:

```
node.dont-reconnect = true  → If my target disappears, don't move me to another device. Just error.
node.dont-move = true       → Don't let external tools (pavucontrol, etc.) re-route me.
node.dont-fallback = true   → If my target doesn't exist, error immediately (don't use default).
node.linger = true          → With dont-fallback, WAIT for the target instead of erroring.
```

### The APE Hijacking Problem

On the Jetson Orin Nano, the built-in audio device is "NVIDIA Jetson Orin Nano APE" — it shows up as the default Audio/Sink and Audio/Source. When we create bridge streams without explicit `media.class`, WirePlumber auto-links them to APE:

```
jarvina-in → APE speakers  (WRONG — should go to jarvina-rust)
jarvina-out ← APE mic      (WRONG — should come from jarvina-rust-out)
```

**Solutions (in order of reliability):**
1. Set `media.class = "Audio/Source"` or `"Audio/Sink"` to be a device, not a stream
2. Use `node.autoconnect = false` + explicit `pw-link`
3. Set `target.object` to the desired node name
4. Use WirePlumber Lua rules to override default routing

**What we actually do in twilio-bridge V8:** Separate PipeWire connections per direction, `AUTOCONNECT` on, then explicit `pw-link` commands after a 2-second delay to let ports register. The `pw-link` overrides WirePlumber's default routing.

---

## 4. Bidirectional Audio

Building a system that captures AND plays audio simultaneously. This is where the real dragons live.

### The Capture Starvation Problem

**What happens:** You create two streams on the same `Core` connection — one `Direction::Input` (capture), one `Direction::Output` (playback). The playback works fine. The capture callback fires, but all samples are zero.

**Why:** When both streams share the same PipeWire connection (same `Core`), they're in the same scheduling group. PipeWire's graph scheduler may not deliver capture buffers correctly when the output stream is actively producing. The output stream "starves" the input stream of processing time.

**This is not a documented limitation** — it's a behavior we observed on PipeWire 1.2.0 on the Jetson. It may be a bug, a configuration issue, or an intentional design choice for real-time scheduling.

### The Solution: Separate Connections

**One PipeWire connection per direction.** Each connection gets its own `MainLoop`, `Context`, and `Core`, running in its own thread:

```rust
// Thread 1: Source (produces audio → graph)
std::thread::spawn(move || {
    pw::init();
    let ml = pw::main_loop::MainLoop::new(None).unwrap();
    let ctx = pw::context::Context::new(&ml).unwrap();
    let core = ctx.connect(None).unwrap();

    // Create Direction::Output stream here
    // ...

    while running.load(Ordering::Relaxed) {
        ml.loop_().iterate(Duration::from_millis(1));
    }
});

// Thread 2: Sink (consumes audio from graph)
std::thread::spawn(move || {
    pw::init();
    let ml = pw::main_loop::MainLoop::new(None).unwrap();
    let ctx = pw::context::Context::new(&ml).unwrap();
    let core = ctx.connect(None).unwrap();

    // Create Direction::Input stream here
    // ...

    while running.load(Ordering::Relaxed) {
        ml.loop_().iterate(Duration::from_millis(1));
    }
});
```

**This is what twilio-bridge V8 does** — and it works. Each port is a completely independent PipeWire client.

### But Jarvina Has Both on One Core...

Yes — `voice-agent-rust` (main.rs) has BOTH `jarvina-rust` (capture) and `jarvina-rust-out` (playback) on the same `MainLoop` and `Core`. And it works with the Sony speaker.

**Why it works there:** The voice agent isn't doing simultaneous capture and playback. It captures → processes → then plays. The streams are never both active at the same time in a competing way. Also, `jarvina-rust-out` has `media.class = "Audio/Source"` (device node), which puts it in a different scheduling group than `jarvina-rust` (stream node).

**Why it breaks for the bridge:** The bridge needs truly simultaneous bidirectional audio — phone caller talks WHILE Jarvina talks back. Both streams are active at the same time. Same-core scheduling fails here.

### Three Valid Architectures

**1. Separate processes (nuclear option):**
- One process for capture, one for playback
- Communicate via Unix socket or shared memory
- Most isolated, most complex

**2. Separate threads with separate connections (what we use):**
- One thread per PipeWire stream, each with its own Core
- Share data via `Arc<Mutex<VecDeque<f32>>>`
- Good balance of isolation and simplicity

**3. Same thread, same core, with media.class device nodes:**
- Might work if both streams are device nodes (Audio/Source and Audio/Sink)
- Untested in our case
- Less isolated, simpler code

---

## 5. Our Architecture: Jarvina + Twilio Bridge

### The Full Picture

```
┌────────────────────────────────────────────────────────────────┐
│                      PIPEWIRE GRAPH                            │
│                                                                │
│  ┌──────────────┐    ┌──────────────┐    ┌──────────────────┐  │
│  │ Twilio Bridge │    │ Jarvina Agent │    │ Sony SRS-XB100   │  │
│  │              │    │              │    │ (Bluetooth HFP)  │  │
│  │              │    │              │    │                  │  │
│  │ jarvina-in   │───►│ jarvina-rust │◄───│ bluez_input      │  │
│  │ (Source,     │    │ (capture ear)│    │ (mic)            │  │
│  │  OUTPUT)     │    │ (Input)      │    │                  │  │
│  │              │    │              │    │                  │  │
│  │ jarvina-out  │◄───│ jarvina-rust │───►│ bluez_output     │  │
│  │ (Sink,       │    │ -out (TTS    │    │ (speaker)        │  │
│  │  INPUT)      │    │  voice,      │    │                  │  │
│  │              │    │  Source,     │    │                  │  │
│  │              │    │  OUTPUT)     │    │                  │  │
│  └──────────────┘    └──────────────┘    └──────────────────┘  │
│                                                                │
│  pw-link connections:                                          │
│  jarvina-in:output_MONO ──→ jarvina-rust:input_MONO            │
│  jarvina-rust-out:capture_MONO ──→ jarvina-out:input_MONO      │
└────────────────────────────────────────────────────────────────┘

         ▲                                          │
         │ Twilio WebSocket                         │
         │ (mu-law 8kHz)                            │
         │                                          ▼

┌─────────────────────────────────────────────────────────────┐
│                     TWILIO CLOUD                             │
│                                                             │
│  Phone caller ←→ Twilio Media Stream ←→ WebSocket (wss://) │
└─────────────────────────────────────────────────────────────┘
```

### Data Flow — Inbound (Phone → Jarvina)

1. Phone caller speaks
2. Twilio sends mu-law 8kHz audio as base64 in WebSocket JSON messages
3. Bridge decodes base64 → mu-law bytes → i16 PCM → f32 samples
4. Bridge upsamples 8kHz → 16kHz (linear interpolation)
5. Samples pushed into `inbound_buf: Arc<Mutex<VecDeque<f32>>>`
6. `jarvina-in` PipeWire stream (Direction::Output, its own thread) pulls from buffer in process callback
7. `pw-link` routes `jarvina-in:output_MONO` → `jarvina-rust:input_MONO`
8. Jarvina's capture callback receives the audio → VAD → STT → LLM

### Data Flow — Outbound (Jarvina → Phone)

1. Jarvina's LLM generates response text
2. Kokoro TTS generates 24kHz audio → resampled to 16kHz
3. Samples pushed into `playback_buffer: Arc<Mutex<VecDeque<f32>>>`
4. `jarvina-rust-out` PipeWire stream (Direction::Output, media.class=Audio/Source) pulls from buffer
5. `pw-link` routes `jarvina-rust-out:capture_MONO` → `jarvina-out:input_MONO`
6. `jarvina-out` PipeWire stream (Direction::Input, its own thread) captures the audio
7. Bridge reads F32 samples → downsample 16kHz → 8kHz → encode mu-law → base64 → WebSocket JSON
8. Twilio plays audio to phone caller

### Port Names and Their pw-link Identifiers

| Node | Direction | Port name in `pw-link -l` |
|---|---|---|
| `jarvina-rust` | Input (capture) | `jarvina-rust:input_MONO` |
| `jarvina-rust-out` | Output (source) | `jarvina-rust-out:capture_MONO` |
| `jarvina-in` | Output (source) | `jarvina-in:output_MONO` |
| `jarvina-out` | Input (sink) | `jarvina-out:input_MONO` |

**Note:** The port suffix (`input_MONO`, `output_MONO`, `capture_MONO`) is assigned by PipeWire based on the stream's direction and format. You don't control it — you discover it with `pw-link -l`.

### The pw-link Commands (in twilio-bridge main.rs)

```rust
// Wait for ports to register
tokio::time::sleep(Duration::from_secs(2)).await;

// Phone audio → Jarvina's ear
std::process::Command::new("pw-link")
    .args(["jarvina-in:output_MONO", "jarvina-rust:input_MONO"])
    .output();

// Jarvina's voice → Bridge capture
std::process::Command::new("pw-link")
    .args(["jarvina-rust-out:capture_MONO", "jarvina-out:input_MONO"])
    .output();
```

The 2-second sleep is necessary — PipeWire needs time to register the ports after `stream.connect()`. Without it, `pw-link` can't find the ports.

### Why Three Ports, Not Two

You might think: "Why not just one PipeWire stream with two ports (duplex)?" Because:

1. PipeWire streams are unidirectional — one direction per stream
2. Each direction needs its own scheduling (see capture starvation)
3. The bridge is an AIRLOCK between Twilio (async WebSocket) and PipeWire (real-time callbacks)
4. Separate threads for each PipeWire connection keeps the real-time callbacks from blocking on WebSocket I/O

---

## 6. Properties Reference

Everything we've learned about PipeWire properties, documented honestly — what the docs say AND what we observed.

### Stream Identity Properties

| Property | Key constant | What it does |
|---|---|---|
| `node.name` | `*pw::keys::NODE_NAME` | Unique name for the node. Used by `pw-link` and `target.object`. |
| `node.description` | `*pw::keys::NODE_DESCRIPTION` | Human-readable label (shows in pavucontrol, etc.) |
| `node.nick` | `*pw::keys::NODE_NICK` | Short nickname |

### Media Type Properties

| Property | Key constant | Values | Effect |
|---|---|---|---|
| `media.type` | `*pw::keys::MEDIA_TYPE` | "Audio", "Video", "Midi" | Broad media category |
| `media.category` | `*pw::keys::MEDIA_CATEGORY` | "Capture", "Playback", "Duplex", "Monitor", "Manager" | What kind of processing you do |
| `media.role` | `*pw::keys::MEDIA_ROLE` | "Communication", "Music", "Movie", "Game", "Notification", etc. | Affects priority and routing policy. "Communication" triggers HFP profile switch on Bluetooth. |
| `media.class` | *(set via props.insert)* | See table below | MOST IMPORTANT — determines how WirePlumber treats you |

### media.class Values — The Master Table

| media.class | You are a... | WirePlumber treats you as... | Direction to use |
|---|---|---|---|
| `Audio/Sink` | Virtual speaker | Device — streams can play TO you | `Direction::Input` |
| `Audio/Source` | Virtual microphone | Device — streams can capture FROM you | `Direction::Output` |
| `Audio/Sink/Virtual` | Virtual speaker (marked virtual) | Should be same as Audio/Sink but... (see pitfalls) | `Direction::Input` |
| `Audio/Source/Virtual` | Virtual microphone (marked virtual) | Creates linkable ports (works) | `Direction::Output` |
| `Audio/Duplex` | Bidirectional device | Device with both input and output ports | Both |
| `Stream/Output/Audio` | Playback application | Stream — auto-linked to default Sink | `Direction::Output` |
| `Stream/Input/Audio` | Capture application | Stream — auto-linked to default Source | `Direction::Input` |
| *(not set)* | Inferred from MEDIA_CATEGORY | Stream — auto-linked to default device | Depends on category |

### Routing Properties

| Property | Key constant | Values | What it really does |
|---|---|---|---|
| `target.object` | `*pw::keys::TARGET_OBJECT` | Node name or object serial | Requests specific target. Works for streams targeting devices. Can be overridden by WirePlumber. |
| `node.autoconnect` | `*pw::keys::NODE_AUTOCONNECT` | "true" (default), "false" | true = WirePlumber auto-links. false = NO auto-linking at all. |
| `node.dont-reconnect` | `*pw::keys::NODE_DONT_RECONNECT` | "true", "false" | If target dies, destroy stream instead of relinking |
| `node.always-process` | `*pw::keys::NODE_ALWAYS_PROCESS` | "true", "false" | Keep calling process callback even with no links. Essential for device nodes that need to stay alive. |
| `node.passive` | `*pw::keys::NODE_PASSIVE` | "true", "false" | Passive links — don't prevent device from suspending |
| `node.virtual` | `*pw::keys::NODE_VIRTUAL` | "true", "false" | Marks node as virtual (no physical hardware) |
| `stream.capture.sink` | `*pw::keys::STREAM_CAPTURE_SINK` | "true", "false" | Capture from sink's monitor ports (hear what's playing to a speaker) |

### Audio Format Properties (in the format pod, not stream props)

| Property | Type | Our value | Notes |
|---|---|---|---|
| MediaType | Id | Audio | Always Audio for us |
| MediaSubtype | Id | Raw | Raw PCM (not compressed) |
| AudioFormat | Id | F32LE | 32-bit float, little-endian. Best for PipeWire internal processing. |
| AudioRate | Int | 16000 | 16kHz — matches our VAD/STT input requirements |
| AudioChannels | Int | 1 | Mono — telephony audio is always mono |

---

## 7. Common Pitfalls

Everything we learned the hard way, so future sessions don't repeat the pain.

### Pitfall 1: Capture Starvation (Zeros from Capture Stream)

**Symptom:** Capture process callback fires, chunk.size() > 0, but all samples are 0.0.

**Cause:** Capture and playback streams on the same PipeWire Core, both active simultaneously.

**Fix:** Separate PipeWire connections (separate Core per direction). Each in its own thread.

**How to diagnose:**
```rust
// In the capture callback, track peak:
let peak = samples.iter().fold(0.0f32, |m, &s| m.max(s.abs()));
capture_peak_bits.store(peak.to_bits(), Ordering::Relaxed);
```
If peak is always 0.0 but the callback fires regularly, you have starvation.

### Pitfall 2: APE Hijacking

**Symptom:** Your streams get linked to `alsa_output.platform-tegra-ape` instead of your virtual nodes.

**Cause:** WirePlumber's default policy links streams to the default device. On Jetson, that's NVIDIA APE.

**Fix:** Either:
- Set `media.class` to a device type (Audio/Source, Audio/Sink) so you're not a "stream" seeking a device
- Use explicit `pw-link` after port registration
- Change the default device: `wpctl set-default <node_id>`

### Pitfall 3: Audio/Sink/Virtual Doesn't Create Linkable Ports

**Symptom:** You set `media.class = "Audio/Sink/Virtual"` but `pw-link -l` shows no ports for your node.

**Cause:** On PipeWire 1.2.0, `Audio/Sink/Virtual` behaves differently from `Audio/Sink`. The "/Virtual" suffix affects how WirePlumber creates the adapter node. Ports may not appear, or they may appear but not be linkable.

**Fix:** Use `Audio/Sink` (without /Virtual). Or use `Audio/Source/Virtual` which DOES work.

**Status:** This might be a PipeWire 1.2.0 bug or a WirePlumber version-specific behavior. Use `wpctl status` and `pw-link -l` to verify port creation after every change.

### Pitfall 4: protocol-simple Capture Fragments

**Symptom:** Using PipeWire's `protocol-simple` module on TCP:4711, the capture side delivers incomplete or fragmented audio.

**Cause:** `protocol-simple` creates a new stream for each TCP connection. The stream negotiation and buffer size is optimized for simple tools like `arecord`, not for real-time bidirectional audio. The capture stream may not get scheduled consistently.

**Fix:** Don't use `protocol-simple` for the Twilio bridge. Use native `pipewire-rs` streams instead. They give you direct control over buffer handling and scheduling.

**When protocol-simple IS useful:** Quick testing with `nc`:
```bash
# Capture from PipeWire and pipe to a file:
nc localhost 4711 > /tmp/capture.raw
# Play raw audio into PipeWire:
cat /tmp/audio.raw | nc localhost 4711
```

### Pitfall 5: Port Name Discovery

**Symptom:** `pw-link jarvina-in:output_MONO jarvina-rust:input_MONO` fails — "no such port."

**Cause:** Port names include a suffix assigned by PipeWire's adapter. The suffix depends on the channel count and format. For mono F32LE, typical suffixes are:
- Input streams: `input_MONO` or `input_FL` (sometimes `input_0`)
- Output streams: `output_MONO` or `capture_MONO` (for Audio/Source nodes)

**Fix:** After `stream.connect()`, wait 1-2 seconds, then run `pw-link -l` to discover the actual port names:
```bash
pw-link -l | grep jarvina
```

**In code, the sleep-then-link pattern:**
```rust
tokio::time::sleep(Duration::from_secs(2)).await;
let output = std::process::Command::new("pw-link").arg("-l").output();
// Parse output to discover actual port names, then link
```

### Pitfall 6: Stereo vs Mono Mismatch

**Symptom:** Audio plays at half speed or double speed. Or you get only silence from one channel.

**Cause:** The format pod says mono (channels=1) but the linked device expects stereo, or vice versa. PipeWire's adapter handles the conversion, but if `NO_CONVERT` flag is set, no conversion happens.

**Fix:**
- Don't use `StreamFlags::NO_CONVERT` unless you're sure about the format
- Verify with `pw-top` — check the format column for each stream
- Our Twilio audio is always mono 16kHz. Our Sony Bluetooth HFP is also mono. No conflict there.

### Pitfall 7: Process Callback Stops Firing

**Symptom:** Process callback fires for a few seconds, then stops. No audio.

**Cause:** If the stream has no links and `node.always-process` is not set, PipeWire may suspend it. Or if the playback buffer runs empty and `chunk.size_mut()` is set to 0, PipeWire thinks you're done.

**Fix:**
- Set `props.insert("node.always-process", "true")` for device nodes
- Always write the full buffer in playback callbacks (silence-pad the remainder)
- Always set `chunk.size_mut()` to a non-zero value

### Pitfall 8: The Listener Must Stay Alive

**Symptom:** Stream connects successfully, but no callbacks ever fire.

**Cause:** The listener object returned by `.register()` was dropped. In Rust, when the variable goes out of scope, the callback is unregistered.

**Fix:** Keep the listener alive:
```rust
// WRONG — listener drops immediately:
stream.add_local_listener().process(|s, _| { ... }).register()?;

// RIGHT — listener lives as long as _listener is in scope:
let _listener = stream.add_local_listener()
    .process(|s, _| { ... })
    .register()?;
```

### Pitfall 9: pw::init() Per Thread

**Symptom:** Crash or assertion failure when creating PipeWire objects in a new thread.

**Cause:** Each thread that uses PipeWire must call `pw::init()` before creating any PipeWire objects.

**Fix:**
```rust
std::thread::spawn(move || {
    pw::init();  // Required per thread
    let ml = pw::main_loop::MainLoop::new(None).unwrap();
    // ...
});
```

### Pitfall 10: Pod Bytes Are Consumed by connect()

**Symptom:** Second stream.connect() call panics or fails with invalid pod.

**Cause:** `stream.connect()` takes `&mut [&Pod]`, and the Pod references the underlying bytes. If you reuse the same byte buffer, the first connect may have mutated it.

**Fix:** Create a fresh pod byte buffer for each stream:
```rust
let pod_bytes1 = make_format_pod();  // For stream 1
let pod_bytes2 = make_format_pod();  // For stream 2

let mut params1 = [Pod::from_bytes(&pod_bytes1).unwrap()];
stream1.connect(..., &mut params1)?;

let mut params2 = [Pod::from_bytes(&pod_bytes2).unwrap()];
stream2.connect(..., &mut params2)?;
```

### Pitfall 11: Twilio's mu-law and Sample Rate

**Symptom:** Audio to/from phone sounds like chipmunks or underwater.

**Cause:** Twilio sends/receives mu-law encoded audio at 8kHz. Our PipeWire streams run at 16kHz F32LE.

**Fix:** Proper conversion chain:
- **Inbound:** mu-law → i16 PCM → f32 normalized → upsample 8k→16k (linear interpolation)
- **Outbound:** f32 16k → downsample 16k→8k (average pairs) → i16 PCM → mu-law encode

The interpolation doesn't need to be fancy — linear is fine for telephony audio.

### Pitfall 12: Bluetooth HFP Profile Flapping

**Symptom:** Sony connects in A2DP (high-quality playback only, no mic). You switch to HFP with `wpctl set-profile`, then the stream reconnects and it switches back to A2DP.

**Cause:** WirePlumber's `policy-bluetooth.lua` auto-manages profiles. When a Communication stream disconnects, it switches back to A2DP.

**Fix:**
- Set `MEDIA_ROLE => "Communication"` on the Jarvina capture stream — this tells WirePlumber to switch to HFP and keep it
- Use a WirePlumber Lua script that forces HFP on device connect (see `jarvina-bluetooth-hfp-discovery.md`)
- As a manual override: `wpctl set-profile <DEVICE_ID> 3` after each reconnect

---

## Diagnostic Commands

Keep these in your toolbox:

```bash
# See all nodes and their media.class:
wpctl status

# See all ports (output and input):
pw-link -o   # output ports
pw-link -i   # input ports
pw-link -l   # all links (who's connected to who)

# Real-time stream monitoring:
pw-top

# Dump full node properties:
pw-dump | jq '.[] | select(.info.props["node.name"] == "jarvina-rust")'

# Create a link:
pw-link jarvina-in:output_MONO jarvina-rust:input_MONO

# Remove a link:
pw-link -d jarvina-in:output_MONO jarvina-rust:input_MONO

# Monitor port connect/disconnect events:
pw-mon

# Test capture (record to file):
pw-record --target jarvina-rust-out /tmp/test.wav

# Test playback (play to node):
pw-play --target jarvina-out /tmp/test.wav
```

---

## Summary: The Rules

1. **One PipeWire connection per direction** for simultaneous bidirectional audio
2. **media.class determines your identity** — Audio/Source = virtual mic, Audio/Sink = virtual speaker, Stream/* = application
3. **pw-link is more reliable than target.object** for explicit routing
4. **Always silence-pad playback buffers** and set chunk.size_mut() to the full buffer
5. **node.always-process = "true"** for device nodes that must stay alive
6. **pw::init() per thread** — don't forget it
7. **Keep listeners alive** — the `_listener` variable must not drop
8. **Sleep before pw-link** — ports need time to register
9. **media.role = "Communication"** triggers Bluetooth HFP auto-switch
10. **Check `wpctl status` after every change** — trust your eyes, not your assumptions

---

*Built by Emil, Cody, Lyra & Ara — Sparked Matter LLC*
*"Code with Soul and Spirit, Powered by Joy"*
