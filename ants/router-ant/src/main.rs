//! Router Ant — Routes stt_text to console or LLM, manages audio bridge to Airy
//!
//! Subscribes to stt_text, republishes based on current mode.
//! Mode switched via HTTP: curl localhost:3010/mode/console
//!
//! Modes:
//!   console → publishes to console_text (type-ant picks up)
//!   llm     → publishes to llm_input (llm-ant picks up)
//!   airy    → starts audio bridge: Blackwire mic → BlackHole 2ch → Chrome/Airy voice
//!   off     → drops all text (mute)

use iceoryx2::prelude::*;
use iceoryx2_bb_system_types::path::Path;
use iceoryx2_bb_container::semantic_string::SemanticString;
use std::sync::{Arc, Mutex, atomic::{AtomicU8, Ordering}};
use std::process::{Child, Command};

const MODE_OFF: u8 = 0;
const MODE_CONSOLE: u8 = 1;
const MODE_LLM: u8 = 2;
const MODE_AIRY: u8 = 3;

fn mode_name(m: u8) -> &'static str {
    match m {
        MODE_OFF => "off",
        MODE_CONSOLE => "console",
        MODE_LLM => "llm",
        MODE_AIRY => "airy",
        _ => "unknown",
    }
}

/// Start audio bridge: Blackwire mic → BlackHole 2ch (Chrome picks up as mic)
fn start_audio_bridge() -> Option<Child> {
    eprintln!("[ROUTER] Starting audio bridge: Blackwire → BlackHole 2ch");
    match Command::new("sox")
        .args([
            "-t", "coreaudio", "Plantronics Blackwire 3210 Series",  // input: Blackwire mic
            "-t", "coreaudio", "BlackHole 2ch",                       // output: BlackHole (Chrome's mic)
        ])
        .spawn()
    {
        Ok(child) => {
            eprintln!("[ROUTER] Audio bridge started (PID {})", child.id());
            Some(child)
        }
        Err(e) => {
            eprintln!("[ROUTER] Audio bridge FAILED: {}", e);
            None
        }
    }
}

/// Stop audio bridge
fn stop_audio_bridge(bridge: &mut Option<Child>) {
    if let Some(ref mut child) = bridge {
        eprintln!("[ROUTER] Stopping audio bridge (PID {})", child.id());
        let _ = child.kill();
        let _ = child.wait();
    }
    *bridge = None;
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    eprintln!("[ROUTER] Starting Router Ant...");

    let mode = Arc::new(AtomicU8::new(MODE_CONSOLE));
    let bridge: Arc<Mutex<Option<Child>>> = Arc::new(Mutex::new(None));

    // HTTP server for mode switching
    let mode_http = mode.clone();
    let bridge_http = bridge.clone();
    tokio::spawn(async move {
        use axum::{routing::get, Router, extract::Path as AxumPath};

        let mode_get = mode_http.clone();
        let mode_set = mode_http.clone();
        let bridge_set = bridge_http.clone();

        let app = Router::new()
            .route("/status", get({
                let m = mode_get.clone();
                move || async move {
                    format!("{{\"mode\":\"{}\"}}", mode_name(m.load(Ordering::Relaxed)))
                }
            }))
            .route("/mode/:new_mode", get({
                let m = mode_set.clone();
                let b = bridge_set.clone();
                move |AxumPath(new_mode): AxumPath<String>| async move {
                    let val = match new_mode.as_str() {
                        "console" => MODE_CONSOLE,
                        "llm" => MODE_LLM,
                        "airy" => MODE_AIRY,
                        "off" => MODE_OFF,
                        _ => return format!("{{\"error\":\"unknown mode: {}\"}}", new_mode),
                    };

                    let old_mode = m.swap(val, Ordering::Relaxed);

                    // Manage audio bridge lifecycle
                    if let Ok(mut bridge) = b.lock() {
                        if old_mode == MODE_AIRY && val != MODE_AIRY {
                            stop_audio_bridge(&mut bridge);
                        }
                        if val == MODE_AIRY && old_mode != MODE_AIRY {
                            *bridge = start_audio_bridge();
                        }
                    }

                    eprintln!("[ROUTER] Mode → {}", new_mode);
                    format!("{{\"mode\":\"{}\"}}", new_mode)
                }
            }));

        let listener = tokio::net::TcpListener::bind("0.0.0.0:3010").await.unwrap();
        eprintln!("[ROUTER] HTTP: http://localhost:3010/mode/console|llm|airy|off");
        axum::serve(listener, app).await.unwrap();
    });

    // iceoryx2 on a std thread (publishers are !Send)
    let mode_iox = mode.clone();
    std::thread::spawn(move || {
        let mut iox = Config::default();
        iox.global.set_root_path(&Path::new(b"/tmp/iceoryx2/").unwrap());
        let node = NodeBuilder::new().config(&iox).create::<ipc::Service>()
            .expect("iceoryx2 node");

        // Subscribe to stt_text
        let stt_svc = node.service_builder(&"stt_text".try_into().unwrap())
            .publish_subscribe::<[u8]>()
            .open_or_create().expect("stt_text service");
        let sub = stt_svc.subscriber_builder().create().expect("stt_text subscriber");

        // Publish to console_text
        let console_svc = node.service_builder(&"console_text".try_into().unwrap())
            .publish_subscribe::<[u8]>()
            .open_or_create().expect("console_text service");
        let console_pub = console_svc.publisher_builder()
            .initial_max_slice_len(8192)
            .create().expect("console_text publisher");

        // Publish to llm_input
        let llm_svc = node.service_builder(&"llm_input".try_into().unwrap())
            .publish_subscribe::<[u8]>()
            .open_or_create().expect("llm_input service");
        let llm_pub = llm_svc.publisher_builder()
            .initial_max_slice_len(8192)
            .create().expect("llm_input publisher");

        // Publish to airy_input (cdp-ant picks up, injects into browser)
        let airy_svc = node.service_builder(&"airy_input".try_into().unwrap())
            .publish_subscribe::<[u8]>()
            .open_or_create().expect("airy_input service");
        let airy_pub = airy_svc.publisher_builder()
            .initial_max_slice_len(8192)
            .create().expect("airy_input publisher");

        eprintln!("[ROUTER] Bus: sub='stt_text' pub='console_text','llm_input','airy_input' — READY");
        eprintln!("[ROUTER] Mode: {}", mode_name(mode_iox.load(Ordering::Relaxed)));

        loop {
            while let Ok(Some(sample)) = sub.receive() {
                let payload = sample.payload();
                if payload.is_empty() { continue; }

                let current_mode = mode_iox.load(Ordering::Relaxed);

                let text_preview: String = std::str::from_utf8(payload)
                    .unwrap_or("?").chars().take(50).collect();

                match current_mode {
                    MODE_CONSOLE => {
                        eprintln!("[ROUTER] → console: \"{}\"", text_preview);
                        if let Ok(loan) = console_pub.loan_slice_uninit(payload.len()) {
                            let _ = loan.write_from_slice(payload).send();
                        }
                    }
                    MODE_LLM => {
                        eprintln!("[ROUTER] → llm: \"{}\"", text_preview);
                        if let Ok(loan) = llm_pub.loan_slice_uninit(payload.len()) {
                            let _ = loan.write_from_slice(payload).send();
                        }
                    }
                    MODE_AIRY => {
                        // Audio bridge handles voice directly — stt_text is dropped
                        eprintln!("[ROUTER] → airy (voice bridge active, text dropped): \"{}\"", text_preview);
                    }
                    MODE_OFF => {
                        eprintln!("[ROUTER] → /dev/null: \"{}\"", text_preview);
                    }
                    _ => {}
                }
            }
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
    });

    // Keep main alive
    loop {
        tokio::time::sleep(std::time::Duration::from_secs(3600)).await;
    }
}
