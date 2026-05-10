//! Web Ant — WebSocket/HTTP Gateway
//!
//! Pure networking. Zero audio processing. Shuttles bytes between
//! Twilio Media Streams and the iceoryx2 bus.
//!
//! Bus topology:
//!   Twilio WS inbound → phone_in bus (raw mu-law bytes, digi-ant handles conversion)
//!   phone_out bus (mu-law bytes from digi-ant) → Twilio WS outbound
//!
//! Also handles:
//!   - TwiML webhook for incoming calls
//!   - Echo gating via Twilio "mark" events
//!   - Health endpoint

use iceoryx2::prelude::*;

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

const CONFIG_PATH: &str = "/Users/rocketman/crystalballmini/hypAiAssist/config/web-ant.json";

#[derive(Deserialize, Debug)]
struct WebConfig {
    #[serde(default = "d_url")]
    server_url: String,
    #[serde(default = "d_port")]
    port: u16,
}
fn d_url() -> String { "https://emils-macbook-pro.tail12e909.ts.net".into() }
fn d_port() -> u16 { 5050 }

impl Default for WebConfig {
    fn default() -> Self { Self { server_url: d_url(), port: d_port() } }
}

impl WebConfig {
    fn load() -> Self {
        match std::fs::read_to_string(CONFIG_PATH) {
            Ok(text) => serde_json::from_str(&text).unwrap_or_else(|e| {
                eprintln!("[WEB] Config error: {} — defaults", e);
                Self::default()
            }),
            Err(_) => { eprintln!("[WEB] No config — defaults"); Self::default() }
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    eprintln!("[WEB] Starting Web Ant...");
    let cfg = WebConfig::load();
    eprintln!("[WEB] Server: {}, Port: {}", cfg.server_url, cfg.port);

    // Channel: async WS handler → std::thread iceoryx2 publisher (phone_in)
    let (iox_tx, iox_rx) = std::sync::mpsc::channel::<Vec<u8>>();

    // Shared state
    let call_active = Arc::new(AtomicBool::new(false));
    let speaking = Arc::new(AtomicBool::new(false));
    let outbound_queue: Arc<std::sync::Mutex<VecDeque<u8>>> = Arc::new(std::sync::Mutex::new(VecDeque::new()));

    // iceoryx2 thread — publisher/subscriber are !Send, must stay on one thread
    let mark_pending_main = Arc::new(AtomicBool::new(false));
    let iox_out = outbound_queue.clone();
    let iox_active = call_active.clone();
    let iox_mark = mark_pending_main.clone();
    std::thread::spawn(move || {
        let node = NodeBuilder::new().create::<ipc::Service>()
            .expect("iceoryx2 node");

        // Contract: phone_in contains raw mu-law bytes from Twilio (8kHz, u-law encoded)
        let phone_in_svc = node.service_builder(&"phone_in".try_into().unwrap())
            .publish_subscribe::<[u8]>()
            .open_or_create().expect("phone_in service");
        let phone_in_pub = phone_in_svc.publisher_builder()
            .initial_max_slice_len(1024 * 1024)
            .create().expect("phone_in publisher");

        // Contract: phone_out contains mu-law bytes from digi-ant (8kHz, u-law encoded)
        let phone_out_svc = node.service_builder(&"phone_out".try_into().unwrap())
            .publish_subscribe::<[u8]>()
            .open_or_create().expect("phone_out service");
        let phone_out_sub = phone_out_svc.subscriber_builder()
            .create().expect("phone_out subscriber");

        eprintln!("[WEB] Bus: pub='phone_in' sub='phone_out' — READY");

        loop {
            // Receive caller audio from channel, publish to phone_in
            while let Ok(bytes) = iox_rx.try_recv() {
                if let Ok(loan) = phone_in_pub.loan_slice_uninit(bytes.len()) {
                    let _ = loan.write_from_slice(&bytes).send();
                }
            }

            // Read phone_out (mu-law from digi-ant), queue for Twilio WS
            if iox_active.load(Ordering::Relaxed) {
                while let Ok(Some(sample)) = phone_out_sub.receive() {
                    let mulaw = sample.payload();
                    if mulaw.is_empty() { continue; }
                    let dur = mulaw.len() as f32 / 8000.0;
                    eprintln!("[WEB] phone_out→Twilio: {:.1}s ({} bytes)", dur, mulaw.len());
                    if let Ok(mut ob) = iox_out.lock() {
                        ob.extend(mulaw.iter());
                        iox_mark.store(true, Ordering::Relaxed);
                    }
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
                        r#"<?xml version="1.0" encoding="UTF-8"?><Response><Say voice="Polly.Ruth-Neural">Welcome to Sparked Matter. One moment please.</Say><Connect><Stream url="{}/ws" /></Connect></Response>"#,
                        ws_url
                    );
                    eprintln!("[WEB] Incoming call → {}/ws", ws_url);
                    ([("content-type", "text/xml")], twiml)
                }
            }
        }))
        .route("/ws", get({
            let ca = call_active.clone();
            let sp = speaking.clone();
            let ob = outbound_queue.clone();
            let tx = iox_tx.clone();
            let mp = mark_pending_main.clone();
            move |ws: WebSocketUpgrade| {
                let ca = ca.clone();
                let sp = sp.clone();
                let ob = ob.clone();
                let tx = tx.clone();
                let mp = mp.clone();
                async move {
                    ws.on_upgrade(move |socket| handle_twilio_ws(socket, ca, sp, ob, tx, mp))
                }
            }
        }))
        .route("/twilio-to-browser", post({
            let ws_url = ws_url.clone();
            move || {
                let ws_url = ws_url.clone();
                async move {
                    let twiml = format!(
                        r#"<?xml version="1.0" encoding="UTF-8"?><Response><Say voice="Polly.Ruth-Neural">Patching you through to Airy. One moment.</Say><Connect><Stream url="{}/ws" /></Connect></Response>"#,
                        ws_url
                    );
                    eprintln!("[WEB] Browser bridge call → {}/ws", ws_url);
                    ([("content-type", "text/xml")], twiml)
                }
            }
        }))
        .route("/health", get(|| async { "ok" }));

    eprintln!("[WEB] Listening on port {}", cfg.port);
    eprintln!("[WEB] Webhook: {}/voice (Jarvina)", cfg.server_url);
    eprintln!("[WEB] Webhook: {}/twilio-to-browser (Airy bridge)", cfg.server_url);
    eprintln!("[WEB] READY — waiting for calls");

    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{}", cfg.port)).await?;
    axum::serve(listener, app).await?;
    Ok(())
}

async fn handle_twilio_ws(
    socket: WebSocket,
    call_active: Arc<AtomicBool>,
    speaking: Arc<AtomicBool>,
    outbound_queue: Arc<std::sync::Mutex<VecDeque<u8>>>,
    iox_tx: std::sync::mpsc::Sender<Vec<u8>>,
    mark_pending: Arc<AtomicBool>,
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
    // mark_pending is passed in from main — shared with iox thread

    // Outbound sender — drains mu-law queue → Twilio in 160-byte chunks (20ms at 8kHz)
    let ws_tx_out = Arc::clone(&ws_tx);
    let sid_out = Arc::clone(&stream_sid);
    let ob_send = outbound_queue.clone();
    let ca_send = call_active.clone();
    let mp_send = mark_pending.clone();
    let sp_send = speaking.clone();

    tokio::spawn(async move {
        loop {
            if !ca_send.load(Ordering::Relaxed) { break; }
            let chunk: Option<Vec<u8>> = {
                let mut ob = ob_send.lock().unwrap();
                if ob.len() >= 160 { Some(ob.drain(..160).collect()) } else { None }
            };
            if let Some(chunk) = chunk {
                sp_send.store(true, Ordering::Relaxed);
                let payload = base64::engine::general_purpose::STANDARD.encode(&chunk);
                let sid = sid_out.lock().await.clone();
                if let Some(ref sid) = sid {
                    let msg = json!({"event":"media","streamSid":sid,"media":{"payload":payload}});
                    let mut tx = ws_tx_out.lock().await;
                    let _ = tx.send(Message::Text(msg.to_string())).await;
                }
            } else {
                // Flush any remaining partial chunk before sending mark
                let tail: Option<Vec<u8>> = {
                    let mut ob = ob_send.lock().unwrap();
                    if !ob.is_empty() { Some(ob.drain(..).collect()) } else { None }
                };
                if let Some(tail) = tail {
                    let payload = base64::engine::general_purpose::STANDARD.encode(&tail);
                    let sid = sid_out.lock().await.clone();
                    if let Some(ref sid) = sid {
                        let msg = json!({"event":"media","streamSid":sid,"media":{"payload":payload}});
                        let mut tx = ws_tx_out.lock().await;
                        let _ = tx.send(Message::Text(msg.to_string())).await;
                    }
                }
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

    // Inbound receiver — Twilio → raw mu-law bytes → phone_in bus
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
                        // Pass raw mu-law bytes to bus — digi-ant handles conversion
                        let _ = iox_tx.send(raw_mulaw);
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
