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
//!
//! Contract: stt_audio payloads are complete VAD-segmented utterances
//! from phone-silero-ant. Each payload is one utterance, not a stream chunk.

use iceoryx2::prelude::*;
use iceoryx2_bb_system_types::path::Path;
use iceoryx2_bb_container::semantic_string::SemanticString;

use std::process::{Command, Stdio};
use std::io::{Write, BufReader, BufRead};

const WORKER_BIN: &str = "/Users/rocketman/.local/bin/parakeet-worker";
const SAMPLE_RATE: u32 = 16000;

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

    // iceoryx2
    let mut config = Config::default();
    config.global.set_root_path(&Path::new(b"/tmp/iceoryx2/").unwrap());
    let node = NodeBuilder::new().config(&config).create::<ipc::Service>()?;

    // Subscribe to audio input (byte-packed f32 samples at 16kHz from phone-silero-ant)
    let audio_svc = node.service_builder(&"stt_audio".try_into()?)
        .publish_subscribe::<[u8]>()
        .open_or_create()?;
    let sub = audio_svc.subscriber_builder().create()?;

    // Publish transcribed text
    // Contract: stt_text contains ONLY recognized speech text (UTF-8).
    // Empty transcriptions and errors are logged but NOT published to stt_text.
    // Downstream ants should not assume 1:1 correspondence with stt_audio utterances.
    // Future: structured payload with utterance ID, status, and text.
    let text_svc = node.service_builder(&"stt_text".try_into()?)
        .publish_subscribe::<[u8]>()
        .open_or_create()?;
    let pub_ = text_svc.publisher_builder()
        .initial_max_slice_len(4096)
        .create()?;

    eprintln!("[STT-ANT] Bus: sub='stt_audio' pub='stt_text' — READY");

    // Reader thread: Swift worker stdout → mpsc channel
    let (tx, rx) = std::sync::mpsc::channel::<String>();

    std::thread::spawn(move || {
        let mut line = String::new();
        loop {
            line.clear();
            match reader.read_line(&mut line) {
                Ok(0) => {
                    eprintln!("[STT-ANT] Swift worker stdout closed");
                    break;
                }
                Ok(_) => {
                    let text = line.trim().to_string();
                    if text.is_empty() { continue; }
                    if text == "<empty>" {
                        eprintln!("[STT-ANT] Empty transcription (VAD utterance had no recognizable speech)");
                        continue;
                    }
                    if text == "<error>" {
                        eprintln!("[STT-ANT] Worker reported transcription error");
                        continue;
                    }
                    let _ = tx.send(text);
                }
                Err(e) => {
                    eprintln!("[STT-ANT] Swift worker read error: {}", e);
                    break;
                }
            }
        }
    });

    // Main loop
    loop {
        // Check if Swift worker is still alive
        match child.try_wait() {
            Ok(Some(status)) => {
                eprintln!("[STT-ANT] Swift worker exited with: {}", status);
                return Err(format!("parakeet-worker died: {}", status).into());
            }
            Ok(None) => {} // still running
            Err(e) => eprintln!("[STT-ANT] Worker status check error: {}", e),
        }

        // Forward audio from bus to Swift worker
        while let Some(sample) = sub.receive()? {
            let payload = sample.payload();

            // Alignment check: payload must be divisible by 4 (f32 samples)
            if payload.len() % 4 != 0 {
                eprintln!("[STT-ANT] WARN: payload {} bytes not aligned to 4, skipping", payload.len());
                continue;
            }

            let sample_count = (payload.len() / 4) as i32;
            if sample_count <= 0 { continue; }

            let duration_s = sample_count as f64 / SAMPLE_RATE as f64;
            eprintln!("[STT-ANT] Forwarding {:.1}s audio ({} samples)", duration_s, sample_count);

            // Note: flush() is intentional — Swift worker reads stdin in blocking mode.
            // Without flush, short utterances may stall in the pipe buffer.
            let write_start = std::time::Instant::now();
            if let Err(e) = worker_stdin.write_all(&sample_count.to_le_bytes()) {
                eprintln!("[STT-ANT] FATAL: Worker stdin write error: {}", e);
                return Err(format!("pipe broken: {}", e).into());
            }
            if let Err(e) = worker_stdin.write_all(payload) {
                eprintln!("[STT-ANT] FATAL: Worker stdin write error: {}", e);
                return Err(format!("pipe broken: {}", e).into());
            }
            if let Err(e) = worker_stdin.flush() {
                eprintln!("[STT-ANT] FATAL: Worker stdin flush error: {}", e);
                return Err(format!("pipe broken: {}", e).into());
            }
            let write_ms = write_start.elapsed().as_millis();
            if write_ms > 100 {
                eprintln!("[STT-ANT] WARN: pipe write took {}ms — worker may be backpressured", write_ms);
            }
        }

        // Publish transcriptions from Swift worker
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
