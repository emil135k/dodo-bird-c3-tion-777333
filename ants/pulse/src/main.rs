//! Pulse — send and self-verify via iceoryx2

use iceoryx2::prelude::*;
use iceoryx2_bb_system_types::path::Path;
use iceoryx2_bb_container::semantic_string::SemanticString;
use std::env;

const SERVICE_NAME: &str = "tts_text";

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: pulse \"text\"");
        std::process::exit(1);
    }
    let text = args[1..].join(" ");

    let mut config = Config::default();
    config.global.set_root_path(&Path::new(b"/tmp/iceoryx2/").unwrap());
    let node = NodeBuilder::new().config(&config).create::<ipc::Service>()?;

    let service = node.service_builder(&SERVICE_NAME.try_into()?)
        .publish_subscribe::<[u8]>()
        .open()?;

    // Create BOTH publisher and subscriber to self-verify
    let publisher = service.publisher_builder()
        .initial_max_slice_len(4096)
        .create()?;
    let subscriber = service.subscriber_builder().create()?;

    let bytes = text.as_bytes();
    let sample = publisher.loan_slice_uninit(bytes.len())?;
    let sample = sample.write_from_slice(bytes);
    sample.send()?;
    eprintln!("[PULSE] Sent {} bytes to '{}'", bytes.len(), SERVICE_NAME);

    // Self-verify
    std::thread::sleep(std::time::Duration::from_millis(100));
    match subscriber.receive()? {
        Some(s) => eprintln!("[PULSE] SELF-VERIFY: received {} bytes", s.payload().len()),
        None => eprintln!("[PULSE] SELF-VERIFY: NO DATA — bus is broken"),
    }

    Ok(())
}
