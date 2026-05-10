//! Patchbay Ant — Audio Routing with SpeexDSP Echo Cancellation
//!
//! Raw cpal for BOTH input and output — no rodio (Lyra: rodio is a black box buffer).
//! AEC reference is captured at the exact moment hardware plays the sample.
//! SpeexDSP cancel_echo(mic, speaker, output) with matched resample paths.

use iceoryx2::prelude::*;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use aec_rs::{Aec, AecConfig};
use serde::Deserialize;
use std::sync::{Arc, Mutex};
use std::collections::VecDeque;

const CONFIG_PATH: &str = "/Users/rocketman/crystalballmini/hypAiAssist/config/patchbay-ant.json";

const AEC_RATE: u32 = 16000;
const AEC_FRAME_SIZE: usize = 160;     // 10ms at 16kHz
const AEC_FILTER_LENGTH: i32 = 3200;   // 200ms tail

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

/// LPF + downsample f32 48kHz → i16 16kHz (3-tap moving average then step_by(3))
fn downsample_to_aec(samples: &[f32]) -> Vec<i16> {
    let mut filtered = vec![0.0f32; samples.len()];
    for i in 0..samples.len() {
        let s0 = if i > 0 { samples[i - 1] } else { samples[i] };
        let s1 = samples[i];
        let s2 = if i + 1 < samples.len() { samples[i + 1] } else { samples[i] };
        filtered[i] = (s0 + s1 + s2) / 3.0;
    }
    filtered.iter().step_by(3)
        .map(|&s| (s * 32767.0).clamp(-32768.0, 32767.0) as i16)
        .collect()
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    eprintln!("[PATCHBAY] Initializing — raw cpal I/O, SpeexDSP AEC...");
    let cfg = PatchbayConfig::load();
    let host = cpal::default_host();

    // --- INPUT DEVICE ---
    let input_dev = find_device(&host, &cfg.input_device, true)
        .expect(&format!("Input '{}' not found", cfg.input_device));
    let input_config = input_dev.default_input_config()?;
    let device_rate = input_config.sample_rate().0;
    let in_channels = input_config.channels() as usize;
    eprintln!("[PATCHBAY] Mic: {} ({}Hz, {}ch)", input_dev.name()?, device_rate, in_channels);
    if input_config.sample_format() != cpal::SampleFormat::F32 {
        eprintln!("[PATCHBAY] FATAL: not F32"); std::process::exit(1);
    }
    if device_rate != 48000 {
        eprintln!("[PATCHBAY] FATAL: {}Hz != 48kHz", device_rate); std::process::exit(1);
    }

    // --- OUTPUT DEVICE (raw cpal, NO rodio) ---
    let output_dev = find_device(&host, &cfg.output_device, false)
        .expect(&format!("Output '{}' not found", cfg.output_device));
    let out_supported = output_dev.supported_output_configs()?
        .filter(|c| c.max_sample_rate().0 >= 24000)
        .next().expect("No suitable output config");
    let out_rate = out_supported.with_max_sample_rate().sample_rate().0;
    let out_channels = out_supported.channels() as usize;
    let out_config = cpal::StreamConfig {
        channels: out_channels as u16,
        sample_rate: cpal::SampleRate(out_rate),
        buffer_size: cpal::BufferSize::Default,
    };
    eprintln!("[PATCHBAY] Spk: {} ({}Hz, {}ch) — RAW CPAL, no rodio", output_dev.name()?, out_rate, out_channels);

    // --- SHARED BUFFERS ---
    // TTS audio waiting to be played (f32 at 24kHz from tts-ant)
    let playback_queue: Arc<Mutex<VecDeque<f32>>> = Arc::new(Mutex::new(VecDeque::new()));
    // AEC speaker reference (i16 at 16kHz) — filled by output callback at playback time
    let speaker_ref: Arc<Mutex<VecDeque<i16>>> = Arc::new(Mutex::new(VecDeque::new()));
    // Mic capture buffer
    let capture_buf: Arc<Mutex<Vec<f32>>> = Arc::new(Mutex::new(Vec::with_capacity(32000)));

    // --- OUTPUT STREAM (raw cpal) ---
    // The callback fires when hardware needs samples. We push to speaker AND AEC ref simultaneously.
    let pq_out = playback_queue.clone();
    let sr_out = speaker_ref.clone();
    let tts_rate = 24000u32;

    let output_stream = output_dev.build_output_stream(
        &out_config,
        move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
            let mut pq = pq_out.lock().unwrap();
            // Buffer for AEC reference — accumulate 48kHz samples played this callback
            let mut played_48k: Vec<f32> = Vec::new();

            for frame in data.chunks_mut(out_channels) {
                // Get next sample from TTS queue (24kHz)
                // Need to resample 24k → out_rate on the fly
                // For simplicity: repeat each 24k sample (out_rate/24000) times
                let sample = pq.pop_front().unwrap_or(0.0);
                for ch in frame.iter_mut() {
                    *ch = sample;
                }
                // Collect for AEC reference at output rate
                played_48k.push(sample);
            }

            // Downsample played audio to 16kHz for AEC reference
            // First upsample 24k→48k via linear interp, then LPF+decimate
            // But played_48k is already at out_rate — need to handle rate
            if !played_48k.is_empty() && played_48k.iter().any(|&s| s.abs() > 0.001) {
                // Upsample from output rate to 48kHz if needed
                let samples_48k = if out_rate == 48000 {
                    played_48k
                } else if out_rate == 44100 {
                    // 44.1k→48k approximate: slight stretch
                    let ratio = 48000.0 / 44100.0;
                    let out_len = (played_48k.len() as f64 * ratio) as usize;
                    let mut up = Vec::with_capacity(out_len);
                    for i in 0..out_len {
                        let src = i as f64 / ratio;
                        let idx = src as usize;
                        let s = played_48k[idx.min(played_48k.len() - 1)];
                        up.push(s);
                    }
                    up
                } else {
                    played_48k
                };

                let ref_16k = downsample_to_aec(&samples_48k);
                if let Ok(mut sr) = sr_out.lock() {
                    sr.extend(ref_16k.iter());
                    let len = sr.len();
                    if len > 32000 { sr.drain(..len - 32000); }
                }
            }
        },
        |err| eprintln!("[PATCHBAY] Output Error: {}", err),
        None,
    )?;
    output_stream.play()?;
    eprintln!("[PATCHBAY] Output stream: PLAYING (raw cpal)");

    // --- INPUT STREAM ---
    let cb_out = capture_buf.clone();
    let input_stream = input_dev.build_input_stream(
        &input_config.into(),
        move |data: &[f32], _: &cpal::InputCallbackInfo| {
            let mut buf = cb_out.lock().unwrap();
            for frame in data.chunks(in_channels) {
                let sample: f32 = frame.iter().sum::<f32>() / in_channels as f32;
                buf.push(sample);
            }
        },
        |err| eprintln!("[PATCHBAY] Input Error: {}", err),
        None,
    )?;
    input_stream.play()?;

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
    eprintln!("[PATCHBAY] AEC ref captured at hardware playback time — no rodio delay");
    eprintln!("[PATCHBAY] Bus: stt_raw <-> tts_audio — READY");

    // --- MAIN LOOP ---
    loop {
        // 1. Receive TTS audio from bus, queue for playback
        while let Some(sample) = spk_sub.receive()? {
            let raw = sample.payload();
            if raw.len() % 4 != 0 { continue; }
            let samples: Vec<f32> = raw.chunks_exact(4)
                .map(|c| f32::from_le_bytes([c[0], c[1], c[2], c[3]]))
                .collect();
            if samples.is_empty() { continue; }

            // Resample 24kHz → output rate (e.g. 44100Hz) via linear interpolation
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

            let mut pq = playback_queue.lock().unwrap();
            pq.extend(resampled.iter());
            let len = pq.len();
            if len > 480000 { pq.drain(..len - 480000); } // 10s at out_rate
        }

        // 2. Process mic capture through AEC
        {
            let mut buf = capture_buf.lock().unwrap();
            let needed = AEC_FRAME_SIZE * 3; // 480 samples at 48kHz = 160 at 16kHz
            if buf.len() >= needed {
                let mut all_clean_48k: Vec<f32> = Vec::new();

                while buf.len() >= needed {
                    let frame_48k: Vec<f32> = buf.drain(..needed).collect();
                    let mic_16k = downsample_to_aec(&frame_48k);

                    let echo_16k: Vec<i16> = {
                        let mut sr = speaker_ref.lock().unwrap();
                        if sr.len() >= AEC_FRAME_SIZE {
                            sr.drain(..AEC_FRAME_SIZE).collect()
                        } else {
                            vec![0i16; AEC_FRAME_SIZE]
                        }
                    };

                    let mut clean_16k = vec![0i16; AEC_FRAME_SIZE];
                    if let Ok(aec) = aec.lock() {
                        aec.cancel_echo(&mic_16k, &echo_16k, &mut clean_16k);
                    }

                    // Linear interpolation upsample 16kHz → 48kHz
                    for i in 0..clean_16k.len() {
                        let s0 = clean_16k[i] as f32 / 32768.0;
                        let s1 = if i + 1 < clean_16k.len() {
                            clean_16k[i + 1] as f32 / 32768.0
                        } else { s0 };
                        all_clean_48k.push(s0);
                        all_clean_48k.push(s0 + (s1 - s0) / 3.0);
                        all_clean_48k.push(s0 + (s1 - s0) * 2.0 / 3.0);
                    }
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
