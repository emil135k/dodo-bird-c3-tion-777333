//! Ear Ant — Microphone capture to iceoryx2 bus
//!
//! Captures audio from the default input device (Mac mic, USB headset, etc.)
//! Publishes raw f32 samples at 16kHz mono to the stt_audio bus.
//! The STT ant subscribes and transcribes.
//!
//! One ant, one job. Zero disk.

use iceoryx2::prelude::*;
use iceoryx2_bb_system_types::path::Path;
use iceoryx2_bb_container::semantic_string::SemanticString;

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use std::sync::{Arc, Mutex};

const TARGET_SAMPLE_RATE: u32 = 16000;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    eprintln!("[EAR] Starting Ear Ant...");

    // Audio input setup
    let host = cpal::default_host();
    let device = host.default_input_device()
        .expect("No input device found");

    eprintln!("[EAR] Input device: {}", device.name().unwrap_or_default());

    let config = device.default_input_config()
        .expect("No default input config");

    eprintln!("[EAR] Input config: {:?}", config);

    let device_sample_rate = config.sample_rate().0;
    let device_channels = config.channels() as usize;

    // iceoryx2 with hardcoded config
    let mut iox_config = Config::default();
    iox_config.global.set_root_path(&Path::new(b"/tmp/iceoryx2/").unwrap());
    let node = NodeBuilder::new().config(&iox_config).create::<ipc::Service>()?;

    let audio_svc = node.service_builder(&"stt_raw".try_into()?)
        .publish_subscribe::<[u8]>()
        .open_or_create()?;

    let publisher = audio_svc.publisher_builder()
        .initial_max_slice_len(4 * 1024 * 1024) // 4MB max
        .create()?;

    eprintln!("[EAR] Bus: pub='stt_raw' — READY");
    eprintln!("[EAR] Device rate: {}Hz, channels: {}, target: {}Hz mono",
        device_sample_rate, device_channels, TARGET_SAMPLE_RATE);

    // Accumulate samples, publish in chunks
    // We collect ~1 second of audio then publish
    let chunk_samples = TARGET_SAMPLE_RATE as usize; // 1 second at 16kHz
    let buffer: Arc<Mutex<Vec<f32>>> = Arc::new(Mutex::new(Vec::with_capacity(chunk_samples * 2)));
    let buffer_clone = buffer.clone();

    // Downsample ratio: keep every Nth sample (48000/16000 = 3)
    let skip = (device_sample_rate / TARGET_SAMPLE_RATE) as usize;

    let stream = device.build_input_stream(
        &config.into(),
        move |data: &[f32], _: &cpal::InputCallbackInfo| {
            let mut buf = buffer_clone.lock().unwrap();

            // Mono mix + downsample by skipping
            for (i, frame) in data.chunks(device_channels).enumerate() {
                if i % skip != 0 { continue; }
                let sample: f32 = frame.iter().sum::<f32>() / device_channels as f32;
                buf.push(sample);
            }
        },
        |err| {
            eprintln!("[EAR] Stream error: {}", err);
        },
        None,
    )?;

    stream.play()?;
    eprintln!("[EAR] Listening...");

    // Main loop — publish accumulated audio chunks
    loop {
        std::thread::sleep(std::time::Duration::from_millis(500));

        let mut buf = buffer.lock().unwrap();
        if buf.len() < 1600 { continue; } // At least 3 seconds at 16kHz

        let samples: Vec<f32> = buf.drain(..).collect();
        drop(buf);

        let duration = samples.len() as f64 / TARGET_SAMPLE_RATE as f64;

        // Convert to bytes for iceoryx2
        let bytes: Vec<u8> = samples.iter()
            .flat_map(|s| s.to_le_bytes())
            .collect();

        match publisher.loan_slice_uninit(bytes.len()) {
            Ok(loan) => {
                let loan = loan.write_from_slice(&bytes);
                loan.send()?;
                eprintln!("[EAR] Published {:.1}s audio ({} samples)", duration, samples.len());
            }
            Err(e) => {
                eprintln!("[EAR] Publish error: {}", e);
            }
        }
    }
}
