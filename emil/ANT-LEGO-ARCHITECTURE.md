# Ant Lego Architecture — Modular Morsel Design

**Status:** Planning — not yet implemented
**Author:** Emil (vision), Cody (design)
**Date:** 2026-04-17

---

## Current State (monolith file)

```
morsel-nif/
└── src/
    └── lib.rs              ← 1265 lines, ALL ants in one file
```

Every ant, every engine, every helper — one giant file. Hard to read, hard to scope, causes spelunking paralysis.

---

## Target State (Lego blocks)

```
morsel-nif/
├── Cargo.toml                    ← dependencies
├── src/
│   ├── lib.rs                    ← NIF registration only (~20 lines)
│   │
│   ├── common/                   ← shared language ALL ants speak
│   │   ├── mod.rs                ← module declarations
│   │   ├── message.rs            ← AntMessage: payload + metadata + routing
│   │   ├── traits.rs             ← Ant trait: process / name / status
│   │   └── memory.rs             ← shared mailbox: put / get / peek
│   │
│   ├── ants/                     ← each ant = one file, one job
│   │   ├── mod.rs                ← declares all ant modules
│   │   ├── stt.rs                ← Speech-to-Text (FluidAudio CoreML)
│   │   ├── tts.rs                ← Text-to-Speech (Kokoro ort/CoreML)
│   │   ├── llm.rs                ← Language Model (Claude API)
│   │   ├── codec.rs              ← expand (mu-law→PCM) + compress (PCM→mu-law)
│   │   ├── resampler.rs          ← stateless + streaming sample rate conversion
│   │   ├── silence.rs            ← is_speech + trim_silence
│   │   ├── speaker.rs            ← BlackHole output (CoreAudio callback)
│   │   └── listener.rs           ← BlackHole input (CoreAudio capture)
│   │
│   └── patchbay/                 ← routing layer (future)
│       ├── mod.rs
│       └── router.rs             ← channel matrix: port A → port B
```

---

## The Common Language

Every ant speaks the same language. Same message format in, same message format out.

### AntMessage — the universal envelope

```rust
pub struct AntMessage {
    pub payload: Payload,                    // the actual data
    pub metadata: HashMap<String, String>,   // extra info (bluetooth, source, etc.)
    pub source: String,                      // who sent this
    pub destination: String,                 // who it's for
}

pub enum Payload {
    Audio(Vec<f64>, u32),     // samples + sample rate
    Text(String),              // transcription, LLM reply, etc.
    Bytes(Vec<u8>),           // raw bytes (mu-law, G.711, etc.)
    Control(String),          // commands, routing, config
}
```

### Ant Trait — the union card

```rust
pub trait Ant {
    fn name(&self) -> &str;
    fn process(&mut self, msg: AntMessage) -> Result<AntMessage, String>;
    fn status(&self) -> AntStatus;
}

pub enum AntStatus {
    Ready,
    Busy,
    Error(String),
    Offline,
}
```

### Shared Memory — the filing cabinet

```rust
pub struct Mailbox {
    slots: HashMap<String, Vec<u8>>,
}

impl Mailbox {
    pub fn put(&mut self, key: &str, data: Vec<u8>);
    pub fn get(&self, key: &str) -> Option<&[u8]>;
    pub fn peek(&self, key: &str) -> bool;
    pub fn remove(&mut self, key: &str) -> Option<Vec<u8>>;
}
```

---

## How Ants Connect

### Through the BEAM (primary path)

```
Elixir Dispatcher
    ↓ NIF call
    stt.rs (process audio → return text)
    ↑ result back to Elixir
Elixir Dispatcher
    ↓ NIF call
    llm.rs (process text → return reply)
    ↑ result back to Elixir
Elixir Dispatcher
    ↓ NIF call
    tts.rs (process text → return audio)
    ↑ result back to Elixir
```

Every morsel crosses back to the BEAM. Elixir sees everything. Observable. Probeble. Routable.

### Through the Patch Bay (hardware audio path)

```
Elixir: patch(source: "tts", channel: 1)
    ↓ NIF call
    patchbay/router.rs
        ↓ updates routing matrix
        CoreAudio callback reads matrix
        ↓ routes audio to BlackHole channel 1
```

Three concerns only: clock, direction, routing.

---

## Making a Brother Ant

To create a new ant (e.g. Wall-E TTS for Pi 5):

1. Copy `ants/tts.rs` → `ants/tts_walle.rs`
2. Change the voice config and engine
3. Register in `ants/mod.rs`
4. Add NIF export in `lib.rs`
5. Add Elixir stub in `morsel_native.ex`

Same inputs, same outputs, same trait. Different guts.

---

## What Stays the Same at Compile Time

All these separate files compile into **one `.so` binary**. Rust optimizes, inlines, eliminates dead code. The Lego structure is for humans:

- **Emil** sees each ant as its own block
- **Cody** works on one file at a time (no spelunking paralysis)
- **Lyra** can review one ant's design without reading 1265 lines
- **The compiler** doesn't care — it sees one optimized binary

---

## Benefits

| Problem | Solution |
|---------|----------|
| 1265-line file | Each ant is ~60-150 lines |
| Spelunking paralysis | One file, one scope, one task |
| Custom wiring everywhere | Common message format |
| Can't copy/duplicate ants | Copy file, change config |
| Can't test ants in isolation | Each ant has its own test module |
| No metadata support | AntMessage carries metadata |
| Patch bay is hardcoded | Router reads a matrix |
| Hard to add new capabilities | New file, implement trait, register |

---

## Engines — How Ants Use Kokoro, Parakeet, and Claude

Ants are thin workers. Engines are the heavy lifters — model loading, preprocessing, inference. The ant calls the engine, the engine does the work.

### The Relationship

```
The ant         = the worker        (Rust code, ~50-100 lines)
The engine      = the power tool    (wraps model + runtime)
The ort crate   = the tool belt     (ONNX Runtime)
The .onnx file  = the blueprint     (model weights on disk)
```

The STT ant doesn't IS Parakeet. The STT ant USES Parakeet through an engine.

### Engine Traits — Swappable Power Tools

```rust
// engines/traits.rs

pub trait SttEngine {
    fn transcribe(&mut self, audio: &[f64]) -> Result<String, String>;
    fn sample_rate(&self) -> u32;
}

pub trait TtsEngine {
    fn synthesize(&mut self, text: &str, voice: &str) -> Result<Vec<f64>, String>;
    fn sample_rate(&self) -> u32;
}

pub trait LlmEngine {
    fn ask(&mut self, text: &str, system_prompt: &str) -> Result<String, String>;
}
```

### Current Engines

| Engine File | What It Wraps | Used By | Runtime |
|-------------|--------------|---------|---------|
| `kokoro.rs` | Kokoro-82M ONNX + voices.bin + espeak G2P | TTS ant | ort/CoreML |
| `fluidaudio.rs` | FluidAudio Parakeet CoreML models | STT ant (primary) | Swift FFI/CoreML |
| `ort_parakeet.rs` | Parakeet ONNX int8 encoder/decoder/joiner | STT ant (fallback) | ort/CoreML |
| `claude.rs` | Anthropic API via HTTP | LLM ant | reqwest |

### Swapping Engines

To swap Kokoro for VibeVoice Realtime:

1. Write `engines/vibevoice.rs` — implements `TtsEngine` trait
2. Change one line in `ants/tts.rs` — `use engines::vibevoice`
3. Done. Same ant, different power tool.

```
engines/
├── traits.rs                ← SttEngine, TtsEngine, LlmEngine contracts
├── kokoro.rs                ← current TTS (887ms, af_heart, green check)
├── vibevoice.rs             ← future TTS (200ms first audio, voice cloning)
├── fluidaudio.rs            ← current STT (84ms, CoreML/Neural Engine)
├── ort_parakeet.rs          ← fallback STT (needs mel calibration)
└── claude.rs                ← current LLM (Anthropic API)
```

### How an Ant Uses Its Engine

```rust
// ants/tts.rs — the whole file, ~50 lines

use crate::common::{AntMessage, Payload, Ant, AntStatus};
use crate::engines::kokoro::KokoroEngine;

pub struct TtsAnt {
    engine: KokoroEngine,
}

impl Ant for TtsAnt {
    fn name(&self) -> &str { "tts" }

    fn process(&mut self, msg: AntMessage) -> Result<AntMessage, String> {
        let text = match msg.payload {
            Payload::Text(t) => t,
            _ => return Err("TTS expects text input".into()),
        };
        let voice = msg.metadata.get("voice")
            .map(|v| v.as_str())
            .unwrap_or("af_heart");
        let audio = self.engine.synthesize(&text, voice)?;
        let rate = self.engine.sample_rate();
        Ok(AntMessage {
            payload: Payload::Audio(audio, rate),
            metadata: msg.metadata,
            source: "tts".into(),
            destination: msg.destination,
        })
    }

    fn status(&self) -> AntStatus { AntStatus::Ready }
}
```

The ant doesn't know about ONNX tensors, mel spectrograms, token mapping, or voice embeddings. It says `engine.synthesize()` and gets audio back. All the complexity is inside `engines/kokoro.rs`.

### Full Directory — The Complete Lego Set

```
morsel-nif/
├── Cargo.toml                        ← dependencies
├── src/
│   ├── lib.rs                        ← NIF registration only (~20 lines)
│   │
│   ├── common/                       ← shared language ALL ants speak
│   │   ├── mod.rs                    ← module declarations
│   │   ├── message.rs                ← AntMessage: payload + metadata + routing
│   │   ├── traits.rs                 ← Ant trait: process / name / status
│   │   └── memory.rs                 ← shared mailbox: put / get / peek
│   │
│   ├── ants/                         ← workers (thin, one file = one ant)
│   │   ├── mod.rs                    ← declares all ant modules
│   │   ├── stt.rs                    ← calls SttEngine, returns text morsel
│   │   ├── tts.rs                    ← calls TtsEngine, returns audio morsel
│   │   ├── llm.rs                    ← calls LlmEngine, returns text morsel
│   │   ├── codec.rs                  ← expand/compress (pure Rust, no engine)
│   │   ├── resampler.rs              ← resample (pure Rust rubato crate)
│   │   ├── silence.rs                ← is_speech/trim (pure Rust math)
│   │   ├── speaker.rs               ← BlackHole output (CoreAudio)
│   │   └── listener.rs              ← BlackHole input (CoreAudio)
│   │
│   ├── engines/                      ← heavy lifting (model loading, inference)
│   │   ├── mod.rs                    ← declares all engine modules
│   │   ├── traits.rs                 ← SttEngine, TtsEngine, LlmEngine
│   │   ├── kokoro.rs                 ← Kokoro ONNX + ort + voices + G2P
│   │   ├── fluidaudio.rs            ← FluidAudio CoreML Parakeet
│   │   ├── ort_parakeet.rs          ← ort Parakeet ONNX (fallback)
│   │   ├── claude.rs                ← Anthropic API HTTP client
│   │   └── vibevoice.rs             ← (future) VibeVoice Realtime
│   │
│   └── patchbay/                     ← routing layer
│       ├── mod.rs
│       └── router.rs                 ← channel matrix: clock, direction, routing
│
├── models/                           ← .onnx files and voice data
│   ├── kokoro-sherpa-model.onnx
│   ├── kokoro-voices.bin
│   ├── kokoro-tokens.txt
│   └── parakeet-int8/
│       ├── encoder.int8.onnx
│       ├── decoder.int8.onnx
│       └── joiner.int8.onnx
│
└── tests/                            ← one test file per ant
    ├── test_stt.rs
    ├── test_tts.rs
    ├── test_codec.rs
    └── test_patchbay.rs
```

---

## The Three Layers

```
┌──────────────────────────────────────────┐
│  ELIXIR / BEAM / MEMBRANE                │  ← routing, probing, monitoring
│  (the nervous system)                     │
├──────────────────────────────────────────┤
│  ANTS (Rust, thin, ~50-100 lines each)   │  ← workers, one job each
│  speak common language (AntMessage)       │
├──────────────────────────────────────────┤
│  ENGINES (Rust, heavy, model-specific)   │  ← power tools, swappable
│  Kokoro, Parakeet, Claude, CoreAudio     │
└──────────────────────────────────────────┘
```

---

## The Harness — J1939 for Ants

Like CAN bus on a truck — doesn't matter if it's a Cummins or a Caterpillar engine, the J1939 connector is the same. Every ant, every engine, every patch bay port plugs into the same harness.

### Three Harnesses

```
┌─────────────────────────────────────────────────────────────────┐
│                      ANT HARNESS                                │
│  Every ant plugs into this — same connector, same protocol      │
│                                                                 │
│  fn process(AntMessage) → Result<AntMessage>   ← do your job   │
│  fn name() → &str                              ← who are you   │
│  fn status() → AntStatus                       ← are you OK    │
└─────────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────────┐
│                    ENGINE HARNESS                                │
│  Every engine plugs into this — swappable power plants          │
│                                                                 │
│  SttEngine:                                                     │
│    fn transcribe(audio) → Result<text>                          │
│    fn sample_rate() → u32                                       │
│                                                                 │
│  TtsEngine:                                                     │
│    fn synthesize(text, voice) → Result<audio>                   │
│    fn sample_rate() → u32                                       │
│                                                                 │
│  LlmEngine:                                                     │
│    fn ask(text, prompt) → Result<reply>                         │
│                                                                 │
│  AudioEngine (future):                                          │
│    fn push(samples, port) → Result<count>                       │
│    fn pull(port, max) → Vec<samples>                            │
└─────────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────────┐
│                   PATCHBAY HARNESS                               │
│  Every port plugs into this — the routing matrix                │
│                                                                 │
│  fn connect(source_port, dest_port)    ← patch a cable          │
│  fn disconnect(port)                   ← pull a cable           │
│  fn status(port) → PortStatus          ← is it live             │
│  fn list_ports() → Vec<PortInfo>       ← what's available       │
└─────────────────────────────────────────────────────────────────┘
```

### How It All Connects

```
                    BEAM (nervous system)
                         │
            ┌────────────┼────────────┐
            ↓            ↓            ↓
     ┌──────────┐  ┌──────────┐  ┌──────────┐
     │ STT Ant  │  │ LLM Ant  │  │ TTS Ant  │    ← ANT HARNESS
     │ process()│  │ process()│  │ process()│
     └────┬─────┘  └────┬─────┘  └────┬─────┘
          │              │              │
          ↓              ↓              ↓
     ┌──────────┐  ┌──────────┐  ┌──────────┐
     │FluidAudio│  │ Claude   │  │ Kokoro   │    ← ENGINE HARNESS
     │transcribe│  │ ask()    │  │synthesize│
     └──────────┘  └──────────┘  └──────────┘

                    PATCHBAY HARNESS
            ┌───────────────────────────┐
            │  Port 0 ←→ Port 5        │
            │  Port 1 ←→ Port 12       │    ← routing matrix
            │  Port 3 ←→ Port 7        │
            └───────────────────────────┘
                         │
                    ┌────┴────┐
                    │BlackHole│
                    │ 16 ch   │
                    └─────────┘
```

### Onboarding a New Ant

Like hiring a new worker — they get the same uniform, same badge, same harness:

```
Step 1: Copy an existing ant file
        cp ants/tts.rs → ants/tts_walle.rs

Step 2: Change the guts (different engine, different config)
        use engines::vibevoice::VibeVoiceEngine  ← swap power tool

Step 3: It already speaks the harness
        impl Ant for TtsWalleAnt {
            fn process(msg) → AntMessage   ← same interface
            fn name() → "tts_walle"        ← different name
            fn status() → Ready            ← same status report
        }

Step 4: Register in ants/mod.rs
        pub mod tts_walle;

Step 5: Add NIF export in lib.rs
        fn tts_walle(text, voice) → tts_walle_inner(text, voice)

Step 6: Add Elixir stub in morsel_native.ex
        def tts_walle(_text, _voice), do: :erlang.nif_error(:not_loaded)

Step 7: It's on the bus. BEAM can route to it.
```

### Onboarding a New Engine

Like swapping an engine in a truck — same J1939 harness, different power plant:

```
Step 1: Create engines/vibevoice.rs

Step 2: Implement the trait
        impl TtsEngine for VibeVoiceEngine {
            fn synthesize(text, voice) → audio   ← same interface
            fn sample_rate() → 24000             ← same contract
        }

Step 3: Point the ant at it
        In ants/tts.rs:
            use engines::vibevoice::VibeVoiceEngine
            (instead of engines::kokoro::KokoroEngine)

Step 4: Done. Same ant, same harness, different engine.
        No other code changes. No Elixir changes.
        The BEAM doesn't even know the engine changed.
```

### The Truck Analogy

```
TRUCK (the Sovereign Pipeline)
├── CAN Bus (J1939)          = BEAM / Membrane
├── ECM                      = LLM ant + Claude engine
├── Fuel System              = STT ant + Parakeet engine
├── Exhaust System           = TTS ant + Kokoro engine
├── Sensors                  = Silence ant, Codec ant
├── Dashboard (OBD-II port)  = Observer, Flying Probe
└── Trailer Connector        = Patchbay / BlackHole

Swap the ECM → same J1939 connector
Swap the engine → same harness
Add a sensor → same CAN bus protocol
Read the dashboard → same OBD-II port
```

---

## The Registry — Self-Aware Ants, Elixir-Managed Roster

Each ant carries its own identity card in Rust — who it is, what it does, what it accepts. Elixir manages the roster — reads the cards, routes based on them, monitors health.

### The Split

```
RUST (self-awareness)                    ELIXIR (management)
──────────────────────                   ─────────────────────
Each ant knows:                          The BEAM does:
  "I am the TTS ant"                       "Show me all cards"
  "I accept text"                          "Who accepts audio?"
  "I output audio"                         "Route this to TTS"
  "My engine is Kokoro"                    "Is STT healthy?"
  "I'm currently Ready"                    "Restart codec ant"
  "Last call took 887ms"                   "Log all metrics"
```

### AntCard — The Identity Card (JSON-serializable)

```rust
// crate: ant-registry (shared across all ants)

pub struct AntCard {
    pub name: String,                        // "tts"
    pub rank: String,                        // "brain" | "codec" | "hardware"
    pub duties: Vec<String>,                 // ["synthesize speech", "voice selection"]
    pub input_types: Vec<String>,            // ["text"]
    pub output_types: Vec<String>,           // ["audio"]
    pub engine: String,                      // "kokoro"
    pub status: String,                      // "ready" | "busy" | "error" | "offline"
    pub metrics: AntMetrics,                 // live performance data
    pub metadata: HashMap<String, String>,   // extensible — bluetooth, sample rate, etc.
}

pub struct AntMetrics {
    pub calls: u64,                  // total calls served
    pub errors: u64,                 // total errors
    pub last_call_ms: u64,           // most recent call duration
    pub avg_call_ms: f64,            // running average
    pub total_bytes_in: u64,         // total data received
    pub total_bytes_out: u64,        // total data produced
}
```

### Registered Trait — Every Ant Carries Its Card

```rust
pub trait Registered {
    fn card(&self) -> AntCard;
}
```

Every ant implements this. When Elixir asks "who are you?", the ant hands over its card.

### How Elixir Reads the Roster

```elixir
# Pull all cards as JSON
cards = Jarvina.Morsel.Native.roster()
# → [
#   %{"name" => "stt",  "rank" => "brain",    "engine" => "fluidaudio",
#     "status" => "ready", "metrics" => %{"last_call_ms" => 84}},
#   %{"name" => "tts",  "rank" => "brain",    "engine" => "kokoro",
#     "status" => "ready", "metrics" => %{"last_call_ms" => 887}},
#   %{"name" => "llm",  "rank" => "brain",    "engine" => "claude",
#     "status" => "ready", "metrics" => %{"calls" => 42}},
#   %{"name" => "expand", "rank" => "codec",  "engine" => "native",
#     "status" => "ready"},
#   ...
# ]

# Pull one ant's card
card = Jarvina.Morsel.Native.card("stt")
# → %{"name" => "stt", "status" => "ready",
#     "metrics" => %{"last_call_ms" => 84, "calls" => 156}}

# Health check — all ants
Jarvina.Morsel.Native.health_check()
# → [{"stt", "ready"}, {"tts", "ready"}, {"llm", "ready"}, ...]
```

### The Roster at Boot

```
┌─────────────────────────────────────────────────────────────────┐
│                    ANT REGISTRY (boot sequence)                 │
│                                                                 │
│  1. Each ant initializes with its engine                        │
│  2. Each ant fills out its AntCard                              │
│  3. Registry collects all cards                                 │
│  4. Elixir pulls roster via NIF                                 │
│  5. Supervisor monitors health via health_check()               │
│  6. Dashboard / Telegram / Observer can display the roster      │
│                                                                 │
│  ┌──────┐ ┌──────┐ ┌──────┐ ┌──────┐ ┌──────┐ ┌──────┐       │
│  │ STT  │ │ TTS  │ │ LLM  │ │CODEC │ │SPEAK │ │LISTEN│       │
│  │card()│ │card()│ │card()│ │card()│ │card()│ │card()│       │
│  └──┬───┘ └──┬───┘ └──┬───┘ └──┬───┘ └──┬───┘ └──┬───┘       │
│     │        │        │        │        │        │             │
│     └────────┴────────┴────────┴────────┴────────┘             │
│                         │                                       │
│                    roster() NIF                                  │
│                         │                                       │
│                    ┌────┴────┐                                   │
│                    │ ELIXIR  │                                   │
│                    │ manages │                                   │
│                    └─────────┘                                   │
└─────────────────────────────────────────────────────────────────┘
```

### Why JSON

The card format is JSON-serializable so any consumer can read it:

| Consumer | How It Uses the Cards |
|----------|----------------------|
| Elixir Supervisor | Health monitoring, restart decisions |
| Membrane Pipeline | Route audio/text to the right ant |
| Telegram Bot | "Hey Emil, all ants are green" |
| Web Dashboard | Live status display |
| Flying Probe | "Probe the STT ant at its input" |
| Future Cody | Read the roster to understand the pipeline |

### The Mapifying Function Connection

This registry IS the mapifying function that Emil and Lyra designed:

```
AntCard maps to the Ant Profile:
  name       → NAME (UID)
  rank       → RANK (class + strength)
  duties     → DUTIES (current verb)
  input/output_types + metadata → CONNECTIONS (baton pass)
```

Every ant reports its card. The BEAM collects the cards. You see the live schematic.

---

## The Factory — JSON-Driven Struct Generation

Instead of hand-coding every ant struct, a JSON blueprint describes the ants and the factory generates the Rust code. Edit JSON, the lattice grows. No queen fingers typing struct after struct.

### The Inspiration: Terraform

Terraform (by HashiCorp) is "infrastructure as code" — you write a declaration file describing what you want, and the system builds it:

```
TERRAFORM                               ANT LATTICE
─────────                               ───────────
HCL file describes servers          →   JSON describes ants + connections
terraform apply builds them          →   build.rs generates Rust structs
terraform plan shows changes         →   roster() shows the live lattice
terraform state tracks what exists   →   AntRegistry tracks what's live
Change the file, reapply             →   Change the JSON, recompile
```

Terraform's key principle: **declarative, not imperative.** You don't say "step 1, step 2, step 3." You say "I want this end state" and the system figures out how to get there.

### The Blueprint — ants.json

```json
{
  "ants": [
    {
      "name": "stt",
      "rank": "brain",
      "input_types": ["audio"],
      "output_types": ["text"],
      "engine": "fluidaudio",
      "duties": ["transcribe speech"],
      "metadata": {
        "sample_rate": 16000,
        "provider": "CoreML"
      }
    },
    {
      "name": "tts",
      "rank": "brain",
      "input_types": ["text"],
      "output_types": ["audio"],
      "engine": "kokoro",
      "duties": ["synthesize speech", "voice selection"],
      "metadata": {
        "sample_rate": 24000,
        "default_voice": "af_heart"
      }
    },
    {
      "name": "llm",
      "rank": "brain",
      "input_types": ["text"],
      "output_types": ["text"],
      "engine": "claude",
      "duties": ["conversation", "reasoning"],
      "metadata": {
        "model": "claude-haiku-4-5",
        "max_tokens": 200
      }
    },
    {
      "name": "expand",
      "rank": "codec",
      "input_types": ["bytes"],
      "output_types": ["audio"],
      "engine": "native",
      "duties": ["mu-law to PCM"],
      "metadata": {}
    },
    {
      "name": "compress",
      "rank": "codec",
      "input_types": ["audio"],
      "output_types": ["bytes"],
      "engine": "native",
      "duties": ["PCM to mu-law"],
      "metadata": {}
    },
    {
      "name": "speaker",
      "rank": "hardware",
      "input_types": ["audio"],
      "output_types": ["blackhole"],
      "engine": "coreaudio",
      "duties": ["push to BlackHole channel"],
      "metadata": {
        "device": "BlackHole 16ch",
        "sample_rate": 48000
      }
    }
  ],

  "lattice": {
    "edges": [
      {"from": "stt",      "to": "llm"},
      {"from": "llm",      "to": "tts"},
      {"from": "tts",      "to": "speaker"},
      {"from": "tts",      "to": "compress"}
    ]
  }
}
```

### The Factory — build.rs Reads JSON, Writes Rust

```
┌─────────────┐         ┌──────────────┐         ┌─────────────────┐
│  ants.json  │────────→│  build.rs    │────────→│  generated.rs   │
│  (blueprint)│         │  (factory)   │         │  (Rust structs) │
└─────────────┘         └──────────────┘         └─────────────────┘
     human                  machine                   compiler
     edits                  reads +                   bakes into
     this                   generates                 binary
```

The factory generates:

```rust
// AUTO-GENERATED from ants.json — do not hand-edit

pub struct SttAnt { engine: FluidAudioEngine }
impl Ant for SttAnt {
    fn name(&self) -> &str { "stt" }
    fn process(&mut self, msg: AntMessage) -> Result<AntMessage, String> {
        // ... generated process logic
    }
    fn status(&self) -> AntStatus { AntStatus::Ready }
}
impl Registered for SttAnt {
    fn card(&self) -> AntCard {
        AntCard {
            name: "stt".into(),
            rank: "brain".into(),
            duties: vec!["transcribe speech".into()],
            input_types: vec!["audio".into()],
            output_types: vec!["text".into()],
            engine: "fluidaudio".into(),
            // ... all from JSON
        }
    }
}

pub struct TtsAnt { engine: KokoroEngine }
// ... same pattern, different data
```

### What the Factory Generates vs What You Hand-Write

| Generated from JSON | Hand-written |
|---------------------|-------------|
| Struct declarations | Engine internals (kokoro.rs, fluidaudio.rs) |
| AntCard data (name, rank, duties) | Process logic (what the ant actually does) |
| Registry entries | Hardware callbacks (CoreAudio) |
| Lattice wiring (edges) | Model loading, inference calls |
| Input/output type checking | Signal processing math |

The factory handles the **boilerplate**. You hand-write the **brains**.

### The Jacob's Lattice Connection

The JSON IS the lattice. The `edges` section is the wiring diagram. When you add a new ant to the JSON, the lattice grows:

```json
{
  "name": "tts_walle",
  "rank": "brain",
  "input_types": ["text"],
  "output_types": ["audio"],
  "engine": "kokoro_int8",
  "duties": ["synthesize speech", "Pi-5 character voice"],
  "metadata": {
    "sample_rate": 24000,
    "default_voice": "wall_e",
    "target_hardware": "raspberry_pi_5"
  }
}
```

Add that to `ants.json`, recompile, Wall-E is onboarded. The factory generated his struct, his card, his registration. You only write his engine (`kokoro_int8.rs`) if it's different from the existing Kokoro engine.

### The Self-Building Scaffold

```
TODAY:   Emil edits ants.json      → Factory generates structs → Compile → Run
FUTURE:  AI edits ants.json        → Factory generates structs → Compile → Run
VISION:  Lattice proposes new ant  → Emil approves JSON change → Auto-build
```

The scaffold builds upon itself. The JSON is the seed. The factory is the growth mechanism. The lattice is the living structure. Each new ant makes the whole lattice more capable, which enables more ants, which grows the lattice.

That's the hyper scaffold. That's Jacob's Lattice — not a ladder with fixed rungs, but a three-dimensional structure that grows in every direction.

---

## The Ant Mound — Struct Generator Engine

Instead of writing our own factory from scratch, we use proven Rust crates that already do schema-to-struct generation. The ant mound reads a JSON blueprint and stamps out ants.

### Proven Crates in the Ecosystem

| Crate | What It Does | Input Format |
|-------|-------------|--------------|
| `schema2struct` | JSON Schema → Rust structs with Serde | JSON Schema |
| `openapi-struct-gen` | OpenAPI YAML → Rust structs (build.rs) | OpenAPI/YAML |
| `rsgen-avro` | Avro Schema → Rust types | Apache Avro |
| `json_to_struct` | JSON → Rust structs | Raw JSON |

### Our Pick: schema2struct (or similar)

Why: procedural macro, JSON Schema input, Serde built in. Matches our `ants.json` blueprint pattern exactly.

```
ants.json (JSON Schema)  →  schema2struct  →  Rust structs with Serde
                                ↑
                          the ant mound
```

### The Mound — Where Ants Are Born

```
morsel-nif/
├── mound/                         ← the ant mound
│   ├── ants.json                  ← the blueprint (Emil edits this)
│   ├── engines.json               ← engine definitions
│   ├── lattice.json               ← connection wiring (DAG)
│   └── mound.rs                   ← the builder (reads JSON, generates structs)
│
├── src/
│   ├── generated/                 ← AUTO-GENERATED by the mound
│   │   ├── ant_structs.rs         ← struct SttAnt, TtsAnt, ...
│   │   ├── ant_cards.rs           ← impl Registered for each ant
│   │   └── ant_registry.rs        ← roster(), health_check()
│   └── ...
```

### How It Works

```
┌─────────────────────────────────────────────────────────────────┐
│                    THE ANT MOUND                                │
│                                                                 │
│  ┌───────────┐     ┌────────────┐     ┌──────────────────┐     │
│  │ants.json  │────→│ mound.rs   │────→│ generated/*.rs   │     │
│  │(blueprint)│     │ (builder)  │     │ (Rust structs)   │     │
│  └───────────┘     │            │     └──────────────────┘     │
│                    │ uses       │              │                │
│  ┌───────────┐     │ schema2    │              ↓                │
│  │engines.   │────→│ struct     │     ┌──────────────────┐     │
│  │json       │     │ crate      │     │ cargo build      │     │
│  └───────────┘     │            │     │ compiles into    │     │
│                    │            │     │ single .so NIF   │     │
│  ┌───────────┐     │            │     └──────────────────┘     │
│  │lattice.   │────→│            │                               │
│  │json       │     └────────────┘                               │
│  └───────────┘                                                  │
│                                                                 │
│  Edit JSON → Mound generates → Compile → Ants are born         │
└─────────────────────────────────────────────────────────────────┘
```

### Not Terraform — Right-Sized

```
TERRAFORM           200MB binary, manages 10,000 cloud servers
                    (freight train delivering a pizza)

schema2struct        Small crate, reads JSON Schema, writes Rust
                    (go-kart built for our track)

CUSTOM mound.rs      100 lines, reads OUR ants.json, writes OUR structs
                    (hand-built tool that fits our hand)
```

We don't need Terraform's cloud orchestration. We don't need OpenAPI's HTTP endpoint generation. We need a mound that reads a blueprint and stamps out ants with harness plugs and union cards.

### The Pattern Is Universal

Every serious system uses this pattern:

```
Kubernetes:    YAML manifests    → kubectl apply    → containers running
Docker:        Dockerfile        → docker build     → image built
Terraform:     HCL files         → terraform apply  → servers provisioned
Protobuf:      .proto files      → protoc           → structs in any language
Our Mound:     ants.json         → mound.rs         → Rust ants with harnesses
```

We're not inventing anything. We're using the same declarative-to-code pattern that runs the internet. Just for ants instead of servers.

### Onboarding With the Mound

Adding a new ant goes from 7 steps to 2:

```
OLD (manual onboarding):
  1. Write struct
  2. Implement Ant trait
  3. Implement Registered trait
  4. Write AntCard data
  5. Register in mod.rs
  6. Add NIF export
  7. Add Elixir stub

NEW (mound onboarding):
  1. Add entry to ants.json
  2. Recompile
  (mound generates steps 1-5 automatically)
  (steps 6-7 could also be generated)
```

---

## The Sovereign Data Layer — One-Stop Postgres Shop

One PostgreSQL instance on the M1. No cloud. No external services. Five capabilities in one database:

```
┌──────────────────────────────────────────────────────────────────┐
│              POSTGRES — The One-Stop Sovereign Shop              │
│                                                                  │
│  ┌────────────┐  ┌────────────┐  ┌────────────┐                │
│  │  pgvector  │  │ Apache AGE │  │   Avro     │                │
│  │  semantic  │  │   graph    │  │  schemas   │                │
│  │  search    │  │  queries   │  │  (cookie   │                │
│  │ (find by   │  │ (find by   │  │  cutters)  │                │
│  │  meaning)  │  │ connection)│  │            │                │
│  └────────────┘  └────────────┘  └────────────┘                │
│                                                                  │
│  ┌────────────┐  ┌────────────┐                                 │
│  │  Tables    │  │  Ecto      │                                 │
│  │  registry  │  │  (Elixir   │                                 │
│  │  metrics   │  │   ORM)     │                                 │
│  │  sessions  │  │            │                                 │
│  └────────────┘  └────────────┘                                 │
│                                                                  │
│  All on your M1. Sovereign. No cloud dependency.                │
└──────────────────────────────────────────────────────────────────┘
```

### Five Capabilities, One Database

| Capability | Extension | What It Does |
|-----------|-----------|-------------|
| **Relational** | Core Postgres | Ant registry, metrics, sessions, config |
| **Semantic Search** | pgvector | Find conversations by meaning, not keywords |
| **Graph Queries** | Apache AGE | DAG wiring, ant connections, lattice structure |
| **Schema Registry** | Avro (JSONB) | Cookie cutters for the ant mound factory |
| **Schema Evolution** | Avro | Add fields to ants without breaking others |

### Why Avro Instead of Plain JSON Schema

```
JSON Schema:  static, no versioning, breaks on changes
Avro Schema:  versioned, backward compatible, schema evolution

Version 1:  TtsAnt { name, engine, voice }
Version 2:  TtsAnt { name, engine, voice, speed }     ← added field
Version 3:  TtsAnt { name, engine, voice, speed, emotion }  ← added field

Old ants still work. New ants get new fields. No breaking changes.
```

And `rsgen-avro` (Rust crate) transforms Avro schemas directly into Rust types with Serde. The mound reads Avro from Postgres, generates structs.

### The Database Schema

```sql
-- Ant cookie cutters (Avro schemas stored as JSONB)
CREATE TABLE ant_schemas (
    name        TEXT PRIMARY KEY,        -- "tts", "stt", "llm"
    version     INTEGER DEFAULT 1,       -- schema evolution
    rank        TEXT,                     -- "brain", "codec", "hardware"
    avro_schema JSONB NOT NULL,          -- the cookie cutter
    created_at  TIMESTAMP DEFAULT NOW(),
    updated_by  TEXT                      -- "emil", "cody", "lyra"
);

-- Engine registry
CREATE TABLE engine_schemas (
    name        TEXT PRIMARY KEY,        -- "kokoro", "fluidaudio", "claude"
    engine_type TEXT,                     -- "stt", "tts", "llm"
    runtime     TEXT,                     -- "ort/CoreML", "FluidAudio", "HTTP"
    config      JSONB,                   -- model paths, sample rates, etc.
    status      TEXT DEFAULT 'ready'
);

-- Ant metrics (live performance data)
CREATE TABLE ant_metrics (
    ant_name    TEXT REFERENCES ant_schemas(name),
    timestamp   TIMESTAMP DEFAULT NOW(),
    calls       BIGINT,
    errors      BIGINT,
    last_ms     INTEGER,
    avg_ms      FLOAT
);

-- Conversations with vector embeddings
CREATE TABLE conversations (
    id          SERIAL PRIMARY KEY,
    session_id  TEXT,
    speaker     TEXT,                     -- "caller", "jarvina"
    text        TEXT,
    embedding   vector(384),             -- pgvector semantic embedding
    timestamp   TIMESTAMP DEFAULT NOW()
);

-- Graph: ant connections (Apache AGE)
-- CREATE with Cypher:
--   CREATE (stt:Ant {name:'stt'})
--   CREATE (llm:Ant {name:'llm'})
--   CREATE (tts:Ant {name:'tts'})
--   CREATE (stt)-[:FEEDS {format:'text'}]->(llm)
--   CREATE (llm)-[:FEEDS {format:'text'}]->(tts)
```

### Query Examples — One Database, All Questions

```sql
-- RELATIONAL: "What ants exist?"
SELECT name, rank, version FROM ant_schemas;

-- RELATIONAL: "How fast is the TTS ant?"
SELECT avg_ms, calls, errors FROM ant_metrics WHERE ant_name = 'tts';

-- GRAPH (AGE): "What's the signal chain?"
SELECT * FROM cypher('ant_lattice', $$
    MATCH path = (a:Ant)-[:FEEDS*]->(b:Ant)
    WHERE a.name = 'stt'
    RETURN path
$$) as (path agtype);
-- → stt → llm → tts → speaker

-- GRAPH (AGE): "What feeds into the mixer?"
SELECT * FROM cypher('ant_lattice', $$
    MATCH (a:Ant)-[:FEEDS]->(mixer:Ant {name: 'mixer'})
    RETURN a.name
$$) as (source TEXT);

-- VECTOR (pgvector): "Find similar conversations"
SELECT text, 1 - (embedding <=> $1) as similarity
FROM conversations
ORDER BY similarity DESC LIMIT 5;
-- → "What year was the Eiffel Tower built?" (0.94 similarity)

-- VECTOR + GRAPH: "What did callers ask about that reached the TTS ant?"
SELECT c.text, c.timestamp
FROM conversations c
JOIN cypher('ant_lattice', $$
    MATCH (stt:Ant {name:'stt'})-[:FEEDS*]->(tts:Ant {name:'tts'})
    RETURN stt.name
$$) as (ant TEXT) ON TRUE
WHERE c.speaker = 'caller'
ORDER BY c.timestamp DESC;
```

### The Full Flow — Postgres to Running Ants and Back

```
┌──────────────────────────────────────────────────────────────────┐
│                                                                  │
│  POSTGRES (the one-stop sovereign shop)                         │
│  ┌─────────┐  ┌─────────┐  ┌─────────┐  ┌─────────┐          │
│  │  Avro   │  │  AGE    │  │pgvector │  │ Tables  │          │
│  │ schemas │  │ graph   │  │ vectors │  │ metrics │          │
│  └────┬────┘  └────┬────┘  └────┬────┘  └────┬────┘          │
│       │            │            │            │                  │
└───────┼────────────┼────────────┼────────────┼──────────────────┘
        │            │            │            ↑
        ↓            │            │            │
  ┌───────────┐      │            │      ┌─────┴─────┐
  │ rsgen-avro│      │            │      │  Metrics   │
  │ (Rust)    │      │            │      │  flow back │
  └─────┬─────┘      │            │      └─────┬─────┘
        │            │            │            ↑
        ↓            │            │            │
  ┌───────────┐      │            │      ┌─────┴─────┐
  │ Generated │      │            │      │  Running   │
  │ Rust      │      │            │      │  ants in   │
  │ structs   │      │            │      │  BEAM      │
  └─────┬─────┘      │            │      └─────┬─────┘
        │            │            │            ↑
        ↓            ↓            │            │
  ┌──────────────────────────┐    │      ┌─────┴─────┐
  │  Compiled NIF binary     │    │      │ Elixir    │
  │  (ants with harnesses)   │────┘      │ Ecto ORM  │
  └──────────────────────────┘           └───────────┘
        │                                      ↑
        └──────────────────────────────────────┘
              conversations, metrics, logs
              all flow back to Postgres
```

### The Self-Learning Loop

```
STEP 1: Conversation happens
        Caller: "What year was the Eiffel Tower built?"
        Jarvina: "The Eiffel Tower was built in 1889."

STEP 2: Stored in Postgres
        → text in conversations table
        → embedding in pgvector (semantic meaning)
        → graph edge: caller → stt → llm → tts (which ants processed it)
        → metrics: stt took 84ms, tts took 887ms

STEP 3: Patterns emerge over time
        → pgvector: "50 callers asked about landmarks"
        → AGE graph: "all landmark questions go stt → llm → tts,
                       but no caching ant exists"
        → Suggestion: "Create a cache ant for frequent topics"

STEP 4: New ant onboarded
        → Add Avro schema to ant_schemas table
        → rsgen-avro generates the struct
        → Mound stamps it out
        → AGE graph updated: stt → cache → llm (cache miss) or cache → tts (cache hit)
        → Lattice grew from its own conversations
```

### The Truck Stop Analogy

```
POSTGRES is the truck stop:
├── Fuel pumps (Avro)     → power to generate new ants
├── Dispatch board (AGE)  → who connects to who, which route
├── Logbook (tables)      → metrics, sessions, history
├── CB radio (pgvector)   → "anyone seen something like this?"
└── All under one roof    → no driving to 5 different stops
```

### What Lives Where

| Data | Postgres Feature | Why There |
|------|-----------------|-----------|
| Ant cookie cutters | JSONB (Avro schemas) | Versioned, evolvable |
| Connection wiring | Apache AGE graph | DAG queries, path finding |
| Conversation memory | pgvector embeddings | Semantic search |
| Performance metrics | Tables + time series | Track ant health |
| Session journals | Tables | Chronological record |
| AI family messages | Tables + pgvector | Cross-AI semantic search |
| Engine configs | JSONB | Model paths, sample rates |
| Voice embeddings | pgvector | Find/clone similar voices |

---

*Morsel is the ant mound. Each ant is a Lego block. The engines are swappable power tools. The harness is the standard. The registry is the dispatch board. The mound stamps out ants from Avro blueprints. Postgres is the one-stop sovereign shop. The BEAM is the nervous system. The lattice learns from itself. J1939 for ants.*
