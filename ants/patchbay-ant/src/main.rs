//! Patchbay Ant — Audio Routing with SpeexDSP Echo Cancellation
//!
//! Uses aec-rs (SpeexDSP) for adaptive echo cancellation.
//! cancel_echo(mic, speaker, output) — SpeexDSP handles delay estimation internally.
//! No hand-tuned delay values. The filter adapts.

use iceoryx2::prelude::*;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use rodio::{OutputStream, Sink, buffer::SamplesBuffer};
use aec_rs::{Aec, AecConfig};
use serde::Deserialize;
use std::sync::{Arc, Mutex};
use std::collections::VecDeque;

const PLAYBACK_SAMPLE_RATE: u32 = 24000;
const CONFIG_PATH: &str = "/Users/rocketman/crystalballmini/hypAiAssist/config/patchbay-ant.json";

// AEC operates at 16kHz i16 — we downsample from 48kHz f32 for AEC, then publish original
const AEC_RATE: u32 = 16000;
const AEC_FRAME_SIZE: usize = 160; // 10ms at 16kHz
const AEC_FILTER_LENGTH: i32 = 3200; // 200ms tail — handles room reverb

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
    if !input { eprintln!("[PATCHBAY] Available Output Devices:"); }
    for device in devices {
        if let Ok(n) = device.name() {
            if !input { eprintln!("  - {}", n); }
            if n.to_lowercase().contains(&name.to_lowercase()) {
                return Some(device);
            }
        }
    }
    None
}

/// Downsample f32 48kHz → i16 16kHz (factor 3, simple decimation)
fn downsample_to_aec(samples: &[f32]) -> Vec<i16> {
    samples.iter().step_by(3)
        .map(|&s| (s * 32767.0).clamp(-32768.0, 32767.0) as i16)
        .collect()
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    eprintln!("[PATCHBAY] Initializing with SpeexDSP Echo Cancellation...");
    let cfg = PatchbayConfig::load();
    let host = cpal::default_host();

    // --- INPUT ---
    let input_dev = find_device(&host, &cfg.input_device, true)
        .expect(&format!("Input '{}' not found", cfg.input_device));
    let input_config = input_dev.default_input_config()?;
    let device_rate = input_config.sample_rate().0;
    let channels = input_config.channels() as usize;
    eprintln!("[PATCHBAY] Mic: {} ({}Hz, {}ch)", input_dev.name()?, device_rate, channels);
    if input_config.sample_format() != cpal::SampleFormat::F32 {
        eprintln!("[PATCHBAY] FATAL: not F32"); std::process::exit(1);
    }
    if device_rate != 48000 {
        eprintln!("[PATCHBAY] FATAL: {}Hz != 48kHz", device_rate); std::process::exit(1);
    }

    // --- OUTPUT ---
    let output_dev = find_device(&host, &cfg.output_device, false)
        .expect(&format!("Output '{}' not found", cfg.output_device));
    let output_config = output_dev.supported_output_configs()?
        .filter(|c| c.max_sample_rate().0 >= PLAYBACK_SAMPLE_RATE)
        .next().expect("No suitable output config")
        .with_max_sample_rate();
    eprintln!("[PATCHBAY] Spk: {} ({}Hz)", output_dev.name()?, output_config.sample_rate().0);
    let (_stream, stream_handle) = OutputStream::try_from_device_config(&output_dev, output_config)?;
    let sink = Arc::new(Mutex::new(Sink::try_new(&stream_handle)?));

    // --- ICEORYX2 ---
    let node = NodeBuilder::new().create::<ipc::Service>()?;
    let raw_svc = node.service_builder(&"stt_raw".try_into()?)
        .publish_subscribe::<[u8]>().open_or_create()?;
    let mic_pub = raw_svc.publisher_builder()
        .initial_max_slice_len(4 * 1024 * 1024).create()?;
    let audio_svc = node.service_builder(&"tts_audio".try_into()?)
        .publish_subscribe::<[u8]>().open_or_create()?;
    let spk_sub = audio_svc.subscriber_builder().create()?;

    // --- SpeexDSP AEC ---
    let aec = Aec::new(&AecConfig {
        frame_size: AEC_FRAME_SIZE,
        filter_length: AEC_FILTER_LENGTH,
        sample_rate: AEC_RATE,
        enable_preprocess: true,
    });
    let aec = Arc::new(Mutex::new(aec));

    eprintln!("[PATCHBAY] SpeexDSP AEC: {}Hz, frame={}, filter={}ms",
        AEC_RATE, AEC_FRAME_SIZE, AEC_FILTER_LENGTH as f64 / AEC_RATE as f64 * 1000.0);
    eprintln!("[PATCHBAY] Bus: stt_raw <-> tts_audio — READY");

    // Speaker reference buffer (i16 at 16kHz for AEC)
    let speaker_ref: Arc<Mutex<VecDeque<i16>>> = Arc::new(Mutex::new(VecDeque::new()));

    // Capture buffer from mic
    let capture_buf: Arc<Mutex<Vec<f32>>> = Arc::new(Mutex::new(Vec::with_capacity(32000)));
    let capture_clone = capture_buf.clone();

    let stream = input_dev.build_input_stream(
        &input_config.into(),
        move |data: &[f32], _: &cpal::InputCallbackInfo| {
            let mut buf = capture_clone.lock().unwrap();
            for frame in data.chunks(channels) {
                let sample: f32 = frame.iter().sum::<f32>() / channels as f32;
                buf.push(sample);
            }
        },
        |err| eprintln!("[PATCHBAY] Input Error: {}", err),
        None,
    )?;
    stream.play()?;

    // --- MAIN LOOP ---
    loop {
        // 1. Receive TTS audio, play it, AND store as speaker reference for AEC
        while let Some(sample) = spk_sub.receive()? {
            let raw = sample.payload();
            if raw.len() % 4 != 0 { continue; }
            let samples: Vec<f32> = raw.chunks_exact(4)
                .map(|c| f32::from_le_bytes([c[0], c[1], c[2], c[3]]))
                .collect();
            if samples.is_empty() { continue; }

            // Play through speaker
            let s = sink.lock().unwrap();
            s.append(SamplesBuffer::new(1, PLAYBACK_SAMPLE_RATE, samples.clone()));

            // Downsample 24kHz → 16kHz for AEC reference (factor 1.5, approximate)
            // Simple: take every 1.5th sample via linear interpolation
            let ratio = PLAYBACK_SAMPLE_RATE as f64 / AEC_RATE as f64; // 1.5
            let out_len = (samples.len() as f64 / ratio) as usize;
            let mut ref_16k = Vec::with_capacity(out_len);
            for i in 0..out_len {
                let src = i as f64 * ratio;
                let idx = src as usize;
                let s0 = samples[idx.min(samples.len() - 1)];
                ref_16k.push((s0 * 32767.0).clamp(-32768.0, 32767.0) as i16);
            }

            let mut sr = speaker_ref.lock().unwrap();
            sr.extend(ref_16k.iter());
            // Prevent unbounded growth
            let len = sr.len();
            if len > 32000 { sr.drain(..len - 32000); }
        }

        // 2. Process mic capture through AEC
        {
            let mut buf = capture_buf.lock().unwrap();
            // Need at least AEC_FRAME_SIZE * 3 samples at 48kHz (= AEC_FRAME_SIZE at 16kHz)
            let needed = AEC_FRAME_SIZE * 3;
            if buf.len() >= needed {
                let mut all_clean_48k: Vec<f32> = Vec::new();

                while buf.len() >= needed {
                    // Take a frame worth of 48kHz samples
                    let frame_48k: Vec<f32> = buf.drain(..needed).collect();

                    // Downsample to 16kHz i16 for AEC
                    let mic_16k = downsample_to_aec(&frame_48k);

                    // Get matching speaker reference
                    let echo_16k: Vec<i16> = {
                        let mut sr = speaker_ref.lock().unwrap();
                        if sr.len() >= AEC_FRAME_SIZE {
                            sr.drain(..AEC_FRAME_SIZE).collect()
                        } else {
                            // No speaker reference = silence (no echo to cancel)
                            vec![0i16; AEC_FRAME_SIZE]
                        }
                    };

                    // Run AEC: cancel speaker echo from mic capture
                    let mut clean_16k = vec![0i16; AEC_FRAME_SIZE];
                    if let Ok(aec) = aec.lock() {
                        aec.cancel_echo(&mic_16k, &echo_16k, &mut clean_16k);
                    }

                    // Trust AEC output — convert back to f32 48kHz and publish
                    // AEC does the math. We don't second-guess it.
                    // Upsample clean 16kHz i16 → 48kHz f32 (factor 3)
                    let clean_48k: Vec<f32> = clean_16k.iter()
                        .flat_map(|&s| {
                            let f = s as f32 / 32768.0;
                            [f, f, f] // simple 3x upsample
                        })
                        .collect();
                    all_clean_48k.extend_from_slice(&clean_48k);
                }

                if !all_clean_48k.is_empty() {
                    let bytes: Vec<u8> = all_clean_48k.iter().flat_map(|s| s.to_le_bytes()).collect();
                    if let Ok(loan) = mic_pub.loan_slice_uninit(bytes.len()) {
                        let _ = loan.write_from_slice(&bytes).send();
                    }
                }
            }
        }

        std::thread::sleep(std::time::Duration::from_millis(5));
    }
}
