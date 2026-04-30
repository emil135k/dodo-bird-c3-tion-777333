//! Twilio Ant — WebSocket bridge for Twilio voice calls
//!
//! Bridges Twilio Media Streams (mu-law 8kHz) to the iceoryx2 swarm.
//!   Inbound:  Twilio → mu-law decode → upsample 8k→16k → stt_raw bus
//!   Outbound: tts_audio bus → downsample 24k→8k → mu-law encode → Twilio
//!
//! Architecture:
//!   - std::thread for iceoryx2 (publisher/subscriber are !Send)
//!   - tokio async for Twilio WebSocket + HTTP
//!   - mpsc channels bridge between the two worlds

use iceoryx2::prelude::*;
use iceoryx2_bb_system_types::path::Path;
use iceoryx2_bb_container::semantic_string::SemanticString;

use axum::{
    extract::ws::{Message, WebSocket, WebSocketUpgrade},
    routing::{get, post},
    Router,
};
use base64::Engine;
use futures_util::{SinkExt, StreamExt};
use serde::Deserialize;
use serde_json::{json, Value};
use std::collections::VecDeque;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use tokio::sync::Mutex;

const CONFIG_PATH: &str = "/Users/rocketman/crystalballmini/hypAiAssist/config/twilio-ant.json";
const TTS_SAMPLE_RATE: f32 = 24000.0;
const PHONE_RATE: f32 = 8000.0;
const STT_RATE: f32 = 16000.0;

#[derive(Deserialize, Debug)]
struct TwilioConfig {
    #[serde(default = "d_from")]
    twilio_from: String,
    #[serde(default = "d_url")]
    server_url: String,
    #[serde(default = "d_port")]
    port: u16,
}
fn d_from() -> String { "+18136076219".into() }
fn d_url() -> String { "https://emils-macbook-pro.tail12e909.ts.net".into() }
fn d_port() -> u16 { 5050 }

impl Default for TwilioConfig {
    fn default() -> Self {
        Self { twilio_from: d_from(), server_url: d_url(), port: d_port() }
    }
}

impl TwilioConfig {
    fn load() -> Self {
        match std::fs::read_to_string(CONFIG_PATH) {
            Ok(text) => serde_json::from_str(&text).unwrap_or_else(|e| {
                eprintln!("[TWILIO] Config error: {} — defaults", e);
                Self::default()
            }),
            Err(_) => { eprintln!("[TWILIO] No config — defaults"); Self::default() }
        }
    }
}

#[inline]
fn mulaw_decode(mulaw: u8) -> i16 {
    audio_codec_algorithms::decode_ulaw(mulaw)
}

#[inline]
fn mulaw_encode(sample: i16) -> u8 {
    audio_codec_algorithms::encode_ulaw(sample)
}

fn resample_linear(src: &[f32], src_rate: f32, dst_rate: f32) -> Vec<f32> {
    if src.is_empty() || (src_rate - dst_rate).abs() < 1.0 { return src.to_vec(); }
    let ratio = dst_rate as f64 / src_rate as f64;
    let out_len = (src.len() as f64 * ratio).round() as usize;
    let mut out = Vec::with_capacity(out_len);
    for i in 0..out_len {
        let pos = i as f64 / ratio;
        let base = pos.floor() as usize;
        let frac = (pos - base as f64) as f32;
        let base = base.min(src.len() - 1);
        let next = (base + 1).min(src.len() - 1);
        out.push(src[base] * (1.0 - frac) + src[next] * frac);
    }
    out
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    eprintln!("[TWILIO] Starting Twilio Ant...");
    let cfg = TwilioConfig::load();
    eprintln!("[TWILIO] From: {}, Server: {}, Port: {}", cfg.twilio_from, cfg.server_url, cfg.port);

    // Channel: async WS handler → std::thread iceoryx2 publisher
    // Sends f32 PCM audio bytes from decoded Twilio stream to bus
    let (iox_tx, iox_rx) = std::sync::mpsc::channel::<Vec<u8>>();

    // Shared state
    let call_active = Arc::new(AtomicBool::new(false));
    let speaking = Arc::new(AtomicBool::new(false));
    let twilio_outbound: Arc<std::sync::Mutex<VecDeque<u8>>> = Arc::new(std::sync::Mutex::new(VecDeque::new()));

    // iceoryx2 thread — owns publisher and subscriber (both are !Send)
    let iox_out = twilio_outbound.clone();
    let iox_active = call_active.clone();
    let iox_speaking = speaking.clone();
    std::thread::spawn(move || {
        let mut iox_cfg = Config::default();
        iox_cfg.global.set_root_path(&Path::new(b"/tmp/iceoryx2/").unwrap());
        let node = NodeBuilder::new().config(&iox_cfg).create::<ipc::Service>()
            .expect("iceoryx2 node");

        // Publisher: caller audio → stt_raw
        let raw_svc = node.service_builder(&"stt_raw".try_into().unwrap())
            .publish_subscribe::<[u8]>()
            .open_or_create().expect("stt_raw service");
        let mic_pub = raw_svc.publisher_builder()
            .initial_max_slice_len(4 * 1024 * 1024)
            .create().expect("stt_raw publisher");

        // Subscriber: tts_audio → mulaw for Twilio
        let audio_svc = node.service_builder(&"tts_audio".try_into().unwrap())
            .publish_subscribe::<[u8]>()
            .open_or_create().expect("tts_audio service");
        let spk_sub = audio_svc.subscriber_builder().create().expect("tts_audio subscriber");

        eprintln!("[TWILIO] Bus: pub='stt_raw' sub='tts_audio' — READY");

        loop {
            // 1. Receive caller audio from channel, publish to stt_raw
            while let Ok(bytes) = iox_rx.try_recv() {
                if let Ok(loan) = mic_pub.loan_slice_uninit(bytes.len()) {
                    let _ = loan.write_from_slice(&bytes).send();
                }
            }

            // 2. Read TTS audio from bus, convert to mulaw, queue for Twilio
            if iox_active.load(Ordering::Relaxed) {
                while let Ok(Some(sample)) = spk_sub.receive() {
                    let raw = sample.payload();
                    let samples: Vec<f32> = raw.chunks(4)
                        .map(|c| f32::from_le_bytes([c[0], c[1], c[2], c[3]]))
                        .collect();
                    if samples.is_empty() { continue; }

                    let phone_8k = resample_linear(&samples, TTS_SAMPLE_RATE, PHONE_RATE);
                    let peak = phone_8k.iter().map(|s| s.abs()).fold(0.0f32, f32::max);
                    let gain = if peak > 0.001 { 0.85 / peak } else { 1.0 };
                    let mulaw: Vec<u8> = phone_8k.iter()
                        .map(|&s| mulaw_encode(((s * gain) * 32767.0).clamp(-32768.0, 32767.0) as i16))
                        .collect();

                    iox_speaking.store(true, Ordering::Relaxed);
                    let dur = mulaw.len() as f32 / PHONE_RATE;
                    eprintln!("[TWILIO] TTS→phone: {:.1}s ({} mulaw bytes)", dur, mulaw.len());
                    if let Ok(mut ob) = iox_out.lock() { ob.extend(mulaw); }
                }
            }

            std::thread::sleep(std::time::Duration::from_millis(10));
        }
    });

    // HTTP server
    let ws_url = cfg.server_url.replace("https://", "wss://").replace("http://", "ws://");

    let app = Router::new()
        .route("/voice", post({
            let ws_url = ws_url.clone();
            move || {
                let ws_url = ws_url.clone();
                async move {
                    let twiml = format!(
                        r#"<?xml version="1.0" encoding="UTF-8"?><Response><Connect><Stream url="{}/ws" /></Connect></Response>"#,
                        ws_url
                    );
                    eprintln!("[TWILIO] Incoming call → {}/ws", ws_url);
                    ([("content-type", "text/xml")], twiml)
                }
            }
        }))
        .route("/ws", get({
            let ca = call_active.clone();
            let sp = speaking.clone();
            let ob = twilio_outbound.clone();
            let tx = iox_tx.clone();
            move |ws: WebSocketUpgrade| {
                let ca = ca.clone();
                let sp = sp.clone();
                let ob = ob.clone();
                let tx = tx.clone();
                async move {
                    ws.on_upgrade(move |socket| handle_twilio_ws(socket, ca, sp, ob, tx))
                }
            }
        }))
        .route("/health", get(|| async { "ok" }));

    eprintln!("[TWILIO] Listening on port {}", cfg.port);
    eprintln!("[TWILIO] Webhook: {}/voice", cfg.server_url);
    eprintln!("[TWILIO] READY — waiting for calls");

    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{}", cfg.port)).await?;
    axum::serve(listener, app).await?;
    Ok(())
}

async fn handle_twilio_ws(
    socket: WebSocket,
    call_active: Arc<AtomicBool>,
    speaking: Arc<AtomicBool>,
    twilio_outbound: Arc<std::sync::Mutex<VecDeque<u8>>>,
    iox_tx: std::sync::mpsc::Sender<Vec<u8>>,
) {
    if call_active.load(Ordering::Relaxed) {
        eprintln!("[WS] Rejected — another stream already active");
        let _ = socket.close().await;
        return;
    }

    eprintln!("[WS] Twilio connected");
    call_active.store(true, Ordering::Relaxed);

    let (ws_tx, mut ws_rx) = socket.split();
    let ws_tx = Arc::new(Mutex::new(ws_tx));
    let stream_sid: Arc<Mutex<Option<String>>> = Arc::new(Mutex::new(None));
    let mark_pending = Arc::new(AtomicBool::new(false));

    // Outbound sender — drains mulaw queue → Twilio in 160-byte chunks (20ms at 8kHz)
    let ws_tx_out = Arc::clone(&ws_tx);
    let sid_out = Arc::clone(&stream_sid);
    let ob_send = twilio_outbound.clone();
    let ca_send = call_active.clone();
    let mp_send = mark_pending.clone();

    tokio::spawn(async move {
        loop {
            if !ca_send.load(Ordering::Relaxed) { break; }
            let chunk: Option<Vec<u8>> = {
                let mut ob = ob_send.lock().unwrap();
                if ob.len() >= 160 { Some(ob.drain(..160).collect()) } else { None }
            };
            if let Some(chunk) = chunk {
                let payload = base64::engine::general_purpose::STANDARD.encode(&chunk);
                let sid = sid_out.lock().await.clone();
                if let Some(ref sid) = sid {
                    let msg = json!({"event":"media","streamSid":sid,"media":{"payload":payload}});
                    let mut tx = ws_tx_out.lock().await;
                    let _ = tx.send(Message::Text(msg.to_string())).await;
                }
            } else {
                if mp_send.swap(false, Ordering::Relaxed) {
                    let sid = sid_out.lock().await.clone();
                    if let Some(ref sid) = sid {
                        let mark_msg = json!({"event":"mark","streamSid":sid,"mark":{"name":"tts-done"}});
                        let mut tx = ws_tx_out.lock().await;
                        let _ = tx.send(Message::Text(mark_msg.to_string())).await;
                        eprintln!("[MARK] Sent — waiting for playback confirmation");
                    }
                }
                tokio::time::sleep(std::time::Duration::from_millis(20)).await;
            }
        }
    });

    // Inbound receiver — Twilio → decode → channel → iceoryx2 thread → stt_raw bus
    while let Some(Ok(msg)) = ws_rx.next().await {
        let Message::Text(text) = msg else { continue };
        let json: Value = serde_json::from_str(&text).unwrap_or(json!({}));
        match json["event"].as_str() {
            Some("connected") => eprintln!("[WS] Protocol connected"),
            Some("start") => {
                let sid = json["start"]["streamSid"].as_str()
                    .or_else(|| json["streamSid"].as_str()).map(|s| s.to_string());
                eprintln!("[WS] Stream started — sid={:?}", sid);
                *stream_sid.lock().await = sid;
            }
            Some("media") => {
                if speaking.load(Ordering::Relaxed) { continue; } // Echo gate
                if let Some(payload) = json["media"]["payload"].as_str() {
                    if let Ok(raw_mulaw) = base64::engine::general_purpose::STANDARD.decode(payload) {
                        // Decode mu-law → f32 PCM, upsample 8kHz → 16kHz for STT
                        let pcm_8k: Vec<f32> = raw_mulaw.iter()
                            .map(|&mu| mulaw_decode(mu) as f32 / 32768.0)
                            .collect();
                        let pcm_16k = resample_linear(&pcm_8k, PHONE_RATE, STT_RATE);
                        let bytes: Vec<u8> = pcm_16k.iter().flat_map(|s| s.to_le_bytes()).collect();
                        let _ = iox_tx.send(bytes);
                    }
                }
            }
            Some("mark") => {
                let mark_name = json["mark"]["name"].as_str().unwrap_or("");
                eprintln!("[MARK] Received '{}' — playback confirmed", mark_name);
                if mark_name == "tts-done" {
                    let sp = speaking.clone();
                    tokio::spawn(async move {
                        tokio::time::sleep(std::time::Duration::from_millis(500)).await;
                        sp.store(false, Ordering::Relaxed);
                        eprintln!("[MARK] Unmuted — listening for caller");
                    });
                }
            }
            Some("stop") => { eprintln!("[WS] Stream stopped"); break; }
            _ => {}
        }
    }

    eprintln!("[WS] Call ended");
    call_active.store(false, Ordering::Relaxed);
    speaking.store(false, Ordering::Relaxed);
}
