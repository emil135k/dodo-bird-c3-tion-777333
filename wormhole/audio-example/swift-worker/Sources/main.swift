// Patchbay Worker — Audio I/O with Apple AEC + Kokoro CoreML TTS
//
// Uses AVAudioEngine with Voice Processing for hardware echo cancellation.
// Apple handles AEC at the driver level — no manual delay/amplitude tuning.
//
// Protocol (pipes to/from Rust patchbay-ant):
//   stdin:  Text lines to synthesize (UTF-8, one per line)
//           Special: "<tts_audio>N\n" followed by N bytes of f32 PCM to play
//   stdout: Clean mic PCM frames (f32 16kHz mono, echo-cancelled by Apple)
//           Format: [i32 sample_count LE][f32 samples...]
//
// The Rust side handles iceoryx2 bus. This worker handles Apple audio.

import Foundation
import AVFAudio
import CoreML

let SAMPLE_RATE: Double = 16000.0
let MIC_BUFFER_SIZE: AVAudioFrameCount = 4096

@main
struct PatchbayWorker {
    static func main() async {
        fputs("[PATCHBAY-WORKER] Starting — AVAudioEngine + Voice Processing\n", stderr)

        let engine = AVAudioEngine()

        // Player node for TTS playback — attach BEFORE configuring voice processing
        let playerNode = AVAudioPlayerNode()
        engine.attach(playerNode)

        // Connect player to output at 24kHz (Kokoro output rate)
        let playFormat = AVAudioFormat(
            commonFormat: .pcmFormatFloat32,
            sampleRate: 24000.0,
            channels: 1,
            interleaved: false
        )!
        engine.connect(playerNode, to: engine.mainMixerNode, format: playFormat)

        // Enable Voice Processing — THIS IS THE AEC
        // Must be done AFTER attaching nodes but BEFORE starting
        do {
            try engine.inputNode.setVoiceProcessingEnabled(true)
            fputs("[PATCHBAY-WORKER] Voice Processing (AEC): ENABLED\n", stderr)

            // Disable AGC — stop Apple from controlling mic gain
            if engine.inputNode.isVoiceProcessingAGCEnabled {
                engine.inputNode.isVoiceProcessingAGCEnabled = false
                fputs("[PATCHBAY-WORKER] Voice Processing AGC: DISABLED\n", stderr)
            }

            // Disable audio ducking — stop Apple from attenuating system output
            // This is the #1 suspect for low volume (confirmed by Airy's diagnosis)
            if #available(macOS 14.0, *) {
                engine.inputNode.voiceProcessingOtherAudioDuckingConfiguration =
                    .init(enableAdvancedDucking: false, duckingLevel: .min)
                fputs("[PATCHBAY-WORKER] Voice Processing Ducking: MIN (no attenuation)\n", stderr)
            }
        } catch {
            fputs("[PATCHBAY-WORKER] WARN: Voice Processing failed: \(error)\n", stderr)
        }

        // Get the actual mic format after voice processing is enabled
        let micFormat = engine.inputNode.outputFormat(forBus: 0)
        fputs("[PATCHBAY-WORKER] Mic format: \(micFormat.sampleRate)Hz, \(micFormat.channelCount)ch\n", stderr)

        // Request mono f32 at the mic's native rate for the tap
        // Pass nil format to let AVAudioEngine handle conversion from 9ch voice processing
        let tapFormat = AVAudioFormat(
            commonFormat: .pcmFormatFloat32,
            sampleRate: micFormat.sampleRate > 0 ? micFormat.sampleRate : 48000.0,
            channels: 1,
            interleaved: false
        )!
        fputs("[PATCHBAY-WORKER] Tap format: \(tapFormat.sampleRate)Hz, \(tapFormat.channelCount)ch\n", stderr)

        // Install tap on input (mic) — echo-cancelled audio
        let stdoutHandle = FileHandle.standardOutput
        var tapFrameCount: UInt64 = 0
        engine.inputNode.installTap(
            onBus: 0,
            bufferSize: MIC_BUFFER_SIZE,
            format: tapFormat
        ) { buffer, time in
            tapFrameCount += 1
            // Convert to 16kHz mono if needed
            guard let channelData = buffer.floatChannelData else { return }
            let frameCount = Int(buffer.frameLength)
            if frameCount == 0 { return }

            // Get mono samples
            let samples = Array(UnsafeBufferPointer(start: channelData[0], count: frameCount))

            // Downsample to 48kHz for stt_raw contract
            let inputRate = buffer.format.sampleRate
            let outputSamples: [Float]
            if inputRate > 48000.0 {
                let ratio = inputRate / 48000.0
                let outLen = Int(Double(frameCount) / ratio)
                var resampled = [Float](repeating: 0, count: outLen)
                for i in 0..<outLen {
                    let src = Double(i) * ratio
                    let idx = min(Int(src), samples.count - 1)
                    resampled[i] = samples[idx]
                }
                outputSamples = resampled
            } else {
                outputSamples = samples
            }

            // Write to stdout: [i32 count][f32 samples...] using POSIX write for thread safety
            let count = Int32(outputSamples.count)
            var countLE = count.littleEndian
            withUnsafeBytes(of: &countLE) { ptr in
                _ = Darwin.write(STDOUT_FILENO, ptr.baseAddress!, 4)
            }
            outputSamples.withUnsafeBufferPointer { ptr in
                _ = Darwin.write(STDOUT_FILENO, ptr.baseAddress!, ptr.count * 4)
            }

            if tapFrameCount % 100 == 1 {
                fputs("[PATCHBAY-WORKER] Tap #\(tapFrameCount): wrote \(outputSamples.count) samples\n", stderr)
            }
        }

        // Start engine
        engine.prepare()
        do {
            try engine.start()
            playerNode.play()
            fputs("[PATCHBAY-WORKER] AVAudioEngine: RUNNING\n", stderr)
        } catch {
            fputs("[PATCHBAY-WORKER] FATAL: Engine start failed: \(error)\n", stderr)
            _Exit(1)
        }

        // Readiness handshake
        if let data = "<ready>\n".data(using: .utf8) {
            stdoutHandle.write(data)
        }
        fputs("[PATCHBAY-WORKER] Ready — mic (AEC), speaker, pipes\n", stderr)

        // Read loop — stdin: receives f32 PCM audio to play (from tts-ant via Rust)
        // Protocol: [i32 sample_count LE][f32 samples at 24kHz...]
        // Boost playback volume to counter AGC ducking
        playerNode.volume = 2.5
        engine.mainMixerNode.outputVolume = 1.0
        fputs("[PATCHBAY-WORKER] Player volume: \(playerNode.volume)\n", stderr)

        let stdinHandle = FileHandle.standardInput

        while true {
            // Read sample count
            let countData = stdinHandle.readData(ofLength: 4)
            if countData.count < 4 { break } // EOF

            let sampleCount = countData.withUnsafeBytes { $0.load(as: Int32.self).littleEndian }
            if sampleCount <= 0 { continue }
            if sampleCount > 960000 {
                fputs("[PATCHBAY-WORKER] FATAL: sample count \(sampleCount) too large\n", stderr)
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
                fputs("[PATCHBAY-WORKER] Incomplete audio: \(audioData.count)/\(byteCount)\n", stderr)
                continue
            }

            // Convert to PCM buffer and play
            let samples: [Float] = audioData.withUnsafeBytes { ptr in
                Array(ptr.bindMemory(to: Float.self))
            }

            guard let pcmBuffer = AVAudioPCMBuffer(
                pcmFormat: playFormat,
                frameCapacity: AVAudioFrameCount(samples.count)
            ) else { continue }

            pcmBuffer.frameLength = AVAudioFrameCount(samples.count)
            if let channelData = pcmBuffer.floatChannelData {
                for i in 0..<samples.count {
                    channelData[0][i] = samples[i]
                }
            }

            let dur = Float(samples.count) / 24000.0
            fputs("[PATCHBAY-WORKER] Playing \(String(format: "%.1f", dur))s audio\n", stderr)
            playerNode.scheduleBuffer(pcmBuffer, completionHandler: nil)
        }

        fputs("[PATCHBAY-WORKER] stdin closed, exiting\n", stderr)
    }
}
