// Parakeet Worker — STT via Apple Neural Engine
//
// Reads raw f32 audio samples from stdin (16kHz mono).
// Runs Parakeet CoreML inference on ANE.
// Writes transcribed text to stdout.
//
// Protocol:
//   Input:  4-byte little-endian i32 (sample count) + f32 samples
//   Output: UTF-8 text line per transcription
//
// Zero disk. Pipe only.

import Foundation
import ParakeetTDT

// Initialize Parakeet with ANE
fputs("[PARAKEET-WORKER] Loading CoreML models...\n", stderr)

let transcriber: ParakeetTranscriber
do {
    let semaphore = DispatchSemaphore(value: 0)
    var result: ParakeetTranscriber?
    var initError: Error?

    Task {
        do {
            result = try await ParakeetTranscriber.fromHuggingFace(computeUnits: .ane)
        } catch {
            initError = error
        }
        semaphore.signal()
    }
    semaphore.wait()

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
    if sampleCount <= 0 { continue }

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

    let duration = Float(samples.count) / 16000.0
    fputs("[PARAKEET-WORKER] Processing \(String(format: "%.1f", duration))s audio...\n", stderr)

    // Transcribe
    do {
        let result = try transcriber.transcribe(samples: samples)
        let text = result.text.trimmingCharacters(in: .whitespacesAndNewlines)

        if !text.isEmpty {
            fputs("[PARAKEET-WORKER] \"\(text)\" (\(String(format: "%.0f", result.rtfx))x RT)\n", stderr)
            // Write text line to stdout
            if let data = (text + "\n").data(using: .utf8) {
                stdout.write(data)
            }
        } else {
            fputs("[PARAKEET-WORKER] Empty transcription\n", stderr)
        }
    } catch {
        fputs("[PARAKEET-WORKER] Error: \(error)\n", stderr)
    }
}

fputs("[PARAKEET-WORKER] stdin closed, exiting\n", stderr)
