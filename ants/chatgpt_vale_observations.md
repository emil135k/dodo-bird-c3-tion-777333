# ChatGPT Vale Observations

Running review/history log for hypAiAssist ant observations.

---

## 2026-05-02 — stt-ant re-review after dodo update

Repository: `emil135k/dodo-bird-c3-tion-777333`

Files reviewed:

- `ants/stt-ant/src/main.rs`
- `ants/stt-ant/swift-worker/Sources/main.swift`

### Summary

`stt-ant` has improved substantially after the latest update. The Rust adapter now treats Swift worker stdin write/flush failures as fatal, which closes the prior zombie-live failure mode. The Swift worker now validates `sampleCount`, enforces a 60-second maximum utterance size, drains oversized payloads to preserve protocol sync, and emits explicit `<empty>` / `<error>` markers so upstream knows an utterance completed or failed.

### Fixed / improved

#### Rust pipe-write fatality

The prior P1 was that write failures only logged and broke out of the inner receive loop, allowing the ant to continue alive but useless. Current code returns `Err(...)` on failed `write_all` or `flush`, which is the correct fail-fast behavior for the current no-restart architecture.

Status: **FIXED**

#### Swift sampleCount bounds

The worker now defines:

```swift
let SAMPLE_RATE: Int = 16000
let MAX_UTTERANCE_SECONDS: Int = 60
let MAX_SAMPLES: Int = MAX_UTTERANCE_SECONDS * SAMPLE_RATE
```

It rejects `sampleCount <= 0` and `sampleCount > 960000`. Oversized payloads are drained before continuing so the pipe protocol remains synchronized.

Status: **FIXED**

#### Empty / error transcription handling

Swift now emits explicit stdout markers:

```text
<empty>
<error>
```

Rust recognizes those markers, logs them, and does not publish them as normal `stt_text` events. This is an acceptable interim contract until utterance IDs exist.

Status: **ACCEPTABLE / IMPROVED**

### Remaining concern

#### Swift initialization still blocks around async work

The previous direct `Task { ... } + semaphore.wait()` pattern has been mitigated, but the current implementation still mixes Swift structured concurrency with blocking primitives:

```swift
DispatchQueue.global(qos: .userInitiated).async {
    let group = DispatchGroup()
    group.enter()

    Task {
        result = try await ParakeetTranscriber.fromHuggingFace(computeUnits: .ane)
        group.leave()
    }

    group.wait()
    done.signal()
}

done.wait()
```

This is better than blocking the main thread directly, but it is still not a clean certification-grade init pattern. Preferred fix remains a true async entry point, for example:

```swift
@main
struct ParakeetWorker {
    static func main() async {
        do {
            let transcriber = try await ParakeetTranscriber.fromHuggingFace(computeUnits: .ane)
            runLoop(transcriber)
        } catch {
            fputs("[PARAKEET-WORKER] FATAL: \\(error)\n", stderr)
            exit(1)
        }
    }
}
```

Status: **PARTIALLY MITIGATED, NOT FULLY CERTIFIED**

### Certification read

```text
Rust pipe fatality: FIXED
Swift sampleCount bound: FIXED
Empty transcription behavior: FIXED / EXPLICIT
Swift init deadlock risk: PARTIALLY MITIGATED
Boundary contract: PRESERVED
Architecture: STRONG
```

Review label:

```text
stt-ant: near-certification
Blocker remaining: replace Swift init with true async @main or equivalent non-blocking init
```

### Vale note

The dodo is no longer wobbling into traffic. It is mostly wearing a little helmet now. Still needs the Swift init cleaned up before certification.
