//! TTS Ant — Kokoro v1.0 ONNX + misaki-rs G2P
//! Standalone daemon. iceoryx2 zero-copy IPC.

use iceoryx2::prelude::*;
use once_cell::sync::Lazy;
use ort::{session::Session, execution_providers::CoreMLExecutionProvider, value::Tensor};
use std::sync::Mutex;

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

fn synthesize(text: &str, voice: &str) -> Result<Vec<f32>, String> {
    let g2p = G2P_ENGINE.lock().map_err(|e| e.to_string())?;
    let (phonemes, _) = g2p.g2p(text).map_err(|e| e.to_string())?;
    let ids: Vec<i64> = kokoro_g2p::kokoro::phonemes_to_ids(&phonemes).iter().map(|&id| id as i64).collect();
    if ids.len() < 3 { return Err("too few tokens".into()); }

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

    let node = NodeBuilder::new().create::<ipc::Service>()?;

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
    let pub_ = audio_svc.publisher_builder().initial_max_slice_len(4 * 1024 * 1024).create()?;

    eprintln!("[TTS-ANT] Bus: sub='tts_text' pub='tts_audio' — READY");

    loop {
        while let Some(sample) = sub.receive()? {
            let payload = sample.payload();
            eprintln!("[TTS-ANT] GOT DATA: {} bytes", payload.len());

            let text = match std::str::from_utf8(payload) {
                Ok(t) => t.trim().to_string(),
                Err(_) => continue,
            };
            if text.is_empty() { continue; }

            let (voice, speech) = if let Some(idx) = text.find(':') {
                (text[..idx].to_string(), text[idx+1..].to_string())
            } else {
                ("af_heart".to_string(), text)
            };

            eprintln!("[TTS-ANT] Synth: \"{}\" voice={}", speech.chars().take(50).collect::<String>(), voice);

            match synthesize(&speech, &voice) {
                Ok(samples) => {
                    let bytes: Vec<u8> = samples.iter().flat_map(|s| s.to_le_bytes()).collect();
                    let sample = pub_.loan_slice_uninit(bytes.len())?;
                    let sample = sample.write_from_slice(&bytes);
                    sample.send()?;
                    eprintln!("[TTS-ANT] Published {} samples", samples.len());
                }
                Err(e) => eprintln!("[TTS-ANT] Error: {}", e),
            }
        }
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
}
