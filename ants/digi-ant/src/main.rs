//! Digi Ant — Digital Signal Processing
//! The Queen was here. 2026-04-30
//! Emil is here!
//! Pure signal processing. Zero networking. Proven crates only.
//!   rubato:                  sinc-based resampling with anti-aliasing
//!   audio-codec-algorithms:  ITU-T G.711 mu-law encode/decode
//!   dasp:                    sample format conversion
//!
//! Bidirectional conversion between TTS audio (24kHz f32) and phone audio (8kHz mu-law).
//!
//! Bus topology:
//!   tts_audio (24kHz f32) → digi-ant → phone_out (8kHz mu-law bytes)
//!   phone_in (8kHz mu-law bytes) → digi-ant → phone_stt (16kHz f32)

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
    #[serde(default = "d_vad_closure_silence_ms")]
    vad_closure_silence_ms: u32,
}
fn d_tts_rate() -> u32 { 24000 }
fn d_phone_rate() -> u32 { 8000 }
fn d_stt_rate() -> u32 { 16000 }
fn d_normalize_peak() -> f32 { 0.85 }
fn d_vad_closure_silence_ms() -> u32 { 512 }

impl Default for DigiConfig {
    fn default() -> Self {
        Self { tts_rate: d_tts_rate(), phone_rate: d_phone_rate(),
               stt_rate: d_stt_rate(), normalize_peak: d_normalize_peak(),
               vad_closure_silence_ms: d_vad_closure_silence_ms() }
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
fn resample_sinc(src: &[f32], src_rate: u32, dst_rate: u32) -> Vec<f32> {
    if src.is_empty() || src_rate == dst_rate { return src.to_vec(); }
    let ratio = dst_rate as f64 / src_rate as f64;
    let params = SincInterpolationParameters {
        sinc_len: 64,
        f_cutoff: 0.88,
        interpolation: SincInterpolationType::Linear,
        oversampling_factor: 128,
        window: WindowFunction::BlackmanHarris2,
    };
    let mut resampler = SincFixedIn::<f32>::new(
        ratio, 2.0, params, src.len(), 1
    ).expect("rubato init");
    let waves_in = vec![src.to_vec()];
    match resampler.process(&waves_in, None) {
        Ok(waves_out) => waves_out.into_iter().next().unwrap_or_default(),
        Err(e) => { eprintln!("[DIGI] Resample error: {}", e); src.to_vec() }
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

    // Publish to phone_stt (16kHz f32 — typed slice for alignment safety)
    let stt_svc = node.service_builder(&"phone_stt".try_into()?)
        .publish_subscribe::<[f32]>()
        .open_or_create()?;
    let stt_pub = stt_svc.publisher_builder()
        .initial_max_slice_len(1024 * 1024)
        .create()?;

    eprintln!("[DIGI] Bus: sub='tts_audio','phone_in' pub='phone_out','phone_stt' — READY");

    // Persistent resampler — maintains filter state across chunks (prevents clicking + sample loss)
    const PHONE_IN_BUFFER_SIZE: usize = 1600; // 200ms at 8kHz
    let mut phone_in_buf: Vec<u8> = Vec::with_capacity(PHONE_IN_BUFFER_SIZE * 2);
    let ratio_8to16 = cfg.stt_rate as f64 / cfg.phone_rate as f64;
    let params = SincInterpolationParameters {
        sinc_len: 64, f_cutoff: 0.88,
        interpolation: SincInterpolationType::Linear,
        oversampling_factor: 128, window: WindowFunction::BlackmanHarris2,
    };
    let mut resampler_8to16 = SincFixedIn::<f32>::new(
        ratio_8to16, 2.0, params, PHONE_IN_BUFFER_SIZE, 1
    ).expect("8→16 resampler init");
    eprintln!("[DIGI] Persistent 8k→16k resampler created (chunk={})", PHONE_IN_BUFFER_SIZE);
    const FLUSH_TIMEOUT: std::time::Duration = std::time::Duration::from_millis(300);
    let mut last_phone_in_data_at = std::time::Instant::now();
    let mut phone_in_has_pending_data = false;

    // Stream stats (Vale's canary metrics)
    let mut stats_packets: u64 = 0;
    let mut stats_bytes_in: u64 = 0;
    let mut stats_output_samples: u64 = 0;
    let mut stats_full_chunks: u64 = 0;
    let mut stats_flush_count: u64 = 0;
    let mut stats_flush_padding_bytes: u64 = 0;
    let mut stats_silence_hint_samples: u64 = 0;
    let mut stats_gap_min_ms: f64 = f64::MAX;
    let mut stats_gap_max_ms: f64 = 0.0;
    let mut stats_gap_sum_ms: f64 = 0.0;
    let mut stats_last_packet_at: Option<std::time::Instant> = None;
    let mut stats_stream_start: Option<std::time::Instant> = None;
    let mut stats_stream_active = false;

    loop {
        // === TTS → Phone: 24kHz f32 → 8kHz mu-law ===
        // Contract: tts-ant publishes one complete utterance per message.
        // Per-call resampler is intentional — each utterance is independent.
        // Do NOT replace with persistent streaming resampler unless tts-ant
        // changes to emit partial/chunked audio for a single utterance.
        while let Some(sample) = tts_sub.receive()? {
            let raw = sample.payload();
            if raw.len() % 4 != 0 {
                eprintln!("[DIGI] WARN: TTS payload {} bytes not aligned to 4 (discarding {} remainder)", raw.len(), raw.len() % 4);
            }
            let samples: Vec<f32> = raw.chunks_exact(4)
                .map(|c| f32::from_le_bytes([c[0], c[1], c[2], c[3]]))
                .collect();
            if samples.is_empty() { continue; }

            let mut phone_8k = resample_sinc(&samples, cfg.tts_rate, cfg.phone_rate);
            normalize(&mut phone_8k, cfg.normalize_peak);
            let mulaw = pcm_to_mulaw(&phone_8k);

            let dur = mulaw.len() as f32 / cfg.phone_rate as f32;
            eprintln!("[DIGI] TTS→phone: {:.1}s ({}→{} Hz, {} mulaw bytes)",
                dur, cfg.tts_rate, cfg.phone_rate, mulaw.len());

            if let Ok(loan) = phone_out_pub.loan_slice_uninit(mulaw.len()) {
                let _ = loan.write_from_slice(&mulaw).send();
            }
        }

        // === Phone → STT: 8kHz mu-law → 16kHz f32 ===
        while let Some(sample) = phone_in_sub.receive()? {
            let mulaw = sample.payload();
            if mulaw.is_empty() { continue; }

            let now = std::time::Instant::now();
            if stats_stream_start.is_none() { stats_stream_start = Some(now); }
            if let Some(prev) = stats_last_packet_at {
                let gap = now.duration_since(prev).as_secs_f64() * 1000.0;
                if gap < stats_gap_min_ms { stats_gap_min_ms = gap; }
                if gap > stats_gap_max_ms { stats_gap_max_ms = gap; }
                stats_gap_sum_ms += gap;
            }
            stats_last_packet_at = Some(now);
            stats_packets += 1;
            stats_bytes_in += mulaw.len() as u64;

            phone_in_buf.extend_from_slice(mulaw);
            last_phone_in_data_at = now;
            phone_in_has_pending_data = true;
            stats_stream_active = true;
        }

        while phone_in_buf.len() >= PHONE_IN_BUFFER_SIZE {
            let chunk: Vec<u8> = phone_in_buf.drain(..PHONE_IN_BUFFER_SIZE).collect();
            let pcm_8k = mulaw_to_pcm(&chunk);
            let pcm_16k = resampler_8to16.process(&[pcm_8k], None)
                .unwrap_or_default().into_iter().next().unwrap_or_default();
            stats_full_chunks += 1;
            stats_output_samples += pcm_16k.len() as u64;

            let dur = pcm_16k.len() as f32 / cfg.stt_rate as f32;
            eprintln!("[DIGI] Phone→STT: {:.1}s ({}→{} Hz, {} samples)",
                dur, cfg.phone_rate, cfg.stt_rate, pcm_16k.len());

            if let Ok(loan) = stt_pub.loan_slice_uninit(pcm_16k.len()) {
                let _ = loan.write_from_slice(&pcm_16k).send();
            }
        }

        // Clear pending flag if buffer was fully drained
        if phone_in_buf.is_empty() {
            phone_in_has_pending_data = false;
        }

        // Flush remaining buffer once after timeout with no new data
        if !phone_in_buf.is_empty() && phone_in_has_pending_data && last_phone_in_data_at.elapsed() > FLUSH_TIMEOUT {
            let padding = PHONE_IN_BUFFER_SIZE - phone_in_buf.len();
            stats_flush_padding_bytes += padding as u64;
            stats_flush_count += 1;

            eprintln!("[DIGI] Phone→STT: FLUSH {} bytes remainder (+{} padding)", phone_in_buf.len(), padding);
            while phone_in_buf.len() < PHONE_IN_BUFFER_SIZE {
                phone_in_buf.push(0xFF); // μ-law silence
            }
            let chunk: Vec<u8> = phone_in_buf.drain(..PHONE_IN_BUFFER_SIZE).collect();
            let pcm_8k = mulaw_to_pcm(&chunk);
            let pcm_16k = resampler_8to16.process(&[pcm_8k], None)
                .unwrap_or_default().into_iter().next().unwrap_or_default();
            stats_output_samples += pcm_16k.len() as u64;
            if let Ok(loan) = stt_pub.loan_slice_uninit(pcm_16k.len()) {
                let _ = loan.write_from_slice(&pcm_16k).send();
            }

            // VAD closure hint: emit silence so downstream VAD can close its utterance
            // via its own state machine (silence_frames_to_end).
            // This is NOT session EOS — digi-ant does not own session truth.
            // This is a data-plane signal: "no more speech audio in this gap."
            let vad_closure_samples = (cfg.vad_closure_silence_ms as usize * cfg.stt_rate as usize) / 1000;
            let silence = vec![0.0f32; vad_closure_samples];
            if let Ok(loan) = stt_pub.loan_slice_uninit(vad_closure_samples) {
                let _ = loan.write_from_slice(&silence).send();
            }
            stats_silence_hint_samples += vad_closure_samples as u64;
            eprintln!("[DIGI] Phone→STT: VAD closure silence ({}ms)", cfg.vad_closure_silence_ms);

            phone_in_has_pending_data = false;
            stats_stream_active = false;
        }

        // Exact-boundary stream end: buffer empty but stream was active and data stopped
        if phone_in_buf.is_empty() && !phone_in_has_pending_data && stats_stream_active
            && last_phone_in_data_at.elapsed() > FLUSH_TIMEOUT {
            // Emit VAD closure silence hint (same as flush path)
            let vad_closure_samples = (cfg.vad_closure_silence_ms as usize * cfg.stt_rate as usize) / 1000;
            let silence = vec![0.0f32; vad_closure_samples];
            if let Ok(loan) = stt_pub.loan_slice_uninit(vad_closure_samples) {
                let _ = loan.write_from_slice(&silence).send();
            }
            stats_silence_hint_samples += vad_closure_samples as u64;
            eprintln!("[DIGI] Phone→STT: VAD closure silence — exact boundary ({}ms)", cfg.vad_closure_silence_ms);
            stats_stream_active = false;
        }

        // Print stats when stream ends (flush or exact-boundary VAD closure hint)
        if !stats_stream_active && stats_packets > 0 {
            let input_ms = stats_bytes_in as f64 / cfg.phone_rate as f64 * 1000.0;
            let audio_ms = stats_output_samples as f64 / cfg.stt_rate as f64 * 1000.0;
            let padding_ms = stats_flush_padding_bytes as f64 / cfg.phone_rate as f64 * 1000.0;
            let silence_hint_ms = stats_silence_hint_samples as f64 / cfg.stt_rate as f64 * 1000.0;
            let total_ms = audio_ms + silence_hint_ms;
            let real_audio_ms = (audio_ms - padding_ms).max(0.0);
            let ratio_total = if input_ms > 0.0 { total_ms / input_ms } else { 0.0 };
            let ratio_real = if input_ms > 0.0 { real_audio_ms / input_ms } else { 0.0 };
            let avg_gap = if stats_packets > 1 { stats_gap_sum_ms / (stats_packets - 1) as f64 } else { 0.0 };
            eprintln!("[DIGI] ── STREAM STATS ──────────────────────────");
            eprintln!("[DIGI]   packets:       {}", stats_packets);
            eprintln!("[DIGI]   bytes_in:      {}", stats_bytes_in);
            eprintln!("[DIGI]   full_chunks:   {}", stats_full_chunks);
            eprintln!("[DIGI]   flush_count:   {} (padding: {} bytes)", stats_flush_count, stats_flush_padding_bytes);
            eprintln!("[DIGI]   silence_hint:  {:.1}ms ({} samples)", silence_hint_ms, stats_silence_hint_samples);
            eprintln!("[DIGI]   input_audio:   {:.1}ms", input_ms);
            eprintln!("[DIGI]   output_audio:  {:.1}ms (resampled chunks)", audio_ms);
            eprintln!("[DIGI]   output_total:  {:.1}ms (audio + silence hint)", total_ms);
            eprintln!("[DIGI]   duration_ratio: {:.4} (total) / {:.4} (real audio only)", ratio_total, ratio_real);
            let min_gap = if stats_packets > 1 { stats_gap_min_ms } else { 0.0 };
            eprintln!("[DIGI]   gap_ms:        min={:.1} avg={:.1} max={:.1}", min_gap, avg_gap, stats_gap_max_ms);
            eprintln!("[DIGI] ────────────────────────────────────────");

            // Reset stats for next stream
            stats_packets = 0; stats_bytes_in = 0; stats_output_samples = 0;
            stats_full_chunks = 0; stats_flush_count = 0; stats_flush_padding_bytes = 0; stats_silence_hint_samples = 0;
            stats_gap_min_ms = f64::MAX; stats_gap_max_ms = 0.0; stats_gap_sum_ms = 0.0;
            stats_last_packet_at = None; stats_stream_start = None;
        }

        std::thread::sleep(std::time::Duration::from_millis(10));
    }
}
