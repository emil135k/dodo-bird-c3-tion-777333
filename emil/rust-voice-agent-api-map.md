# Rust Voice Agent — API Map & Architecture
## Research Report for sherpa-onnx + pipewire-rs
### Sparked Matter LLC — March 29, 2026

---

## CRITICAL GOTCHAS (Read First!)

### 1. sherpa-onnx CUDA on Jetson
The Rust crate auto-downloads **CPU-ONLY** libraries for aarch64.
For CUDA on Jetson, you MUST:
```bash
# Point to our existing GPU libraries
export SHERPA_ONNX_LIB_DIR=/home/rocketman/downloads/sherpa-onnx-v1.12.34-linux-aarch64-shared-gpu-onnxruntime-1.18.1/lib
cargo build --features shared
```
Use `--features shared` (NOT the default `static`). No static CUDA build exists for aarch64.

### 2. Thread Safety
| Type | Send | Sync | Rule |
|------|------|------|------|
| VoiceActivityDetector | NO | NO | Single thread only |
| OfflineRecognizer | NO | NO | Single thread only |
| OfflineTts | YES | NO | Can move to thread |
| LinearResampler | NO | NO | Single thread only |

ALL VAD + STT must live on the SAME thread. Cannot process inside PipeWire's RT callback.

### 3. PipeWire Process Callback
The process callback runs in a real-time thread. It must be FAST:
- No allocations
- No blocking
- No CUDA calls
- Just copy samples to a ring buffer and return

---

## Architecture (Three Threads)

```
Thread 1 (PipeWire ThreadLoop): Audio I/O
  ├─ Capture callback: copy mic samples → ring buffer
  └─ Playback callback: read from playback ring buffer → speaker

Thread 2 (Processing): VAD + STT + LLM (all !Send, created here)
  ├─ Read from capture ring buffer
  ├─ Feed to VAD → detect speech segments
  ├─ Feed segments to STT (Parakeet, CUDA)
  ├─ Send text to LLM (libcurl/reqwest)
  └─ Receive LLM response → send to TTS thread

Thread 3 (TTS): Kokoro generation (OfflineTts is Send)
  ├─ Receive text from processing thread
  ├─ Generate audio (Kokoro, CUDA)
  └─ Push samples to playback ring buffer

Communication: lock-free ring buffers or channels between threads
```

---

## pipewire-rs Audio Capture

### Use ThreadLoop (non-blocking)
```rust
let thread_loop = pw::thread_loop::ThreadLoopBox::new(None, None)?;
// Create context/core/streams under lock
{
    let _guard = thread_loop.lock();
    // ... setup ...
}
thread_loop.start(); // Runs PipeWire in background thread
```

### Target Echo-Cancelled Source
```rust
let mut props = properties! {
    *pw::keys::MEDIA_TYPE => "Audio",
    *pw::keys::MEDIA_CATEGORY => "Capture",
    *pw::keys::MEDIA_ROLE => "Communication",
};
props.insert(*pw::keys::TARGET_OBJECT, "echo_cancel_source");
```

### Specify Audio Format (F32LE)
```rust
let mut audio_info = spa::param::audio::AudioInfoRaw::new();
audio_info.set_format(spa::param::audio::AudioFormat::F32LE);
// Serialize to pod for stream.connect()
let obj = pw::spa::pod::Object { ... };
let values = PodSerializer::serialize(...);
let mut params = [Pod::from_bytes(&values).unwrap()];
```

### Process Callback (Capture)
```rust
.process(|stream, user_data| {
    if let Some(mut buffer) = stream.dequeue_buffer() {
        let datas = buffer.datas_mut();
        if let Some(d) = datas.first_mut() {
            let chunk_size = d.chunk().size() as usize;
            if let Some(bytes) = d.data() {
                // Copy to ring buffer — DON'T process here!
                let n_samples = chunk_size / mem::size_of::<f32>();
                // ... copy to shared ring buffer ...
            }
        }
    }
})
```

---

## pipewire-rs Audio Playback

### Target Echo-Cancelled Sink
```rust
let props = properties! {
    *pw::keys::MEDIA_TYPE => "Audio",
    *pw::keys::MEDIA_CATEGORY => "Playback",
    *pw::keys::MEDIA_ROLE => "Communication",
    *pw::keys::TARGET_OBJECT => "echo_cancel_sink",
};
```

### Process Callback (Playback — Direction::Output)
```rust
.process(|stream, user_data| {
    if let Some(mut buffer) = stream.dequeue_buffer() {
        let datas = buffer.datas_mut();
        if let Some(d) = datas.first_mut() {
            if let Some(slice) = d.data() {
                // Read from playback ring buffer, write to slice
                let chunk = d.chunk_mut();
                *chunk.size_mut() = bytes_written as u32;
                *chunk.offset_mut() = 0;
            }
        }
    }
})
```

### Barge-in: Stop Playback
```rust
// From processing thread:
let _guard = thread_loop.lock();
stream.set_active(false);  // Stop playback instantly
```

### Detect Playback Complete
```rust
.drained(|stream, user_data| {
    user_data.playback_complete = true;
})
// Call stream.flush(true) when samples run out
```

---

## sherpa-onnx Rust API

### VAD
```rust
let vad = VoiceActivityDetector::create(&config, 60.0)?;
vad.accept_waveform(&samples_f32);  // f32, [-1,1], 16kHz
while !vad.is_empty() {
    let seg = vad.front().unwrap();
    // seg.samples(), seg.start(), seg.n()
    vad.pop();
}
```

### STT (Parakeet)
```rust
let recognizer = OfflineRecognizer::create(&config)?;
let stream = recognizer.create_stream();
stream.accept_waveform(16000, &speech_f32);
recognizer.decode(&stream);
let result = stream.get_result().unwrap();
println!("{}", result.text);
```

### TTS (Kokoro) — with streaming callback!
```rust
let tts = OfflineTts::create(&config)?;
let audio = tts.generate_with_config(
    "Hello Emil",
    &GenerationConfig { sid: 3, speed: 1.0, ..Default::default() },
    Some(|chunk: &[f32], progress: f32| -> bool {
        // Push chunk to playback ring buffer in real-time!
        // Return true to continue, false to abort (barge-in!)
        true
    }),
)?;
```

### Resampler
```rust
let resampler = LinearResampler::create(24000, 16000)?;
let resampled = resampler.resample(&samples, false); // false = more coming
let final_chunk = resampler.resample(&last, true);    // true = flush
```

### Denoiser
```rust
let denoiser = OfflineSpeechDenoiser::create(&config)?;
let clean = denoiser.run(&noisy_samples, 16000);
// clean.samples: Vec<f32>
```

---

## Key Insights

1. **TTS streaming callback** = natural barge-in! Return `false` from the callback to stop generation.
2. **F32LE capture from PipeWire** avoids S16→float conversion. Use LinearResampler for 48kHz→16kHz.
3. **ThreadLoop** is mandatory — MainLoop blocks. Full duplex with two streams on same core.
4. **TARGET_OBJECT** property routes to echo_cancel_source/sink — same as pw-link but automatic.
5. **SHERPA_ONNX_LIB_DIR** must point to our existing GPU libraries — the crate won't download CUDA for ARM.
