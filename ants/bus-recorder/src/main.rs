//! Bus Recorder — iceoryx2 debugging tool
//!
//! Records raw bus data from any iceoryx2 service for offline analysis.
//!
//! Usage:
//!   bus-recorder phone_in 20              # Record 20 messages from phone_in
//!   bus-recorder phone_stt 10 f32         # Record 10 messages from phone_stt (f32 type)
//!   bus-recorder phone_in phone_stt 20    # Record both buses concurrently

use iceoryx2::prelude::*;
use iceoryx2_bb_system_types::path::Path;
use iceoryx2_bb_container::semantic_string::SemanticString;
use std::time::Instant;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();

    if args.len() < 3 {
        eprintln!("Usage: bus-recorder <bus_name> <max_messages> [u8|f32]");
        eprintln!("       bus-recorder <bus1> <bus2> <max_messages>  (concurrent)");
        return Ok(());
    }

    // Check if concurrent mode (two bus names)
    let is_concurrent = args.len() >= 4 && args[3].parse::<usize>().is_ok();

    if is_concurrent {
        let bus1 = &args[1];
        let bus2 = &args[2];
        let max_msgs: usize = args[3].parse()?;
        record_concurrent(bus1, bus2, max_msgs)?;
    } else {
        let bus_name = &args[1];
        let max_msgs: usize = args[2].parse()?;
        let bus_type = args.get(3).map(|s| s.as_str()).unwrap_or("u8");
        record_single(bus_name, max_msgs, bus_type)?;
    }

    Ok(())
}

fn record_single(bus_name: &str, max_msgs: usize, bus_type: &str) -> Result<(), Box<dyn std::error::Error>> {
    eprintln!("[REC] Recording '{}' (type={}) — {} messages max", bus_name, bus_type, max_msgs);

    let mut iox = Config::default();
    iox.global.set_root_path(&Path::new(b"/tmp/iceoryx2/").unwrap());
    let node = NodeBuilder::new().config(&iox).create::<ipc::Service>()?;

    let start = Instant::now();
    let mut count = 0usize;
    let mut total_bytes = 0usize;

    let outfile = format!("/tmp/rec-{}-{}.csv", bus_name, chrono_stamp());
    let mut out = std::fs::File::create(&outfile)?;
    use std::io::Write;
    writeln!(out, "msg,elapsed_ms,payload_len,sample_count,peak,rms")?;

    if bus_type == "f32" {
        let svc = node.service_builder(&bus_name.try_into()?)
            .publish_subscribe::<[f32]>().open_or_create()?;
        let sub = svc.subscriber_builder().create()?;

        while count < max_msgs {
            while let Some(sample) = sub.receive()? {
                let p = sample.payload();
                let elapsed = start.elapsed().as_millis();
                let peak = p.iter().map(|s| s.abs()).fold(0.0f32, f32::max);
                let rms = (p.iter().map(|s| s * s).sum::<f32>() / p.len() as f32).sqrt();
                total_bytes += p.len() * 4;
                count += 1;

                writeln!(out, "{},{},{},{},{:.6},{:.6}", count, elapsed, p.len() * 4, p.len(), peak, rms)?;
                eprintln!("[REC] #{} t={}ms len={} peak={:.4} rms={:.4}", count, elapsed, p.len(), peak, rms);

                if count >= max_msgs { break; }
            }
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
    } else {
        let svc = node.service_builder(&bus_name.try_into()?)
            .publish_subscribe::<[u8]>().open_or_create()?;
        let sub = svc.subscriber_builder().create()?;

        while count < max_msgs {
            while let Some(sample) = sub.receive()? {
                let p = sample.payload();
                let elapsed = start.elapsed().as_millis();
                total_bytes += p.len();
                count += 1;

                // For u8 (mu-law), show byte stats
                let min_val = p.iter().copied().min().unwrap_or(0);
                let max_val = p.iter().copied().max().unwrap_or(0);
                writeln!(out, "{},{},{},{},{},{}", count, elapsed, p.len(), p.len(), min_val, max_val)?;
                eprintln!("[REC] #{} t={}ms len={} min=0x{:02X} max=0x{:02X}", count, elapsed, p.len(), min_val, max_val);

                if count >= max_msgs { break; }
            }
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
    }

    let elapsed = start.elapsed().as_secs_f32();
    eprintln!("[REC] DONE: {} msgs, {} bytes in {:.1}s → {}", count, total_bytes, elapsed, outfile);
    Ok(())
}

fn record_concurrent(bus1: &str, bus2: &str, max_msgs: usize) -> Result<(), Box<dyn std::error::Error>> {
    eprintln!("[REC] Concurrent recording: '{}' + '{}' — {} messages each", bus1, bus2, max_msgs);

    let stamp = chrono_stamp();
    let outfile1 = format!("/tmp/rec-{}-{}.csv", bus1, stamp);
    let outfile2 = format!("/tmp/rec-{}-{}.csv", bus2, stamp);

    let b1 = bus1.to_string();
    let b2 = bus2.to_string();
    let of1 = outfile1.clone();
    let of2 = outfile2.clone();

    let h1 = std::thread::spawn(move || {
        record_to_file(&b1, max_msgs, "u8", &of1).ok();
    });

    let h2 = std::thread::spawn(move || {
        record_to_file(&b2, max_msgs, "f32", &of2).ok();
    });

    h1.join().ok();
    h2.join().ok();

    eprintln!("[REC] Files: {} + {}", outfile1, outfile2);
    Ok(())
}

fn record_to_file(bus_name: &str, max_msgs: usize, bus_type: &str, outfile: &str) -> Result<(), Box<dyn std::error::Error>> {
    let mut iox = Config::default();
    iox.global.set_root_path(&Path::new(b"/tmp/iceoryx2/").unwrap());
    let node = NodeBuilder::new().config(&iox).create::<ipc::Service>()?;

    let start = Instant::now();
    let mut count = 0usize;
    let mut out = std::fs::File::create(outfile)?;
    use std::io::Write;

    if bus_type == "f32" {
        writeln!(out, "msg,elapsed_ms,payload_len,sample_count,peak,rms")?;
        let svc = node.service_builder(&bus_name.try_into()?)
            .publish_subscribe::<[f32]>().open_or_create()?;
        let sub = svc.subscriber_builder().create()?;

        while count < max_msgs {
            while let Some(sample) = sub.receive()? {
                let p = sample.payload();
                let elapsed = start.elapsed().as_millis();
                let peak = p.iter().map(|s| s.abs()).fold(0.0f32, f32::max);
                let rms = (p.iter().map(|s| s * s).sum::<f32>() / p.len() as f32).sqrt();
                count += 1;
                writeln!(out, "{},{},{},{},{:.6},{:.6}", count, elapsed, p.len() * 4, p.len(), peak, rms)?;
                eprintln!("[REC:{}] #{} t={}ms samples={} peak={:.4} rms={:.4}", bus_name, count, elapsed, p.len(), peak, rms);
                if count >= max_msgs { break; }
            }
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
    } else {
        writeln!(out, "msg,elapsed_ms,payload_len,byte_count,min_val,max_val")?;
        let svc = node.service_builder(&bus_name.try_into()?)
            .publish_subscribe::<[u8]>().open_or_create()?;
        let sub = svc.subscriber_builder().create()?;

        while count < max_msgs {
            while let Some(sample) = sub.receive()? {
                let p = sample.payload();
                let elapsed = start.elapsed().as_millis();
                let min_val = p.iter().copied().min().unwrap_or(0);
                let max_val = p.iter().copied().max().unwrap_or(0);
                count += 1;
                writeln!(out, "{},{},{},{},{},{}", count, elapsed, p.len(), p.len(), min_val, max_val)?;
                eprintln!("[REC:{}] #{} t={}ms len={} min=0x{:02X} max=0x{:02X}", bus_name, count, elapsed, p.len(), min_val, max_val);
                if count >= max_msgs { break; }
            }
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
    }

    eprintln!("[REC:{}] DONE: {} msgs → {}", bus_name, count, outfile);
    Ok(())
}

fn chrono_stamp() -> String {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH).unwrap();
    format!("{}", now.as_secs())
}
