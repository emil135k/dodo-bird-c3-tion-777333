// Parakeet Worker — STT via Apple Neural Engine
//
// Reads raw f32 audio samples from stdin (16kHz mono).
// Runs Parakeet CoreML inference on ANE.
// Writes transcribed text to stdout.
//
// Protocol:
//   Input:  4-byte little-endian i32 (sample count) + f32 samples
//   Output: UTF-8 text line per transcription
//           Empty transcriptions emit "<empty>" so upstream knows STT completed.
//           Errors emit "<error>".
//
// Contract: Each input is one complete VAD-segmented utterance.
// This worker is a compute engine, not a pipeline coordinator.
//
// Zero disk. Pipe only.

import Foundation
import ParakeetTDT

// Safety bounds
let SAMPLE_RATE: Int = 16000
let MAX_UTTERANCE_SECONDS: Int = 60
let MAX_SAMPLES: Int32 = Int32(MAX_UTTERANCE_SECONDS * SAMPLE_RATE)  // 960000

@main
struct ParakeetWorker {
    static func main() async {
        fputs("[PARAKEET-WORKER] Loading CoreML models...\n", stderr)

        let transcriber: ParakeetTranscriber
        do {
            transcriber = try await ParakeetTranscriber.fromHuggingFace(computeUnits: .ane)
        } catch {
            fputs("[PARAKEET-WORKER] FATAL: \(error)\n", stderr)
            _Exit(1)
        }

        fputs("[PARAKEET-WORKER] Ready (CoreML ANE)\n", stderr)

        // Readiness handshake: emit READY on stdout so Rust knows model is loaded
        if let data = "<ready>\n".data(using: .utf8) {
            FileHandle.standardOutput.write(data)
        }

        // Read loop — stdin protocol: [i32 sample_count][f32 samples...]
        let stdinHandle = FileHandle.standardInput
        let stdoutHandle = FileHandle.standardOutput

        while true {
            // Read sample count (4 bytes, little-endian i32)
            let countData = stdinHandle.readData(ofLength: 4)
            if countData.count < 4 { break } // EOF

            let sampleCount = countData.withUnsafeBytes { $0.load(as: Int32.self).littleEndian }

            // Validate sample count
            if sampleCount <= 0 {
                fputs("[PARAKEET-WORKER] WARN: invalid sample count \(sampleCount), skipping\n", stderr)
                continue
            }
            if sampleCount > MAX_SAMPLES {
                // Oversized or corrupt header — pipe protocol is likely desynchronized.
                // Draining is unsafe (may block forever on corrupt count).
                // Exit so Rust ant detects worker death and fails fast.
                fputs("[PARAKEET-WORKER] FATAL: sample count \(sampleCount) exceeds max \(MAX_SAMPLES) — protocol desync, exiting\n", stderr)
                _Exit(1)
            }

            // Read f32 samples
            let byteCount = Int(sampleCount) * 4
            var audioData = Data()
            while audioData.count < byteCount {
                let chunk = stdinHandle.readData(ofLength: byteCount - audioData.count)
                if chunk.isEmpty { break }
                audioData.append(chunk)
            }

            if audioData.count < byteCount {
                fputs("[PARAKEET-WORKER] Incomplete audio: got \(audioData.count)/\(byteCount) bytes\n", stderr)
                continue
            }

            // Convert to [Float]
            let samples: [Float] = audioData.withUnsafeBytes { ptr in
                Array(ptr.bindMemory(to: Float.self))
            }

            let duration = Float(samples.count) / Float(SAMPLE_RATE)
            fputs("[PARAKEET-WORKER] Processing \(String(format: "%.1f", duration))s audio...\n", stderr)

            // Transcribe
            do {
                let result = try transcriber.transcribe(samples: samples)
                let text = result.text.trimmingCharacters(in: .whitespacesAndNewlines)

                if !text.isEmpty {
                    fputs("[PARAKEET-WORKER] \"\(text)\" (\(String(format: "%.0f", result.rtfx))x RT)\n", stderr)
                    if let data = (text + "\n").data(using: .utf8) {
                        stdoutHandle.write(data)
                    }
                } else {
                    fputs("[PARAKEET-WORKER] Empty transcription\n", stderr)
                    if let data = "<empty>\n".data(using: .utf8) {
                        stdoutHandle.write(data)
                    }
                }
            } catch {
                fputs("[PARAKEET-WORKER] Error: \(error)\n", stderr)
                if let data = "<error>\n".data(using: .utf8) {
                    stdoutHandle.write(data)
                }
            }
        }

        fputs("[PARAKEET-WORKER] stdin closed, exiting\n", stderr)
    }
}
