//! Patchbay Ant — Audio Routing with SpeexDSP AEC
//!
//! Simple loop: get mic, get speaker ref, cancel echo, publish clean.
//! No scattered callbacks. No multi-step resampling. Matched signals.

use iceoryx2::prelude::*;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use aec_rs::{Aec, AecConfig};
use serde::Deserialize;
use std::sync::{Arc, Mutex};
use std::collections::VecDeque;

const CONFIG_PATH: &str = "/Users/rocketman/crystalballmini/hypAiAssist/config/patchbay-ant.json";

// AEC parameters — from the guide, optimized for laptop speakers
const SAMPLE_RATE: u32 = 16000;
const FRAME_SIZE: usize = 320;     // 20ms at 16kHz
const FILTER_LEN: i32 = 1024;      // ~64ms delay capacity

#[derive(Deserialize, Debug)]
struct PatchbayConfig {
    #[serde(default = "d_input")]
    input_device: String,
    #[serde(default = "d_output")]
    output_device: String,
}
fn d_input() -> String { "Plantronics Blackwire 3210 Series".into() }
fn d_output() -> String { "Plantronics Blackwire 3210 Series".into() }
impl Default for PatchbayConfig {
    fn default() -> Self { Self { input_device: d_input(), output_device: d_output() } }
}
impl PatchbayConfig {
    fn load() -> Self {
        match std::fs::read_to_string(CONFIG_PATH) {
            Ok(text) => serde_json::from_str(&text).unwrap_or_else(|e| {
                eprintln!("[PATCHBAY] Config error: {} — defaults", e); Self::default()
            }),
            Err(_) => { eprintln!("[PATCHBAY] No config — defaults"); Self::default() }
        }
    }
}

fn find_device(host: &cpal::Host, name: &str, input: bool) -> Option<cpal::Device> {
    let devices = if input { host.input_devices().ok()? } else { host.output_devices().ok()? };
    for device in devices {
        if let Ok(n) = device.name() {
            if n.to_lowercase().contains(&name.to_lowercase()) {
                return Some(device);
            }
        }
    }
    None
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    eprintln!("[PATCHBAY] Starting — SpeexDSP AEC, {}Hz, frame={}, filter={}",
        SAMPLE_RATE, FRAME_SIZE, FILTER_LEN);
    let cfg = PatchbayConfig::load();
    let host = cpal::default_host();

    // --- DEVICES ---
    let input_dev = find_device(&host, &cfg.input_device, true)
        .expect(&format!("Input '{}' not found", cfg.input_device));
    let output_dev = find_device(&host, &cfg.output_device, false)
        .expect(&format!("Output '{}' not found", cfg.output_device));

    let in_config = input_dev.default_input_config()?;
    let in_rate = in_config.sample_rate().0;
    let in_channels = in_config.channels() as usize;
    eprintln!("[PATCHBAY] Mic: {} ({}Hz, {}ch)", input_dev.name()?, in_rate, in_channels);

    let out_supported = output_dev.supported_output_configs()?
        .next().expect("No output config");
    let out_rate = out_supported.with_max_sample_rate().sample_rate().0;
    let out_channels = out_supported.channels() as usize;
    eprintln!("[PATCHBAY] Spk: {} ({}Hz, {}ch)", output_dev.name()?, out_rate, out_channels);

    // --- SHARED BUFFERS ---
    // Mic capture at native rate → main loop downsamples to 16kHz
    let mic_buf: Arc<Mutex<Vec<f32>>> = Arc::new(Mutex::new(Vec::with_capacity(16000)));
    // Speaker playback queue at native rate → output callback drains
    let spk_queue: Arc<Mutex<VecDeque<f32>>> = Arc::new(Mutex::new(VecDeque::new()));
    // Speaker reference at 16kHz — EXACT samples that went to speaker, downsampled
    let spk_ref: Arc<Mutex<VecDeque<i16>>> = Arc::new(Mutex::new(VecDeque::new()));

    // --- INPUT STREAM ---
    let mic_clone = mic_buf.clone();
    let input_stream = input_dev.build_input_stream(
        &in_config.into(),
        move |data: &[f32], _: &cpal::InputCallbackInfo| {
            let mut buf = mic_clone.lock().unwrap();
            for frame in data.chunks(in_channels) {
                buf.push(frame.iter().sum::<f32>() / in_channels as f32);
            }
        },
        |err| eprintln!("[PATCHBAY] Mic error: {}", err),
        None,
    )?;
    input_stream.play()?;

    // --- OUTPUT STREAM ---
    // Callback plays AND captures the exact reference at output rate
    let spk_play = spk_queue.clone();
    let spk_ref_out = spk_ref.clone();
    let out_config = cpal::StreamConfig {
        channels: out_channels as u16,
        sample_rate: cpal::SampleRate(out_rate),
        buffer_size: cpal::BufferSize::Default,
    };
    let out_rate_f = out_rate as f64;

    let output_stream = output_dev.build_output_stream(
        &out_config,
        move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
            let mut pq = spk_play.lock().unwrap();
            let mut played: Vec<f32> = Vec::new();
            for frame in data.chunks_mut(out_channels) {
                let sample = pq.pop_front().unwrap_or(0.0);
                for ch in frame.iter_mut() { *ch = sample; }
                played.push(sample);
            }
            // Downsample played samples to 16kHz for AEC reference
            if played.iter().any(|&s| s.abs() > 0.0001) {
                let ratio = out_rate_f / SAMPLE_RATE as f64;
                let out_len = (played.len() as f64 / ratio) as usize;
                let mut ref_16k: Vec<i16> = Vec::with_capacity(out_len);
                for i in 0..out_len {
                    let src = i as f64 * ratio;
                    let idx = src as usize;
                    let frac = (src - idx as f64) as f32;
                    let s0 = played[idx.min(played.len() - 1)];
                    let s1 = played[(idx + 1).min(played.len() - 1)];
                    ref_16k.push(((s0 + (s1 - s0) * frac) * 32767.0).clamp(-32768.0, 32767.0) as i16);
                }
                if let Ok(mut sr) = spk_ref_out.lock() {
                    sr.extend(ref_16k.iter());
                    let len = sr.len();
                    if len > 32000 { sr.drain(..len - 32000); }
                }
            }
        },
        |err| eprintln!("[PATCHBAY] Spk error: {}", err),
        None,
    )?;
    output_stream.play()?;

    // --- ICEORYX2 ---
    let node = NodeBuilder::new().create::<ipc::Service>()?;
    let raw_svc = node.service_builder(&"stt_raw".try_into()?)
        .publish_subscribe::<[u8]>().open_or_create()?;
    let mic_pub = raw_svc.publisher_builder()
        .initial_max_slice_len(4 * 1024 * 1024).create()?;
    let audio_svc = node.service_builder(&"tts_audio".try_into()?)
        .publish_subscribe::<[u8]>().open_or_create()?;
    let spk_sub = audio_svc.subscriber_builder().create()?;

    // --- AEC ---
    let aec = Aec::new(&AecConfig {
        frame_size: FRAME_SIZE,
        filter_length: FILTER_LEN,
        sample_rate: SAMPLE_RATE,
        enable_preprocess: true,
    });

    eprintln!("[PATCHBAY] AEC ready. Bus: stt_raw <-> tts_audio");

    // --- MAIN LOOP (matches the guide exactly) ---
    loop {
        // 1. Receive TTS audio, resample to output rate, queue for playback
        while let Some(sample) = spk_sub.receive()? {
            let raw = sample.payload();
            if raw.len() % 4 != 0 { continue; }
            let samples: Vec<f32> = raw.chunks_exact(4)
                .map(|c| f32::from_le_bytes([c[0], c[1], c[2], c[3]]))
                .collect();
            if samples.is_empty() { continue; }

            // Resample 24kHz → output rate
            let ratio = out_rate as f64 / 24000.0;
            let out_len = (samples.len() as f64 * ratio) as usize;
            let mut resampled = Vec::with_capacity(out_len);
            for i in 0..out_len {
                let src = i as f64 / ratio;
                let idx = src as usize;
                let frac = (src - idx as f64) as f32;
                let s0 = samples[idx.min(samples.len() - 1)];
                let s1 = samples[(idx + 1).min(samples.len() - 1)];
                resampled.push(s0 + (s1 - s0) * frac);
            }
            let mut pq = spk_queue.lock().unwrap();
            pq.extend(resampled.iter());
        }

        // 2. Get mic data — downsample to 16kHz
        let mic_frame: Vec<i16> = {
            let mut buf = mic_buf.lock().unwrap();
            let needed = (FRAME_SIZE as f64 * in_rate as f64 / SAMPLE_RATE as f64) as usize;
            if buf.len() < needed {
                std::thread::sleep(std::time::Duration::from_millis(5));
                continue;
            }
            let raw: Vec<f32> = buf.drain(..needed).collect();
            // Downsample in_rate → 16kHz via linear interpolation
            let ratio = in_rate as f64 / SAMPLE_RATE as f64;
            let mut out = Vec::with_capacity(FRAME_SIZE);
            for i in 0..FRAME_SIZE {
                let src = i as f64 * ratio;
                let idx = src as usize;
                let frac = (src - idx as f64) as f32;
                let s0 = raw[idx.min(raw.len() - 1)];
                let s1 = raw[(idx + 1).min(raw.len() - 1)];
                out.push(((s0 + (s1 - s0) * frac) * 32767.0).clamp(-32768.0, 32767.0) as i16);
            }
            out
        };

        // 3. Get speaker reference — EXACT what was played, already at 16kHz
        let speaker_frame: Vec<i16> = {
            let mut sr = spk_ref.lock().unwrap();
            if sr.len() >= FRAME_SIZE {
                sr.drain(..FRAME_SIZE).collect()
            } else {
                vec![0i16; FRAME_SIZE]
            }
        };

        // 4. Process — one call, matched signals
        let mut clean_frame = vec![0i16; FRAME_SIZE];
        aec.cancel_echo(&mic_frame, &speaker_frame, &mut clean_frame);

        // 5. Publish clean_frame to stt_raw (upsample 16k → 48k for silero)
        let mut out_48k: Vec<f32> = Vec::with_capacity(FRAME_SIZE * 3);
        for i in 0..clean_frame.len() {
            let s0 = clean_frame[i] as f32 / 32768.0;
            let s1 = if i + 1 < clean_frame.len() {
                clean_frame[i + 1] as f32 / 32768.0
            } else { s0 };
            out_48k.push(s0);
            out_48k.push(s0 + (s1 - s0) / 3.0);
            out_48k.push(s0 + (s1 - s0) * 2.0 / 3.0);
        }
        let bytes: Vec<u8> = out_48k.iter().flat_map(|s| s.to_le_bytes()).collect();
        if let Ok(loan) = mic_pub.loan_slice_uninit(bytes.len()) {
            let _ = loan.write_from_slice(&bytes).send();
        }
    }
}
