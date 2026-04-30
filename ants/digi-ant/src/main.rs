//! Digi Ant — Digital Signal Processing
//! The Queen was here. 2026-04-30
//!
//! Pure signal processing. Zero networking. Proven crates only.
//!   rubato:                  sinc-based resampling with anti-aliasing
//!   audio-codec-algorithms:  ITU-T G.711 mu-law encode/decode
//!   dasp:                    sample format conversion
//!
//! Bidirectional conversion between TTS audio (24kHz f32) and phone audio (8kHz mu-law).
//!
//! Bus topology:
//!   tts_audio (24kHz f32) → digi-ant → phone_out (8kHz mu-law bytes)
//!   phone_in (8kHz mu-law bytes) → digi-ant → stt_raw (16kHz f32)

use iceoryx2::prelude::*;
use iceoryx2_bb_system_types::path::Path;
use iceoryx2_bb_container::semantic_string::SemanticString;
use rubato::{SincFixedIn, SincInterpolationParameters, SincInterpolationType, WindowFunction, Resampler};
use serde::Deserialize;

const CONFIG_PATH: &str = "/Users/rocketman/crystalballmini/hypAiAssist/config/digi-ant.json";

#[derive(Deserialize, Debug)]
struct DigiConfig {
    #[serde(default = "d_tts_rate")]
    tts_rate: u32,
    #[serde(default = "d_phone_rate")]
    phone_rate: u32,
    #[serde(default = "d_stt_rate")]
    stt_rate: u32,
    #[serde(default = "d_normalize_peak")]
    normalize_peak: f32,
}
fn d_tts_rate() -> u32 { 24000 }
fn d_phone_rate() -> u32 { 8000 }
fn d_stt_rate() -> u32 { 16000 }
fn d_normalize_peak() -> f32 { 0.85 }

impl Default for DigiConfig {
    fn default() -> Self {
        Self { tts_rate: d_tts_rate(), phone_rate: d_phone_rate(),
               stt_rate: d_stt_rate(), normalize_peak: d_normalize_peak() }
    }
}

impl DigiConfig {
    fn load() -> Self {
        match std::fs::read_to_string(CONFIG_PATH) {
            Ok(text) => serde_json::from_str(&text).unwrap_or_else(|e| {
                eprintln!("[DIGI] Config error: {} — defaults", e);
                Self::default()
            }),
            Err(_) => { eprintln!("[DIGI] No config — defaults"); Self::default() }
        }
    }
}

/// Sinc resampler with anti-aliasing — rubato SincFixedIn
/// CRITICAL: FastFixedIn has NO anti-aliasing. SincFixedIn's sinc filter prevents frequency folding.
fn resample_sinc(src: &[f32], src_rate: u32, dst_rate: u32) -> Vec<f32> {
    if src.is_empty() || src_rate == dst_rate { return src.to_vec(); }

    let ratio = dst_rate as f64 / src_rate as f64;
    let params = SincInterpolationParameters {
        sinc_len: 256,
        f_cutoff: 0.925, // steep rolloff just below Nyquist
        interpolation: SincInterpolationType::Linear,
        oversampling_factor: 256,
        window: WindowFunction::BlackmanHarris2,
    };

    let mut resampler = SincFixedIn::<f32>::new(
        ratio, 2.0, params, src.len(), 1
    ).expect("rubato init");

    let waves_in = vec![src.to_vec()];
    match resampler.process(&waves_in, None) {
        Ok(waves_out) => waves_out.into_iter().next().unwrap_or_default(),
        Err(e) => {
            eprintln!("[DIGI] Resample error: {} — passing through", e);
            src.to_vec()
        }
    }
}

/// Normalize audio to target peak — uses full dynamic range without clipping
fn normalize(samples: &mut [f32], target_peak: f32) {
    let peak = samples.iter().map(|s| s.abs()).fold(0.0f32, f32::max);
    if peak > 0.001 {
        let gain = target_peak / peak;
        for s in samples.iter_mut() { *s *= gain; }
    }
}

/// f32 PCM → mu-law bytes (ITU-T G.711)
fn pcm_to_mulaw(samples: &[f32]) -> Vec<u8> {
    samples.iter()
        .map(|&s| {
            let clamped = (s * 32767.0).clamp(-32768.0, 32767.0) as i16;
            audio_codec_algorithms::encode_ulaw(clamped)
        })
        .collect()
}

/// mu-law bytes → f32 PCM
fn mulaw_to_pcm(mulaw: &[u8]) -> Vec<f32> {
    mulaw.iter()
        .map(|&mu| audio_codec_algorithms::decode_ulaw(mu) as f32 / 32768.0)
        .collect()
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    eprintln!("[DIGI] Starting Digital Signal Processing Ant...");
    let cfg = DigiConfig::load();
    eprintln!("[DIGI] TTS: {}Hz, Phone: {}Hz, STT: {}Hz, Peak: {}",
        cfg.tts_rate, cfg.phone_rate, cfg.stt_rate, cfg.normalize_peak);

    let mut iox = Config::default();
    iox.global.set_root_path(&Path::new(b"/tmp/iceoryx2/").unwrap());
    let node = NodeBuilder::new().config(&iox).create::<ipc::Service>()?;

    // === TTS → Phone direction ===
    // Subscribe to tts_audio (24kHz f32 from Kokoro)
    let tts_svc = node.service_builder(&"tts_audio".try_into()?)
        .publish_subscribe::<[u8]>()
        .open_or_create()?;
    let tts_sub = tts_svc.subscriber_builder().create()?;

    // Publish to phone_out (8kHz mu-law for web-ant/Twilio)
    let phone_out_svc = node.service_builder(&"phone_out".try_into()?)
        .publish_subscribe::<[u8]>()
        .open_or_create()?;
    let phone_out_pub = phone_out_svc.publisher_builder()
        .initial_max_slice_len(1024 * 1024)
        .create()?;

    // === Phone → STT direction ===
    // Subscribe to phone_in (8kHz mu-law from web-ant/Twilio)
    let phone_in_svc = node.service_builder(&"phone_in".try_into()?)
        .publish_subscribe::<[u8]>()
        .open_or_create()?;
    let phone_in_sub = phone_in_svc.subscriber_builder().create()?;

    // Publish to phone_stt (16kHz f32 for phone-silero-ant)
    let stt_svc = node.service_builder(&"phone_stt".try_into()?)
        .publish_subscribe::<[u8]>()
        .open_or_create()?;
    let stt_pub = stt_svc.publisher_builder()
        .initial_max_slice_len(4 * 1024 * 1024)
        .create()?;

    eprintln!("[DIGI] Bus: sub='tts_audio','phone_in' pub='phone_out','stt_raw' — READY");

    // Accumulator for phone_in — buffer until we have enough for a proper resample
    // 1600 mulaw bytes = 200ms at 8kHz — good chunk size for VAD downstream
    const PHONE_IN_BUFFER_SIZE: usize = 1600;
    let mut phone_in_buf: Vec<u8> = Vec::with_capacity(PHONE_IN_BUFFER_SIZE * 2);

    loop {
        // === TTS → Phone: 24kHz f32 → 8kHz mu-law ===
        while let Some(sample) = tts_sub.receive()? {
            let raw = sample.payload();
            let samples: Vec<f32> = raw.chunks(4)
                .map(|c| f32::from_le_bytes([c[0], c[1], c[2], c[3]]))
                .collect();
            if samples.is_empty() { continue; }

            // Resample 24kHz → 8kHz with sinc anti-aliasing
            let mut phone_8k = resample_sinc(&samples, cfg.tts_rate, cfg.phone_rate);
            // Normalize to use full mu-law dynamic range
            normalize(&mut phone_8k, cfg.normalize_peak);
            // Encode to mu-law
            let mulaw = pcm_to_mulaw(&phone_8k);

            let dur = mulaw.len() as f32 / cfg.phone_rate as f32;
            eprintln!("[DIGI] TTS→phone: {:.1}s ({}→{} Hz, {} mulaw bytes)",
                dur, cfg.tts_rate, cfg.phone_rate, mulaw.len());

            if let Ok(loan) = phone_out_pub.loan_slice_uninit(mulaw.len()) {
                let _ = loan.write_from_slice(&mulaw).send();
            }
        }

        // === Phone → STT: 8kHz mu-law → 16kHz f32 ===
        // Accumulate small Twilio packets into proper-sized buffers
        while let Some(sample) = phone_in_sub.receive()? {
            let mulaw = sample.payload();
            if mulaw.is_empty() { continue; }
            phone_in_buf.extend_from_slice(mulaw);
        }

        // Publish when we have enough accumulated (200ms chunks)
        while phone_in_buf.len() >= PHONE_IN_BUFFER_SIZE {
            let chunk: Vec<u8> = phone_in_buf.drain(..PHONE_IN_BUFFER_SIZE).collect();

            // Decode mu-law → f32 PCM
            let pcm_8k = mulaw_to_pcm(&chunk);
            // Resample 8kHz → 16kHz with sinc interpolation
            let pcm_16k = resample_sinc(&pcm_8k, cfg.phone_rate, cfg.stt_rate);

            let dur = pcm_16k.len() as f32 / cfg.stt_rate as f32;
            eprintln!("[DIGI] Phone→STT: {:.1}s ({}→{} Hz, {} samples)",
                dur, cfg.phone_rate, cfg.stt_rate, pcm_16k.len());

            let bytes: Vec<u8> = pcm_16k.iter().flat_map(|s| s.to_le_bytes()).collect();
            if let Ok(loan) = stt_pub.loan_slice_uninit(bytes.len()) {
                let _ = loan.write_from_slice(&bytes).send();
            }
        }

        std::thread::sleep(std::time::Duration::from_millis(10));
    }
}
