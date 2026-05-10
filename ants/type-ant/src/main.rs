//! Type Ant — Keyboard injection from bus
//!
//! Subscribes to console_text, pastes into focused window via AppleScript.
//! Echo cancellation is handled by patchbay-ant's AEC3 — no hacks here.

use iceoryx2::prelude::*;
use iceoryx2_bb_system_types::path::Path;
use iceoryx2_bb_container::semantic_string::SemanticString;
use std::process::Command;

fn type_text(text: &str) {
    // Copy to clipboard
    let mut child = Command::new("pbcopy")
        .stdin(std::process::Stdio::piped())
        .spawn()
        .expect("pbcopy");
    if let Some(mut stdin) = child.stdin.take() {
        use std::io::Write;
        let _ = stdin.write_all(text.as_bytes());
    }
    let _ = child.wait();

    // Paste via Cmd+V
    let _ = Command::new("osascript")
        .args(["-e", "tell application \"System Events\" to keystroke \"v\" using command down"])
        .output();

    // Brief pause then Enter
    std::thread::sleep(std::time::Duration::from_millis(200));
    let _ = Command::new("osascript")
        .args(["-e", "tell application \"System Events\" to keystroke return"])
        .output();
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    eprintln!("[TYPE] Starting Type Ant...");

    let mut iox = Config::default();
    iox.global.set_root_path(&Path::new(b"/tmp/iceoryx2/").unwrap());
    let node = NodeBuilder::new().config(&iox).create::<ipc::Service>()?;

    let svc = node.service_builder(&"console_text".try_into()?)
        .publish_subscribe::<[u8]>()
        .open_or_create()?;
    let sub = svc.subscriber_builder().create()?;

    eprintln!("[TYPE] Bus: sub='console_text' — READY");
    eprintln!("[TYPE] Types into ANY focused window. AEC3 handles echo.");

    loop {
        while let Some(sample) = sub.receive()? {
            let raw = std::str::from_utf8(sample.payload())
                .unwrap_or("").trim().to_string();
            if raw.is_empty() { continue; }

            // Strip Parakeet hallucination tail:
            let text = if let Some(pos) = raw.find("...") {
                &raw[..pos]
            } else {
                &raw[..]
            };
            let text = text.trim_end_matches(|c: char| c == '.' || !c.is_ascii()).trim();
            let text = {
                let bytes = text.as_bytes();
                let mut upper_run = 0usize;
                let mut cut_at = text.len();
                for (i, &b) in bytes.iter().enumerate() {
                    if b.is_ascii_uppercase() {
                        upper_run += 1;
                        if upper_run >= 5 {
                            cut_at = i + 1 - upper_run;
                            break;
                        }
                    } else {
                        upper_run = 0;
                    }
                }
                text[..cut_at].trim_end_matches(|c: char| !c.is_alphanumeric() && c != '\'' && c != '-').trim()
            };
            if text.is_empty() || text.len() < 2 { continue; }

            eprintln!("[TYPE] Typing: \"{}\"", text.chars().take(60).collect::<String>());
            type_text(&text);
        }
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
}
