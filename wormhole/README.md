# iceoryx2-Swift Wormhole

A pipe-based bridge between Rust (iceoryx2 zero-copy IPC) and Swift (Apple CoreML / AVAudioEngine).

Zero FFI. Zero shared libraries. Zero build coupling. Two processes. One pipe.

## The Pattern

```
Rust (iceoryx2 bus)                    Swift (Apple frameworks)
    │                                       │
    │  stdin:  [i32 count LE][payload...]   │
    ├─────────────────────────────────────→ │
    │                                       │
    │  stdout: [i32 count LE][payload...]   │
    │ ←────────────────────────────────────┤
    │                                       │
    │  Handshake: Swift sends "<ready>"    │
    │  before Rust subscribes to bus       │
```

Rust spawns the Swift binary as a child process. They communicate through anonymous Unix pipes with a minimal binary protocol. Each side does what it does best:

- **Rust**: iceoryx2 zero-copy IPC, bus routing, config, lifecycle management
- **Swift**: CoreML Neural Engine inference, AVAudioEngine audio I/O, Apple AEC

## Examples

### stt-example — Speech-to-Text via CoreML
- Rust subscribes to `stt_audio` bus, forwards PCM to Swift
- Swift runs Parakeet CoreML on the Apple Neural Engine
- Swift returns transcribed text through stdout pipe
- Rust publishes text to `stt_text` bus

### audio-example — Audio I/O with Apple Echo Cancellation
- Swift runs AVAudioEngine with `setVoiceProcessingEnabled(true)`
- Apple handles acoustic echo cancellation at the driver level
- Swift captures echo-cancelled mic audio, sends through stdout pipe
- Rust receives TTS audio from `tts_audio` bus, sends through stdin pipe
- Swift plays TTS through AVAudioEngine's playerNode

## Why Not FFI?

| Approach | Build Coupling | ABI Fragility | Debug Story | Complexity |
|----------|---------------|---------------|-------------|------------|
| FFI / C bridge | High | High | Poor | High |
| xcframework | Medium | Medium | Poor | Medium |
| **Pipe wormhole** | **None** | **None** | **Excellent** | **Low** |

The pipe wormhole requires no shared headers, no bridging modules, no framework embedding. Each side compiles independently. You can replace the Swift worker without recompiling Rust, and vice versa.

## Requirements

- macOS 14+ (Sonoma)
- Apple Silicon (M1/M2/M3/M4)
- Rust + iceoryx2 0.8
- Swift 5.10+
- Xcode Command Line Tools

## Origin

Created by Emil Rivas and the AI Family (Cody, Airy, Lyra, Vale, Ara) at Sparked Matter LLC.

Built in a Hawk camper. Powered by sovereign engineering.

*"Code with Soul and Spirit, Powered by Joy"*
