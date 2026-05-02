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
let MAX_SAMPLES: Int = MAX_UTTERANCE_SECONDS * SAMPLE_RATE  // 960000

// Initialize Parakeet with ANE — synchronous, no async deadlock risk
fputs("[PARAKEET-WORKER] Loading CoreML models...\n", stderr)

let transcriber: ParakeetTranscriber
do {
    // Use RunLoop-based async execution to avoid Task + semaphore deadlock.
    // The semaphore.wait() + Task pattern can deadlock when no async executor
    // is making progress on the main thread.
    var result: ParakeetTranscriber?
    var initError: Error?
    let done = DispatchSemaphore(value: 0)

    DispatchQueue.global(qos: .userInitiated).async {
        let group = DispatchGroup()
        group.enter()

        Task {
            do {
                result = try await ParakeetTranscriber.fromHuggingFace(computeUnits: .ane)
            } catch {
                initError = error
            }
            group.leave()
        }

        group.wait()
        done.signal()
    }

    done.wait()

    if let err = initError {
        fputs("[PARAKEET-WORKER] FATAL: \(err)\n", stderr)
        exit(1)
    }
    transcriber = result!
}

fputs("[PARAKEET-WORKER] Ready (CoreML ANE)\n", stderr)

// Read loop — stdin protocol: [i32 sample_count][f32 samples...]
let stdin = FileHandle.standardInput
let stdout = FileHandle.standardOutput

while true {
    // Read sample count (4 bytes, little-endian i32)
    let countData = stdin.readData(ofLength: 4)
    if countData.count < 4 { break } // EOF

    let sampleCount = countData.withUnsafeBytes { $0.load(as: Int32.self).littleEndian }

    // Validate sample count
    if sampleCount <= 0 {
        fputs("[PARAKEET-WORKER] WARN: invalid sample count \(sampleCount), skipping\n", stderr)
        continue
    }
    if sampleCount > Int32(MAX_SAMPLES) {
        fputs("[PARAKEET-WORKER] WARN: sample count \(sampleCount) exceeds max \(MAX_SAMPLES) (\(MAX_UTTERANCE_SECONDS)s), skipping\n", stderr)
        // Drain the oversized payload to stay in sync with the protocol
        let drainBytes = Int(sampleCount) * 4
        var drained = 0
        while drained < drainBytes {
            let chunk = stdin.readData(ofLength: min(65536, drainBytes - drained))
            if chunk.isEmpty { break }
            drained += chunk.count
        }
        continue
    }

    // Read f32 samples
    let byteCount = Int(sampleCount) * 4
    var audioData = Data()
    while audioData.count < byteCount {
        let chunk = stdin.readData(ofLength: byteCount - audioData.count)
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
                stdout.write(data)
            }
        } else {
            // Emit explicit empty result so upstream knows STT completed
            fputs("[PARAKEET-WORKER] Empty transcription\n", stderr)
            if let data = "<empty>\n".data(using: .utf8) {
                stdout.write(data)
            }
        }
    } catch {
        fputs("[PARAKEET-WORKER] Error: \(error)\n", stderr)
        // Emit error marker so upstream knows this utterance failed
        if let data = "<error>\n".data(using: .utf8) {
            stdout.write(data)
        }
    }
}

fputs("[PARAKEET-WORKER] stdin closed, exiting\n", stderr)
