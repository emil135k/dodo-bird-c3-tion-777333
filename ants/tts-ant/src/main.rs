//! TTS Ant — Kokoro v1.0 ONNX + misaki-rs G2P
//! Standalone daemon. iceoryx2 zero-copy IPC.

use iceoryx2::prelude::*;
use once_cell::sync::Lazy;
use ort::{session::Session, execution_providers::CoreMLExecutionProvider, value::Tensor};
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};

const KOKORO_MODEL: &str = "/Users/rocketman/crystalballmini/downloads/kokoro-v1.0.fp16-gpu.onnx";
const VOICES_DIR: &str = "/Users/rocketman/crystalballmini/voices-v1.0";

struct Engine { session: Session }

static ENGINE: Lazy<Mutex<Engine>> = Lazy::new(|| {
    eprintln!("[TTS-ANT] Loading Kokoro v1.0 ONNX with CoreML...");
    let session = Session::builder()
        .expect("session builder")
        .with_execution_providers([CoreMLExecutionProvider::default().build()])
        .expect("CoreML provider")
        .commit_from_file(KOKORO_MODEL)
        .expect("load Kokoro v1.0 ONNX");
    eprintln!("[TTS-ANT] Model: READY");
    Mutex::new(Engine { session })
});

static G2P_ENGINE: Lazy<Mutex<misaki_rs::G2P>> = Lazy::new(|| {
    let engine = misaki_rs::G2P::new(misaki_rs::Language::EnglishUS);
    eprintln!("[TTS-ANT] G2P: READY");
    Mutex::new(engine)
});

fn load_voice(name: &str) -> Result<Vec<f32>, String> {
    let path = format!("{}/{}.bin", VOICES_DIR, name);
    let data = std::fs::read(&path).map_err(|e| format!("{}: {}", name, e))?;
    if data.len() != 522240 { return Err("wrong voice size".into()); }
    Ok(data.chunks(4).map(|c| f32::from_le_bytes([c[0],c[1],c[2],c[3]])).collect())
}

/// Split text into sentence-sized chunks that stay under the model's ~500 token limit.
/// Splits on sentence boundaries (. ! ?) then merges short sentences together.
fn split_sentences(text: &str) -> Vec<String> {
    let mut chunks = Vec::new();
    let mut current = String::new();

    for part in text.split_inclusive(|c: char| c == '.' || c == '!' || c == '?') {
        if current.len() + part.len() > 300 {
            if !current.is_empty() {
                chunks.push(current.clone());
                current.clear();
            }
            if part.len() > 300 {
                // Single sentence too long — split on commas or dashes
                for sub in part.split_inclusive(|c: char| c == ',' || c == ';' || c == '—') {
                    if current.len() + sub.len() > 300 && !current.is_empty() {
                        chunks.push(current.clone());
                        current.clear();
                    }
                    current.push_str(sub);
                }
            } else {
                current.push_str(part);
            }
        } else {
            current.push_str(part);
        }
    }
    if !current.is_empty() {
        chunks.push(current);
    }
    if chunks.is_empty() {
        chunks.push(text.to_string());
    }
    chunks
}

fn synthesize(text: &str, voice: &str) -> Result<Vec<f32>, String> {
    let g2p = G2P_ENGINE.lock().map_err(|e| e.to_string())?;
    let (phonemes, _) = g2p.g2p(text).map_err(|e| e.to_string())?;
    let ids: Vec<i64> = kokoro_g2p::kokoro::phonemes_to_ids(&phonemes).iter().map(|&id| id as i64).collect();
    if ids.len() < 3 { return Err("too few tokens".into()); }
    if ids.len() > 2000 { return Err(format!("too many tokens ({}) — max 2000", ids.len())); }

    let voice_data = load_voice(voice)?;
    let idx = ids.len().min(509);
    let style: Vec<f32> = voice_data[idx*256..(idx+1)*256].to_vec();

    let mut engine = ENGINE.lock().map_err(|e| e.to_string())?;
    let mut padded = vec![0i64];
    padded.extend_from_slice(&ids);
    padded.push(0i64);
    let plen = padded.len();

    let tokens = Tensor::from_array(([1, plen], padded)).map_err(|e| e.to_string())?;
    let style_t = Tensor::from_array(([1, 256usize], style)).map_err(|e| e.to_string())?;
    let speed_t = Tensor::from_array(([1usize], vec![1.0f32])).map_err(|e| e.to_string())?;

    let t = std::time::Instant::now();
    let out = engine.session.run(ort::inputs![tokens, style_t, speed_t]).map_err(|e| e.to_string())?;
    let ms = t.elapsed().as_millis();
    let (_, raw) = out[0].try_extract_tensor::<f32>().map_err(|e| e.to_string())?;
    let samples: Vec<f32> = raw.iter().copied().collect();
    eprintln!("[TTS-ANT] {:.1}s audio in {}ms", samples.len() as f64 / 24000.0, ms);
    Ok(samples)
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    eprintln!("[TTS-ANT] Starting...");
    { let _e = ENGINE.lock(); }
    { let _g = G2P_ENGINE.lock(); }

    let mut iox = Config::default();
    iox.global.set_root_path(&iceoryx2_bb_system_types::path::Path::new(b"/tmp/iceoryx2/").unwrap());
    let node = NodeBuilder::new().config(&iox).create::<ipc::Service>()?;

    // Contract: tts_text contains assistant replies (UTF-8) from llm-ant.
    // Format: "voice_name:text" or plain text (defaults to af_heart).
    let text_svc = node.service_builder(&"tts_text".try_into()?)
        .publish_subscribe::<[u8]>()
        .open_or_create()?;
    let sub = text_svc.subscriber_builder().create()?;

    // Contract: tts_audio contains f32 PCM at 24kHz mono.
    let audio_svc = node.service_builder(&"tts_audio".try_into()?)
        .publish_subscribe::<[u8]>()
        .open_or_create()?;
    let pub_ = audio_svc.publisher_builder().initial_max_slice_len(16 * 1024 * 1024).create()?;

    // Subscribe to speaker_control — abort synthesis on FLUSH
    let ctl_svc = node.service_builder(&"speaker_control".try_into()?)
        .publish_subscribe::<[u8]>().open_or_create()?;
    let ctl_sub = ctl_svc.subscriber_builder().create()?;

    static INTERRUPTED: AtomicBool = AtomicBool::new(false);

    eprintln!("[TTS-ANT] Bus: sub='tts_text'+'speaker_control' pub='tts_audio' — READY");

    loop {
        // Check for flush/interrupt commands
        while let Some(ctl) = ctl_sub.receive()? {
            if let Some(&cmd) = ctl.payload().first() {
                if cmd == 0x01 { // FLUSH
                    INTERRUPTED.store(true, Ordering::Relaxed);
                    // Drain all pending tts_text messages — they're stale
                    let mut drained = 0;
                    while let Some(_) = sub.receive()? { drained += 1; }
                    eprintln!("[TTS-ANT] FLUSH received — aborting synthesis, drained {} pending messages", drained);
                }
            }
        }
        while let Some(sample) = sub.receive()? {
            let payload = sample.payload();
            eprintln!("[TTS-ANT] GOT DATA: {} bytes", payload.len());

            let text = match std::str::from_utf8(payload) {
                Ok(t) => t.trim().to_string(),
                Err(_) => continue,
            };
            if text.is_empty() { continue; }

            // Voice prefix: only accept if prefix is a valid voice file (no spaces, short)
            let (voice, speech) = if let Some(idx) = text.find(':') {
                let prefix = &text[..idx];
                if prefix.len() < 20 && !prefix.contains(' ') && load_voice(prefix).is_ok() {
                    (prefix.to_string(), text[idx+1..].to_string())
                } else {
                    ("af_heart".to_string(), text)
                }
            } else {
                ("af_heart".to_string(), text)
            };

            eprintln!("[TTS-ANT] Synth: \"{}\" voice={}", speech.chars().take(50).collect::<String>(), voice);

            // Clear interrupt flag for new message
            INTERRUPTED.store(false, Ordering::Relaxed);

            // Split long text into sentence chunks to stay within model limits
            // Publish each chunk separately so patchbay can play them sequentially
            let chunks = split_sentences(&speech);

            for (i, chunk) in chunks.iter().enumerate() {
                // Check for flush between chunks
                while let Some(ctl) = ctl_sub.receive()? {
                    if let Some(&cmd) = ctl.payload().first() {
                        if cmd == 0x01 {
                            INTERRUPTED.store(true, Ordering::Relaxed);
                            eprintln!("[TTS-ANT] FLUSH received — aborting remaining chunks");
                        }
                    }
                }
                if INTERRUPTED.load(Ordering::Relaxed) {
                    eprintln!("[TTS-ANT] Interrupted — skipping chunk {}/{}", i+1, chunks.len());
                    break;
                }

                if chunk.trim().is_empty() { continue; }
                if chunks.len() > 1 {
                    eprintln!("[TTS-ANT]   chunk {}/{}: \"{}\"", i+1, chunks.len(), chunk.chars().take(40).collect::<String>());
                }
                match synthesize(chunk, &voice) {
                    Ok(mut samples) => {
                        // Re-check after synthesis (takes 4-5 seconds)
                        while let Some(ctl) = ctl_sub.receive()? {
                            if let Some(&cmd) = ctl.payload().first() {
                                if cmd == 0x01 {
                                    INTERRUPTED.store(true, Ordering::Relaxed);
                                    eprintln!("[TTS-ANT] FLUSH during synthesis — dropping chunk {}", i+1);
                                }
                            }
                        }
                        if INTERRUPTED.load(Ordering::Relaxed) {
                            eprintln!("[TTS-ANT] Interrupted — dropping synthesized chunk {}/{}", i+1, chunks.len());
                            break;
                        }

                        let peak = samples.iter().map(|s| s.abs()).fold(0.0f32, f32::max);
                        if peak > 0.001 && peak < 0.85 {
                            let gain = 0.9 / peak;
                            for s in samples.iter_mut() { *s *= gain; }
                        }
                        let bytes: Vec<u8> = samples.iter().flat_map(|s| s.to_le_bytes()).collect();
                        let sample = pub_.loan_slice_uninit(bytes.len())?;
                        let sample = sample.write_from_slice(&bytes);
                        sample.send()?;
                        eprintln!("[TTS-ANT] Published chunk {}: {} samples", i+1, samples.len());
                    }
                    Err(e) => { eprintln!("[TTS-ANT] Error on chunk {}: {} — skipping", i+1, e); continue; }
                }
            }
        }
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
}
