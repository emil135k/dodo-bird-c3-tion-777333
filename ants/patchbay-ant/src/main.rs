//! Patchbay Ant — High-Fidelity Audio Routing
//!
//! Replaces Ear + Mouth with centralized device management.
//! Native sample-rate negotiation avoids the 8kHz telephony trap.

use iceoryx2::prelude::*;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use rodio::{OutputStream, Sink, buffer::SamplesBuffer};
use serde::Deserialize;
use std::sync::{Arc, Mutex};

const PLAYBACK_SAMPLE_RATE: u32 = 24000;
const CONFIG_PATH: &str = "/Users/rocketman/crystalballmini/hypAiAssist/config/patchbay-ant.json";

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
    fn default() -> Self {
        Self { input_device: d_input(), output_device: d_output() }
    }
}

impl PatchbayConfig {
    fn load() -> Self {
        match std::fs::read_to_string(CONFIG_PATH) {
            Ok(text) => serde_json::from_str(&text).unwrap_or_else(|e| {
                eprintln!("[PATCHBAY] Config error: {} — using defaults", e);
                Self::default()
            }),
            Err(_) => { 
                eprintln!("[PATCHBAY] Config not found at {} — using defaults", CONFIG_PATH); 
                Self::default() 
            }
        }
    }
}

fn find_input_device(host: &cpal::Host, name: &str) -> Option<cpal::Device> {
    let devices = host.input_devices().ok()?;
    for device in devices {
        if let Ok(n) = device.name() {
            if n.to_lowercase().contains(&name.to_lowercase()) {
                return Some(device);
            }
        }
    }
    None
}

fn find_output_device(host: &cpal::Host, name: &str) -> Option<cpal::Device> {
    let devices = host.output_devices().ok()?;
    eprintln!("[PATCHBAY] Available Output Devices:");
    for device in devices {
        if let Ok(n) = device.name() {
            eprintln!("  - {}", n); // This helps us debug the exact name
            if n.to_lowercase().contains(&name.to_lowercase()) {
                return Some(device);
            }
        }
    }
    None
}


fn main() -> Result<(), Box<dyn std::error::Error>> {
    eprintln!("[PATCHBAY] Initializing Sovereign Audio Pipeline...");
    let cfg = PatchbayConfig::load();

    let host = cpal::default_host();

    // --- INPUT SETUP (Ear) ---
    let input_dev = find_input_device(&host, &cfg.input_device)
        .expect(&format!("Input device '{}' not found", cfg.input_device));
    
    let input_config = input_dev.default_input_config()?;
    let device_rate = input_config.sample_rate().0;
    let channels = input_config.channels() as usize;

    eprintln!("[PATCHBAY] Mic: {} ({}Hz, {}ch) — native on bus", input_dev.name()?, device_rate, channels);

    // --- OUTPUT SETUP (Mouth) ---
    // Native Handshake: Demand high-fidelity 24kHz+ config to avoid 8kHz clipping
    let output_dev = find_output_device(&host, &cfg.output_device)
        .expect(&format!("Output device '{}' not found", cfg.output_device));

    let output_config = output_dev.supported_output_configs()?
        .filter(|c| c.max_sample_rate().0 >= PLAYBACK_SAMPLE_RATE)
        .next()
        .expect("Device does not support high-fidelity playback")
        .with_sample_rate(cpal::SampleRate(PLAYBACK_SAMPLE_RATE));

    eprintln!("[PATCHBAY] Spk: {} ({}Hz)", output_dev.name()?, output_config.sample_rate().0);

    let (_stream, stream_handle) = OutputStream::try_from_device_config(&output_dev, output_config)?;
    let sink = Arc::new(Mutex::new(Sink::try_new(&stream_handle)?));

    // --- ICEORYX2 BUS SETUP ---
    let node = NodeBuilder::new().create::<ipc::Service>()?;

    // Contract: stt_raw contains f32 PCM at 48kHz mono.
    // Patchbay captures at the device's native rate and publishes as-is.
    // Downstream ants (silero-ant) expect 48kHz — if the device rate differs,
    // patchbay must resample before publishing. Current Blackwire 3210 = 48kHz.
    eprintln!("[PATCHBAY] Device rate: {}Hz — stt_raw contract: 48kHz", device_rate);
    if device_rate != 48000 {
        eprintln!("[PATCHBAY] WARNING: device rate {}Hz != 48kHz — downstream may malfunction", device_rate);
    }
    let raw_svc = node.service_builder(&"stt_raw".try_into()?)
        .publish_subscribe::<[u8]>()
        .open_or_create()?;
    let mic_pub = raw_svc.publisher_builder()
        .initial_max_slice_len(4 * 1024 * 1024)
        .create()?;

    // Contract: tts_audio contains f32 PCM at 24kHz mono from tts-ant
    let audio_svc = node.service_builder(&"tts_audio".try_into()?)
        .publish_subscribe::<[u8]>()
        .open_or_create()?;
    let spk_sub = audio_svc.subscriber_builder().create()?;

    eprintln!("[PATCHBAY] Sovereign Bus: stt_raw <-> tts_audio — READY");

    // Capture Loop Logic
    let buffer: Arc<Mutex<Vec<f32>>> = Arc::new(Mutex::new(Vec::with_capacity(32000)));
    let buf_clone = buffer.clone();

    let stream = input_dev.build_input_stream(
        &input_config.into(),
        move |data: &[f32], _: &cpal::InputCallbackInfo| {
            let mut buf = buf_clone.lock().unwrap();
            for frame in data.chunks(channels) {
                let sample: f32 = frame.iter().sum::<f32>() / channels as f32;
                buf.push(sample);
            }
        },
        |err| eprintln!("[PATCHBAY] Input Error: {}", err),
        None,
    )?;
    stream.play()?;

    // --- MAIN EXECUTION LOOP ---
    loop {
        // 1. Drain mic buffer and publish
        {
            let mut buf = buffer.lock().unwrap();
            if buf.len() >= 1600 {
                let samples: Vec<f32> = buf.drain(..).collect();
                drop(buf);
                let bytes: Vec<u8> = samples.iter().flat_map(|s| s.to_le_bytes()).collect();
                if let Ok(loan) = mic_pub.loan_slice_uninit(bytes.len()) {
                    let _ = loan.write_from_slice(&bytes).send();
                }
            }
        }

        // 2. Play incoming Jarvina audio
        while let Some(sample) = spk_sub.receive()? {
            let raw = sample.payload();
            if raw.len() % 4 != 0 {
                eprintln!("[PATCHBAY] Contract violation: tts_audio {} bytes not divisible by 4 — skipping", raw.len());
                continue;
            }
            let samples: Vec<f32> = raw.chunks_exact(4)
                .map(|c| f32::from_le_bytes([c[0], c[1], c[2], c[3]]))
                .collect();
            if !samples.is_empty() {
                let s = sink.lock().unwrap();
                s.append(SamplesBuffer::new(1, PLAYBACK_SAMPLE_RATE, samples));
            }
        }

        std::thread::sleep(std::time::Duration::from_millis(10));
    }
}

