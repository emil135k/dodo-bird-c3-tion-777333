//! speaker-ctl-ant — Speaker control daemon + CLI
//!
//! Daemon mode (no args): subscribes to speaker_control, logs commands.
//! CLI mode (with args):  publishes a command to speaker_control.
//!
//! Commands: flush, pause, resume
//! Protocol: single byte on the bus (0x01=flush, 0x02=pause, 0x03=resume)
//! Patchbay-ant subscribes and forwards as negative i32 through pipe to Swift.

use iceoryx2::prelude::*;
use iceoryx2_bb_system_types::path::Path;
use iceoryx2_bb_container::semantic_string::SemanticString;

const FLUSH: u8 = 0x01;
const PAUSE: u8 = 0x02;
const RESUME: u8 = 0x03;

fn command_to_byte(cmd: &str) -> Option<u8> {
    match cmd {
        "flush" | "stop" => Some(FLUSH),
        "pause" => Some(PAUSE),
        "resume" | "play" => Some(RESUME),
        _ => None,
    }
}

fn byte_to_name(b: u8) -> &'static str {
    match b {
        FLUSH => "FLUSH",
        PAUSE => "PAUSE",
        RESUME => "RESUME",
        _ => "UNKNOWN",
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().skip(1).collect();

    let mut iox = Config::default();
    iox.global.set_root_path(&Path::new(b"/tmp/iceoryx2/").unwrap());
    let node = NodeBuilder::new().config(&iox).create::<ipc::Service>()?;

    let svc = node.service_builder(&"speaker_control".try_into()?)
        .publish_subscribe::<[u8]>().open_or_create()?;

    if args.is_empty() {
        // Daemon mode — subscribe and log
        eprintln!("[SPEAKER-CTL] Daemon mode — subscribing to speaker_control");
        let sub = svc.subscriber_builder().create()?;
        eprintln!("[SPEAKER-CTL] READY");

        loop {
            while let Some(sample) = sub.receive()? {
                let payload = sample.payload();
                if let Some(&cmd) = payload.first() {
                    eprintln!("[SPEAKER-CTL] Observed: {} (0x{:02x})", byte_to_name(cmd), cmd);
                }
            }
            std::thread::sleep(std::time::Duration::from_millis(50));
        }
    } else {
        // CLI mode — publish command
        let cmd_str = args[0].to_lowercase();
        let cmd_byte = command_to_byte(&cmd_str)
            .ok_or_else(|| format!("Unknown command '{}'. Use: flush, pause, resume", cmd_str))?;

        let pub_ = svc.publisher_builder().initial_max_slice_len(1).create()?;
        let sample = pub_.loan_slice_uninit(1)?;
        sample.write_from_slice(&[cmd_byte]).send()?;
        eprintln!("[SPEAKER-CTL] Published: {} (0x{:02x})", byte_to_name(cmd_byte), cmd_byte);

        // Keep alive briefly so subscriber can receive
        std::thread::sleep(std::time::Duration::from_millis(500));
    }

    Ok(())
}
