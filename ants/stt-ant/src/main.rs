//! STT Ant — Bus Adapter for Parakeet CoreML Swift Worker
//!
//! Rust handles iceoryx2 bus. Swift handles CoreML ANE.
//! Connected by anonymous Unix pipes. Zero disk. Zero FFI.
//!
//! Data flow:
//!   iceoryx2(stt_audio) → Rust → pipe(stdin) → Swift(Parakeet ANE)
//!   Swift(stdout) → pipe → Rust → iceoryx2(stt_text)
//!
//! Protocol:
//!   stdin:  [i32 sample_count LE] [f32 samples LE...]
//!   stdout: UTF-8 text line per transcription

use iceoryx2::prelude::*;
use iceoryx2_bb_system_types::path::Path;
use iceoryx2_bb_container::semantic_string::SemanticString;

use std::process::{Command, Stdio};
use std::io::{Write, BufReader, BufRead};

// Path to the Swift worker binary
const WORKER_BIN: &str = "/Users/rocketman/.local/bin/parakeet-worker";

fn main() -> Result<(), Box<dyn std::error::Error>> {
    eprintln!("[STT-ANT] Starting Bus Adapter...");

    // Spawn Swift worker with piped I/O
    let mut child = Command::new(WORKER_BIN)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit()) // Swift logs visible
        .spawn()
        .expect("Failed to spawn parakeet-worker");

    let mut worker_stdin = child.stdin.take().expect("stdin");
    let worker_stdout = child.stdout.take().expect("stdout");
    let mut reader = BufReader::new(worker_stdout);

    eprintln!("[STT-ANT] Swift Worker spawned (PID {})", child.id());

    // iceoryx2 with hardcoded config
    let mut config = Config::default();
    config.global.set_root_path(&Path::new(b"/tmp/iceoryx2/").unwrap());
    let node = NodeBuilder::new().config(&config).create::<ipc::Service>()?;

    // Subscribe to audio input (f32 samples at 16kHz)
    let audio_svc = node.service_builder(&"stt_audio".try_into()?)
        .publish_subscribe::<[u8]>()
        .subscriber_max_buffer_size(64)
        .history_size(16)
        .open_or_create()?;
    let sub = audio_svc.subscriber_builder().create()?;

    // Publish transcribed text
    let text_svc = node.service_builder(&"stt_text".try_into()?)
        .publish_subscribe::<[u8]>()
        .open_or_create()?;
    let pub_ = text_svc.publisher_builder()
        .initial_max_slice_len(4096)
        .create()?;

    eprintln!("[STT-ANT] Bus: sub='stt_audio' pub='stt_text' — READY");

    // Use channels to pass text from reader thread to main thread for publishing
    let (tx, rx) = std::sync::mpsc::channel::<String>();

    // Spawn stdout reader thread
    std::thread::spawn(move || {
        let mut line = String::new();
        loop {
            line.clear();
            match reader.read_line(&mut line) {
                Ok(0) => break,
                Ok(_) => {
                    let text = line.trim().to_string();
                    if !text.is_empty() {
                        let _ = tx.send(text);
                    }
                }
                Err(_) => break,
            }
        }
    });

    // Main loop — forward audio AND publish transcriptions
    loop {
        // Forward audio from bus to Swift worker
        while let Some(sample) = sub.receive()? {
            let payload = sample.payload();
            let sample_count = (payload.len() / 4) as i32;
            if sample_count <= 0 { continue; }

            eprintln!("[STT-ANT] Forwarding {:.1}s audio", sample_count as f64 / 16000.0);
            worker_stdin.write_all(&sample_count.to_le_bytes())?;
            worker_stdin.write_all(payload)?;
            worker_stdin.flush()?;
        }

        // Publish any transcriptions from Swift worker
        while let Ok(text) = rx.try_recv() {
            eprintln!("[STT-ANT] Transcribed: \"{}\"", text);
            let bytes = text.as_bytes();
            let sample = pub_.loan_slice_uninit(bytes.len())?;
            let sample = sample.write_from_slice(bytes);
            sample.send()?;
            eprintln!("[STT-ANT] Published: {} bytes", bytes.len());
        }

        std::thread::sleep(std::time::Duration::from_millis(10));
    }
}
