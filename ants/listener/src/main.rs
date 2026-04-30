//! Listener — prints what the swarm heard
//!
//! Subscribes to stt_text bus. Prints every transcription.
//! The reverse of Pulse.
//!
//! Usage: listener
//!        (then speak into your mic — text appears in real time)

use iceoryx2::prelude::*;
use iceoryx2_bb_system_types::path::Path;
use iceoryx2_bb_container::semantic_string::SemanticString;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    eprintln!("[LISTENER] Subscribing to 'stt_text' — waiting for transcriptions...");

    let mut config = Config::default();
    config.global.set_root_path(&Path::new(b"/tmp/iceoryx2/").unwrap());
    let node = NodeBuilder::new().config(&config).create::<ipc::Service>()?;

    let svc = node.service_builder(&"stt_text".try_into()?)
        .publish_subscribe::<[u8]>()
        .open()?;

    let sub = svc.subscriber_builder().create()?;

    loop {
        while let Some(sample) = sub.receive()? {
            let text = std::str::from_utf8(sample.payload())
                .unwrap_or("<invalid utf8>")
                .trim();

            if !text.is_empty() {
                println!("[HEARD] {}", text);
            }
        }
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
}
