//! Silero Ant — VAD + signal conditioning, configurable via JSON
//!
//! Subscribes to stt_raw (raw mic audio from Patchbay at native 48kHz)
//! Publishes complete utterances to stt_audio (for STT ant)
//! Uses silero-vad-rust v6 which accepts 48kHz natively
//!
//! Data flow: Patchbay → [stt_raw] → Silero → [stt_audio] → STT

use iceoryx2::prelude::*;
use iceoryx2_bb_system_types::path::Path;
use iceoryx2_bb_container::semantic_string::SemanticString;
use silero_vad_rust::silero_vad::model::{load_silero_vad, OnnxModel};
use serde::Deserialize;

const SAMPLE_RATE: u32 = 48000;
const STT_RATE: u32 = 16000;
const DECIMATE: usize = (SAMPLE_RATE / STT_RATE) as usize; // 3
const CHUNK_SIZE: usize = 1536; // 512 * 3 — model decimates 48k→16k internally, needs 512 at 16k
const CONFIG_PATH: &str = "/Users/rocketman/crystalballmini/hypAiAssist/config/silero-ant.json";

#[derive(Deserialize, Debug)]
struct SileroConfig {
    #[serde(default = "d_thresh")]
    threshold: f32,
    #[serde(default = "d_silence")]
    silence_frames_to_end: usize,
    #[serde(default = "d_min")]
    min_utterance_ms: u32,
    #[serde(default = "d_max")]
    max_utterance_ms: u32,
}
fn d_thresh() -> f32 { 0.5 }
fn d_silence() -> usize { 20 }
fn d_min() -> u32 { 500 }
fn d_max() -> u32 { 10000 }

impl Default for SileroConfig {
    fn default() -> Self {
        Self { threshold: d_thresh(), silence_frames_to_end: d_silence(),
               min_utterance_ms: d_min(), max_utterance_ms: d_max() }
    }
}

impl SileroConfig {
    fn load() -> Self {
        match std::fs::read_to_string(CONFIG_PATH) {
            Ok(text) => serde_json::from_str(&text).unwrap_or_else(|e| {
                eprintln!("[SILERO] Config parse error: {} — defaults", e);
                Self::default()
            }),
            Err(_) => { eprintln!("[SILERO] No config file — defaults"); Self::default() }
        }
    }
    fn min_samples(&self) -> usize { (self.min_utterance_ms as usize * SAMPLE_RATE as usize) / 1000 }
    fn max_samples(&self) -> usize { (self.max_utterance_ms as usize * SAMPLE_RATE as usize) / 1000 }
}

#[derive(PartialEq)]
enum State { Silence, Speech, Trailing }

fn normalize(samples: &mut [f32]) {
    let peak = samples.iter().map(|s| s.abs()).fold(0.0f32, f32::max);
    if peak > 0.001 { let g = 0.9 / peak; for s in samples.iter_mut() { *s *= g; } }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    eprintln!("[SILERO] Starting — Silero VAD v6, {}Hz native...", SAMPLE_RATE);
    let cfg = SileroConfig::load();
    eprintln!("[SILERO] Config: {:?}", cfg);

    let mut model: OnnxModel = load_silero_vad()?;
    eprintln!("[SILERO] Model loaded — supported rates: {:?}", model.sample_rates());

    let mut iox = Config::default();
    iox.global.set_root_path(&Path::new(b"/tmp/iceoryx2/").unwrap());
    let node = NodeBuilder::new().config(&iox).create::<ipc::Service>()?;

    let raw_svc = node.service_builder(&"stt_raw".try_into()?)
        .publish_subscribe::<[u8]>().subscriber_max_buffer_size(128).open_or_create()?;
    let sub = raw_svc.subscriber_builder().create()?;

    let audio_svc = node.service_builder(&"stt_audio".try_into()?)
        .publish_subscribe::<[u8]>().open_or_create()?;
    let pub_ = audio_svc.publisher_builder().initial_max_slice_len(4 * 1024 * 1024).create()?;

    eprintln!("[SILERO] Bus: sub='stt_raw' pub='stt_audio' — READY");

    let mut state = State::Silence;
    let mut silence_count: usize = 0;
    let mut utterance: Vec<f32> = Vec::with_capacity(cfg.max_samples());
    let mut incoming: Vec<f32> = Vec::new();

    loop {
        while let Some(sample) = sub.receive()? {
            let p = sample.payload();
            incoming.extend(p.chunks(4).map(|c| f32::from_le_bytes([c[0],c[1],c[2],c[3]])));
        }

        while incoming.len() >= CHUNK_SIZE {
            let chunk: Vec<f32> = incoming.drain(..CHUNK_SIZE).collect();
            let output = model.forward_chunk(&chunk, SAMPLE_RATE)?;
            let prob = output[[0, 0]];
            let speech = prob > cfg.threshold;

            match state {
                State::Silence => if speech {
                    state = State::Speech;
                    utterance.clear();
                    utterance.extend_from_slice(&chunk);
                    eprintln!("[SILERO] Speech (p={:.2})", prob);
                },
                State::Speech => {
                    utterance.extend_from_slice(&chunk);
                    if !speech { silence_count = 1; state = State::Trailing; }
                    if utterance.len() >= cfg.max_samples() {
                        publish(&mut utterance, &pub_)?;
                        state = State::Silence;
                    }
                },
                State::Trailing => {
                    utterance.extend_from_slice(&chunk);
                    if speech { silence_count = 0; state = State::Speech; }
                    else {
                        silence_count += 1;
                        if silence_count >= cfg.silence_frames_to_end {
                            if utterance.len() >= cfg.min_samples() {
                                publish(&mut utterance, &pub_)?;
                            } else { eprintln!("[SILERO] Too short — skip"); }
                            state = State::Silence; silence_count = 0; utterance.clear();
                        }
                    }
                },
            }
        }
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
}

fn publish(utt: &mut Vec<f32>, pub_: &iceoryx2::port::publisher::Publisher<ipc::Service, [u8], ()>)
    -> Result<(), Box<dyn std::error::Error>> {
    normalize(utt);
    // Decimate 48kHz → 16kHz for STT (Parakeet expects 16kHz)
    let resampled: Vec<f32> = utt.iter().step_by(DECIMATE).copied().collect();
    let dur = resampled.len() as f64 / STT_RATE as f64;
    eprintln!("[SILERO] Publish {:.1}s ({} samples @ {}Hz)", dur, resampled.len(), STT_RATE);
    let bytes: Vec<u8> = resampled.iter().flat_map(|s| s.to_le_bytes()).collect();
    let s = pub_.loan_slice_uninit(bytes.len())?;
    s.write_from_slice(&bytes).send()?;
    utt.clear();
    Ok(())
}
