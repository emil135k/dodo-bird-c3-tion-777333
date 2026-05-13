//! Patchbay Ant — Audio I/O via Swift Worker with Apple AEC
//!
//! Rust handles iceoryx2 bus. Swift handles audio (AVAudioEngine + Voice Processing).
//! Connected by anonymous Unix pipes. Same pattern as stt-ant.
//!
//! Data flow:
//!   tts_audio bus → Rust → pipe(stdin) → Swift(AVAudioEngine plays + AEC)
//!   Swift(mic, echo-cancelled) → pipe(stdout) → Rust → stt_raw bus

use iceoryx2::prelude::*;
use iceoryx2_bb_system_types::path::Path;
use iceoryx2_bb_container::semantic_string::SemanticString;

use std::process::{Command, Stdio};
use std::io::{Write, BufReader, BufRead, Read};

const WORKER_BIN: &str = "/Users/rocketman/.local/bin/patchbay-worker";

fn main() -> Result<(), Box<dyn std::error::Error>> {
    eprintln!("[PATCHBAY] Starting — Swift worker with Apple AEC...");

    // Spawn Swift worker
    let mut child = Command::new(WORKER_BIN)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .spawn()
        .expect("Failed to spawn patchbay-worker");

    let mut worker_stdin = child.stdin.take().expect("stdin");
    let worker_stdout = child.stdout.take().expect("stdout");
    let mut reader = BufReader::new(worker_stdout);

    eprintln!("[PATCHBAY] Swift worker spawned (PID {})", child.id());

    // Readiness handshake
    eprintln!("[PATCHBAY] Waiting for Swift worker...");
    {
        let mut ready_line = String::new();
        match reader.read_line(&mut ready_line) {
            Ok(0) => {
                eprintln!("[PATCHBAY] FATAL: worker closed stdout before ready");
                return Err("worker died during init".into());
            }
            Ok(_) => {
                if ready_line.trim() == "<ready>" {
                    eprintln!("[PATCHBAY] Swift worker READY — AVAudioEngine + AEC");
                } else {
                    eprintln!("[PATCHBAY] FATAL: bad handshake: {:?}", ready_line.trim());
                    return Err("bad handshake".into());
                }
            }
            Err(e) => return Err(format!("handshake error: {}", e).into()),
        }
    }

    // iceoryx2 — ALL ants use explicit /tmp/iceoryx2/ path
    let mut iox = Config::default();
    iox.global.set_root_path(&Path::new(b"/tmp/iceoryx2/").unwrap());
    let node = NodeBuilder::new().config(&iox).create::<ipc::Service>()?;

    // Publish mic audio (echo-cancelled by Apple) to stt_raw
    let raw_svc = node.service_builder(&"stt_raw".try_into()?)
        .publish_subscribe::<[u8]>().open_or_create()?;
    let mic_pub = raw_svc.publisher_builder()
        .initial_max_slice_len(4 * 1024 * 1024).create()?;

    // Subscribe to TTS audio to play through speaker
    let audio_svc = node.service_builder(&"tts_audio".try_into()?)
        .publish_subscribe::<[u8]>().open_or_create()?;
    let spk_sub = audio_svc.subscriber_builder().create()?;

    // Subscribe to speaker control commands (flush/pause/resume)
    let ctl_svc = node.service_builder(&"speaker_control".try_into()?)
        .publish_subscribe::<[u8]>().open_or_create()?;
    let ctl_sub = ctl_svc.subscriber_builder().create()?;

    eprintln!("[PATCHBAY] Bus: stt_raw <-> tts_audio + speaker_control — READY");

    // Reader thread: Swift worker stdout → mic audio → stt_raw bus
    // Protocol: [i32 sample_count LE][f32 samples at 16kHz]
    let (mic_tx, mic_rx) = std::sync::mpsc::channel::<Vec<f32>>();

    std::thread::spawn(move || {
        eprintln!("[PATCHBAY] Reader thread started, attempting first read...");
        let mut frame_count: u64 = 0;
        loop {
            // Read sample count (4 bytes LE i32)
            let mut count_buf = [0u8; 4];
            if let Err(e) = reader.read_exact(&mut count_buf) {
                eprintln!("[PATCHBAY] Reader error: {}", e);
                break;
            }
            frame_count += 1;
            let sample_count = i32::from_le_bytes(count_buf);
            if frame_count <= 3 || frame_count % 500 == 0 {
                eprintln!("[PATCHBAY] Reader frame #{}: {} samples", frame_count, sample_count);
            }
            if sample_count <= 0 { continue; }
            if sample_count > 4800000 {
                eprintln!("[PATCHBAY] FATAL: sample count {} too large", sample_count);
                break;
            }

            // Read f32 samples
            let byte_count = sample_count as usize * 4;
            let mut audio_data = vec![0u8; byte_count];
            if reader.read_exact(&mut audio_data).is_err() {
                eprintln!("[PATCHBAY] Worker stdout read error");
                break;
            }

            let samples: Vec<f32> = audio_data.chunks_exact(4)
                .map(|c| f32::from_le_bytes([c[0], c[1], c[2], c[3]]))
                .collect();

            let _ = mic_tx.send(samples);
        }
    });

    // Main loop
    loop {
        // Check if worker is alive
        match child.try_wait() {
            Ok(Some(status)) => {
                eprintln!("[PATCHBAY] Worker exited: {}", status);
                return Err(format!("worker died: {}", status).into());
            }
            Ok(None) => {}
            Err(e) => eprintln!("[PATCHBAY] Worker check error: {}", e),
        }

        // 1. Check speaker control FIRST — flush must happen before forwarding audio
        let mut flushed = false;
        while let Some(sample) = ctl_sub.receive()? {
            let payload = sample.payload();
            if let Some(&cmd) = payload.first() {
                let code: i32 = -(cmd as i32);
                let names = ["-", "FLUSH", "PAUSE", "RESUME"];
                let name = names.get(cmd as usize).unwrap_or(&"UNKNOWN");
                eprintln!("[PATCHBAY] Speaker control: {} → pipe ({})", name, code);
                if let Err(e) = worker_stdin.write_all(&code.to_le_bytes()) {
                    eprintln!("[PATCHBAY] FATAL: pipe write error on control: {}", e);
                    return Err(format!("pipe broken: {}", e).into());
                }
                if let Err(e) = worker_stdin.flush() {
                    eprintln!("[PATCHBAY] FATAL: pipe flush error on control: {}", e);
                    return Err(format!("pipe broken: {}", e).into());
                }
                if cmd == 0x01 { flushed = true; }
            }
        }

        // If flushed, drain all pending tts_audio — it's stale
        if flushed {
            let mut drained = 0;
            while let Some(_) = spk_sub.receive()? { drained += 1; }
            if drained > 0 {
                eprintln!("[PATCHBAY] Drained {} stale audio messages after FLUSH", drained);
            }
        }

        // 2. Receive TTS audio from bus, forward to Swift worker for playback
        while let Some(sample) = spk_sub.receive()? {
            // Check for flush BETWEEN each audio message
            while let Some(ctl) = ctl_sub.receive()? {
                if let Some(&cmd) = ctl.payload().first() {
                    if cmd == 0x01 {
                        let flush_code: i32 = -1;
                        let _ = worker_stdin.write_all(&flush_code.to_le_bytes());
                        let _ = worker_stdin.flush();
                        eprintln!("[PATCHBAY] FLUSH mid-forward → draining remaining audio");
                        // Drain all remaining audio messages
                        while let Some(_) = spk_sub.receive()? {}
                        break;
                    }
                    let code: i32 = -(cmd as i32);
                    let _ = worker_stdin.write_all(&code.to_le_bytes());
                    let _ = worker_stdin.flush();
                }
            }

            let payload = sample.payload();
            if payload.len() < 4 { continue; }

            let sample_count = (payload.len() / 4) as i32;

            if let Err(e) = worker_stdin.write_all(&sample_count.to_le_bytes()) {
                eprintln!("[PATCHBAY] FATAL: pipe write error: {}", e);
                return Err(format!("pipe broken: {}", e).into());
            }
            let boosted: Vec<u8> = payload.chunks_exact(4)
                .map(|c| f32::from_le_bytes([c[0], c[1], c[2], c[3]]))
                .map(|s| (s * 2.0).clamp(-1.0, 1.0))
                .flat_map(|s| s.to_le_bytes())
                .collect();
            if let Err(e) = worker_stdin.write_all(&boosted) {
                eprintln!("[PATCHBAY] FATAL: pipe write error: {}", e);
                return Err(format!("pipe broken: {}", e).into());
            }
            if let Err(e) = worker_stdin.flush() {
                eprintln!("[PATCHBAY] FATAL: pipe flush error: {}", e);
                return Err(format!("pipe broken: {}", e).into());
            }

            let dur = sample_count as f32 / 24000.0;
            eprintln!("[PATCHBAY] → speaker: {:.1}s ({} samples)", dur, sample_count);
        }

        // 3. Receive echo-cancelled mic audio from Swift worker, publish to stt_raw
        while let Ok(samples) = mic_rx.try_recv() {
            if samples.is_empty() { continue; }

            // Mic comes at 48kHz from Swift, stt_raw contract is 48kHz — publish as-is
            let bytes: Vec<u8> = samples.iter().flat_map(|s| s.to_le_bytes()).collect();
            if let Ok(loan) = mic_pub.loan_slice_uninit(bytes.len()) {
                let _ = loan.write_from_slice(&bytes).send();
            }
        }

        std::thread::sleep(std::time::Duration::from_millis(5));
    }
}
