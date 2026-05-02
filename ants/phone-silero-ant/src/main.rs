//! Phone Silero Ant — VAD for phone audio path
//!
//! Subscribes to phone_stt (16kHz f32 from digi-ant)
//! Publishes complete utterances to stt_audio (for STT ant)
//! Native 16kHz — no resampling, no decimation. Clean signal.
//!
//! Data flow: digi-ant → [phone_stt] → Phone Silero → [stt_audio] → STT

use iceoryx2::prelude::*;
use iceoryx2_bb_system_types::path::Path;
use iceoryx2_bb_container::semantic_string::SemanticString;
use silero_vad_rust::silero_vad::model::{load_silero_vad, OnnxModel};
use serde::Deserialize;

const SAMPLE_RATE: u32 = 16000;
const CHUNK_SIZE: usize = 512; // Silero v6 native chunk at 16kHz
const CONFIG_PATH: &str = "/Users/rocketman/crystalballmini/hypAiAssist/config/phone-silero-ant.json";

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
                eprintln!("[PHONE-SILERO] Config parse error: {} — defaults", e);
                Self::default()
            }),
            Err(_) => { eprintln!("[PHONE-SILERO] No config file — defaults"); Self::default() }
        }
    }
    fn min_samples(&self) -> usize { (self.min_utterance_ms as usize * SAMPLE_RATE as usize) / 1000 }
    fn max_samples(&self) -> usize { (self.max_utterance_ms as usize * SAMPLE_RATE as usize) / 1000 }
}

#[derive(PartialEq)]
enum State { Silence, Speech, Trailing }

// NOTE: No normalize here. VAD is transparent — same samples in, same samples out.
// Gain staging belongs in digi-ant or a dedicated gain ant, not in VAD.

fn main() -> Result<(), Box<dyn std::error::Error>> {
    eprintln!("[PHONE-SILERO] Starting — 16kHz native phone VAD...");
    let cfg = SileroConfig::load();
    eprintln!("[PHONE-SILERO] Config: {:?}", cfg);

    let mut model: OnnxModel = load_silero_vad()?;
    eprintln!("[PHONE-SILERO] Model loaded — supported rates: {:?}", model.sample_rates());

    let mut iox = Config::default();
    iox.global.set_root_path(&Path::new(b"/tmp/iceoryx2/").unwrap());
    let node = NodeBuilder::new().config(&iox).create::<ipc::Service>()?;

    // Subscribe to phone_stt (16kHz f32 typed bus from digi-ant)
    let raw_svc = node.service_builder(&"phone_stt".try_into()?)
        .publish_subscribe::<[f32]>().open_or_create()?;
    let sub = raw_svc.subscriber_builder().create()?;

    // Publish to stt_audio (16kHz f32 for STT/Parakeet)
    let audio_svc = node.service_builder(&"stt_audio".try_into()?)
        .publish_subscribe::<[u8]>().open_or_create()?;
    let pub_ = audio_svc.publisher_builder().initial_max_slice_len(4 * 1024 * 1024).create()?;

    eprintln!("[PHONE-SILERO] Bus: sub='phone_stt'[f32] pub='stt_audio'[u8] — READY");

    let mut state = State::Silence;
    let mut silence_count: usize = 0;
    let mut utterance: Vec<f32> = Vec::with_capacity(cfg.max_samples());
    let mut incoming: Vec<f32> = Vec::new();

    // Stream state: track data flow for cleanup only (not for utterance boundaries)
    // Utterance boundaries are handled by the VAD state machine (silence_frames_to_end)
    // Stream cleanup happens when the call/stream truly ends — not latency-sensitive
    const STREAM_CLEANUP_TIMEOUT: std::time::Duration = std::time::Duration::from_millis(2000);
    let mut last_data_at = std::time::Instant::now();
    let mut stream_active = false;

    loop {
        let mut received_this_cycle = false;
        while let Some(sample) = sub.receive()? {
            let p = sample.payload();
            incoming.extend_from_slice(p);
            last_data_at = std::time::Instant::now();
            stream_active = true;
            received_this_cycle = true;
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
                    eprintln!("[PHONE-SILERO] Speech (p={:.2})", prob);
                },
                State::Speech => {
                    utterance.extend_from_slice(&chunk);
                    if !speech { silence_count = 1; state = State::Trailing; }
                    if utterance.len() >= cfg.max_samples() {
                        publish(&mut utterance, &pub_)?;
                        model.reset_states();
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
                            } else { eprintln!("[PHONE-SILERO] Too short — skip"); }
                            model.reset_states();
                            state = State::Silence; silence_count = 0; utterance.clear();
                        }
                    }
                },
            }
        }

        // Data-driven EOS: only when stream was active and data stopped flowing
        // This is NOT an utterance boundary — the VAD state machine handles those
        // This is stream cleanup: publish any remaining active utterance when the
        // upstream (digi-ant) has gone completely silent for 2 seconds
        if stream_active && !received_this_cycle && last_data_at.elapsed() > STREAM_CLEANUP_TIMEOUT {
            if state != State::Silence {
                if !incoming.is_empty() {
                    utterance.extend(incoming.drain(..));
                }
                if utterance.len() >= cfg.min_samples() {
                    eprintln!("[PHONE-SILERO] Stream ended — publishing remaining utterance");
                    publish(&mut utterance, &pub_)?;
                } else if !utterance.is_empty() {
                    eprintln!("[PHONE-SILERO] Stream ended — utterance too short ({} samples), skip", utterance.len());
                }
                state = State::Silence;
                silence_count = 0;
                utterance.clear();
            }
            if !incoming.is_empty() { incoming.clear(); }
            model.reset_states();
            stream_active = false;
        }

        std::thread::sleep(std::time::Duration::from_millis(10));
    }
}

fn publish(utt: &mut Vec<f32>, pub_: &iceoryx2::port::publisher::Publisher<ipc::Service, [u8], ()>)
    -> Result<(), Box<dyn std::error::Error>> {
    // No normalization — VAD is transparent, passes audio unmodified
    let dur = utt.len() as f64 / SAMPLE_RATE as f64;
    eprintln!("[PHONE-SILERO] Publish {:.1}s ({} samples @ {}Hz)", dur, utt.len(), SAMPLE_RATE);
    let bytes: Vec<u8> = utt.iter().flat_map(|s| s.to_le_bytes()).collect();
    let s = pub_.loan_slice_uninit(bytes.len())?;
    s.write_from_slice(&bytes).send()?;
    utt.clear();
    Ok(())
}
