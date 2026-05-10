//! Bridge Ant — Audio bridge between Twilio and browser AI
//!
//! Subscribes to phone_stt (f32 16kHz from digi-ant) → plays to BlackHole 2ch (Chrome mic)
//! Captures BlackHole 16ch (Chrome audio output) → publishes to tts_audio (digi-ant converts back)
//!
//! Uses digi-ant for mu-law conversion. Bridge-ant handles PCM routing only.
//! Flow: phone_in → digi-ant → [phone_stt] → bridge-ant → BlackHole → Chrome/Airy
//!       Chrome/Airy → BlackHole → bridge-ant → [tts_audio] → digi-ant → [phone_out]

use iceoryx2::prelude::*;
use iceoryx2_bb_system_types::path::Path;
use iceoryx2_bb_container::semantic_string::SemanticString;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use std::sync::{Arc, Mutex};
use std::collections::VecDeque;

fn find_device(host: &cpal::Host, name: &str, input: bool) -> Option<cpal::Device> {
    let devices = if input { host.input_devices().ok()? } else { host.output_devices().ok()? };
    for dev in devices {
        if let Ok(n) = dev.name() {
            if n.to_lowercase().contains(&name.to_lowercase()) {
                return Some(dev);
            }
        }
    }
    None
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    eprintln!("[BRIDGE] Starting Audio Bridge Ant...");

    let host = cpal::default_host();

    // Output: BlackHole 2ch (Chrome uses this as mic input)
    let bh2_out = find_device(&host, "BlackHole 2ch", false)
        .expect("BlackHole 2ch output not found");
    eprintln!("[BRIDGE] To Chrome mic → {}", bh2_out.name()?);

    // Input: BlackHole 16ch (Chrome plays audio here)
    let bh16_in = find_device(&host, "BlackHole 16ch", true)
        .expect("BlackHole 16ch input not found");
    eprintln!("[BRIDGE] From Chrome audio ← {}", bh16_in.name()?);

    // Buffer: phone_stt f32 samples → BlackHole 2ch output
    let to_chrome: Arc<Mutex<VecDeque<f32>>> = Arc::new(Mutex::new(VecDeque::with_capacity(32000)));

    // Buffer: BlackHole 16ch input → tts_audio f32 samples
    let from_chrome: Arc<Mutex<Vec<f32>>> = Arc::new(Mutex::new(Vec::with_capacity(32000)));

    // Output stream: plays f32 PCM to BlackHole 2ch at 16kHz (matches phone_stt rate)
    let to_chrome_play = to_chrome.clone();
    let out_config = cpal::StreamConfig {
        channels: 1,
        sample_rate: cpal::SampleRate(16000),
        buffer_size: cpal::BufferSize::Default,
    };
    let out_stream = bh2_out.build_output_stream(
        &out_config,
        move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
            let mut buf = to_chrome_play.lock().unwrap();
            for sample in data.iter_mut() {
                *sample = buf.pop_front().unwrap_or(0.0);
            }
        },
        |err| eprintln!("[BRIDGE] Output error: {}", err),
        None,
    )?;
    out_stream.play()?;
    eprintln!("[BRIDGE] BlackHole 2ch output stream: 16kHz mono");

    // Input stream: captures f32 PCM from BlackHole 16ch
    let from_chrome_cap = from_chrome.clone();
    let in_config = bh16_in.default_input_config()?;
    let in_channels = in_config.channels() as usize;
    let in_rate = in_config.sample_rate().0;
    eprintln!("[BRIDGE] BlackHole 16ch capture: {}Hz {}ch", in_rate, in_channels);

    // We need to resample from BlackHole's rate to 24kHz for tts_audio (what digi-ant expects)
    let target_rate = 24000u32;
    let in_stream = bh16_in.build_input_stream(
        &in_config.into(),
        move |data: &[f32], _: &cpal::InputCallbackInfo| {
            let mut buf = from_chrome_cap.lock().unwrap();
            // Mix to mono, simple decimation to approximate target rate
            let step = std::cmp::max(1, (in_rate / target_rate) as usize);
            for frame in data.chunks(in_channels * step) {
                let mono: f32 = frame.iter().take(in_channels).sum::<f32>() / in_channels as f32;
                buf.push(mono);
            }
            // Prevent unbounded growth
            let len = buf.len();
            if len > 48000 {
                buf.drain(..len - 48000);
            }
        },
        |err| eprintln!("[BRIDGE] Input error: {}", err),
        None,
    )?;
    in_stream.play()?;

    // iceoryx2 bus
    let mut iox = Config::default();
    iox.global.set_root_path(&Path::new(b"/tmp/iceoryx2/").unwrap());
    let node = NodeBuilder::new().config(&iox).create::<ipc::Service>()?;

    // Subscribe to phone_stt (f32 16kHz from digi-ant)
    let stt_svc = node.service_builder(&"phone_stt".try_into()?)
        .publish_subscribe::<[f32]>()
        .open_or_create()?;
    let stt_sub = stt_svc.subscriber_builder().create()?;

    // Publish to tts_audio (f32 for digi-ant to convert back to mu-law)
    let audio_svc = node.service_builder(&"tts_audio".try_into()?)
        .publish_subscribe::<[u8]>()
        .open_or_create()?;
    let audio_pub = audio_svc.publisher_builder()
        .initial_max_slice_len(4 * 1024 * 1024)
        .create()?;

    eprintln!("[BRIDGE] Bus: sub='phone_stt'[f32] pub='tts_audio'[u8] — READY");
    eprintln!("[BRIDGE] Voice bridge active: phone → digi → bridge → Chrome/Airy → bridge → digi → phone");

    loop {
        // phone_stt (f32 from digi-ant) → BlackHole 2ch → Chrome mic
        while let Some(sample) = stt_sub.receive()? {
            let pcm = sample.payload();
            if pcm.is_empty() { continue; }
            let mut buf = to_chrome.lock().unwrap();
            buf.extend(pcm.iter());
        }

        // BlackHole 16ch (Chrome audio) → tts_audio (f32 bytes for digi-ant)
        {
            let mut buf = from_chrome.lock().unwrap();
            if buf.len() >= 4800 {  // 200ms at 24kHz
                let samples: Vec<f32> = buf.drain(..4800).collect();
                // Check if there's actual audio (not silence)
                let peak = samples.iter().map(|s| s.abs()).fold(0.0f32, f32::max);
                if peak > 0.001 {
                    let bytes: Vec<u8> = samples.iter().flat_map(|s| s.to_le_bytes()).collect();
                    if let Ok(loan) = audio_pub.loan_slice_uninit(bytes.len()) {
                        let _ = loan.write_from_slice(&bytes).send();
                    }
                }
            }
        }

        std::thread::sleep(std::time::Duration::from_millis(10));
    }
}
