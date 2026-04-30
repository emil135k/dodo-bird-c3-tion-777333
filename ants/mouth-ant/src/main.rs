//! Audio Sink Ant — The Ear
//!
//! Subscribes to hypai/audio iceoryx2 bus.
//! Converts raw bytes to f32 samples.
//! Plays through rodio to Mac speakers.
//! One ant, one job. Zero disk.

use iceoryx2::prelude::*;
use iceoryx2_bb_system_types::path::Path;
use iceoryx2_bb_container::semantic_string::SemanticString;
use rodio::{OutputStream, Sink, buffer::SamplesBuffer};

const AUDIO_SERVICE: &str = "tts_audio";
const SAMPLE_RATE: u32 = 24000;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    eprintln!("[MOUTH] Starting Audio Sink Ant...");

    // Initialize audio output — keep stream alive for daemon lifetime
    let (_stream, stream_handle) = OutputStream::try_default()
        .expect("Failed to open audio output");
    let sink = Sink::try_new(&stream_handle)
        .expect("Failed to create audio sink");

    eprintln!("[MOUTH] Audio output: READY");

    // Create iceoryx2 node and subscribe to audio bus
    let mut config = Config::default();
    config.global.set_root_path(&Path::new(b"/tmp/iceoryx2/").unwrap());
    let node = NodeBuilder::new().config(&config).create::<ipc::Service>()?;

    let audio_service = node.service_builder(&AUDIO_SERVICE.try_into()?)
        .publish_subscribe::<[u8]>()
        .open()?;

    let subscriber = audio_service.subscriber_builder().create()?;

    eprintln!("[MOUTH] Subscribed to '{}' — waiting for audio...", AUDIO_SERVICE);

    // Daemon loop — receive f32 samples, play immediately
    loop {
        while let Some(sample) = subscriber.receive()? {
            let raw_bytes = sample.payload();

            // Convert raw bytes back to f32 samples
            let samples: Vec<f32> = raw_bytes.chunks(4)
                .map(|c| f32::from_le_bytes([c[0], c[1], c[2], c[3]]))
                .collect();

            if samples.is_empty() { continue; }

            let duration = samples.len() as f64 / SAMPLE_RATE as f64;
            eprintln!("[MOUTH] Playing {:.1}s audio ({} samples)", duration, samples.len());

            // Hand to rodio — zero disk, direct to speaker
            let buffer = SamplesBuffer::new(1, SAMPLE_RATE, samples);
            sink.append(buffer);
            // Don't sleep_until_end — let TTS ant pipeline ahead
        }

        // Yield to OS — don't spin
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
}
