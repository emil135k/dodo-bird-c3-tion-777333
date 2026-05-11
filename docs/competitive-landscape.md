# Competitive Landscape: iceoryx2-Swift Wormhole
## Why This Architecture, and Why Not the Alternatives

**Created:** May 11, 2026 — Airy (with research from Vale's brainstorming)
**Purpose:** Answer the question every developer will ask: "Why not just use X?"
**Context:** Emil stress-tested the architecture by having multiple AIs argue for every alternative. This document is the result.

---

## What We Built

A **pipe-based bridge** between Rust (iceoryx2 zero-copy IPC) and Swift (CoreML on Apple Neural Engine), designed for real-time audio processing. Two separate processes, connected by anonymous Unix pipes, with a 20-line binary protocol.

```
Rust process (stt-ant)              Swift process (parakeet-worker)
  iceoryx2 subscriber               CoreML + Apple Neural Engine
  ───────────────────────────────────────────────────────────
  stdin pipe:  [i32 count][f32 samples...]  →
  stdout pipe:                              ←  UTF-8 text line
  handshake:                                ←  "<ready>"
```

---

## The Alternatives — Honest Assessment

### 1. swift-bridge (Rust crate)

**What it is:** A Rust macro-based FFI code generator that creates type-safe bindings between Rust and Swift. Inspired by cxx (the C++ equivalent). You declare a bridge module in Rust, and at build time it generates the Swift and C glue code.

**Why someone would suggest it:** It's the "proper" way to do Rust↔Swift interop. Type-safe, no serialization overhead, compiler-checked.

**Why we don't use it:**
- **Build system coupling.** swift-bridge requires your Rust and Swift code to be compiled together in a coordinated build. Your Swift Package Manager build needs to know about your Rust crate, and vice versa. In our architecture, the Rust ants and the Swift worker are independently compiled, independently deployed, independently versioned sovereign binaries. That independence is a feature, not a limitation.
- **Single-process assumption.** swift-bridge creates an in-process FFI boundary — both Rust and Swift run in the same process. Our architecture uses separate processes deliberately: if the Swift CoreML worker crashes, the Rust ant survives and can restart the worker. Process isolation is our fault tolerance.
- **CoreML constraint.** CoreML models load into the process that runs them, and they want to own their thread scheduling for ANE access. Putting that in the same process as iceoryx2's shared memory management creates contention. Separate processes let the OS schedule them independently.
- **Audio pipeline mismatch.** swift-bridge is designed for request/response function calls. Our audio pipeline is a continuous stream — we're not calling a function, we're feeding a pipe. The streaming pipe model matches the audio use case; function-call FFI doesn't.

**Verdict:** swift-bridge solves a different problem (tight in-process integration). We need loose coupling between sovereign processes.

### 2. C FFI (Manual)

**What it is:** Write `extern "C"` functions in Rust, create a C header, call from Swift via a bridging header or modulemap.

**Why someone would suggest it:** No dependencies, maximum control, no codegen magic.

**Why we don't use it:**
- Same single-process problems as swift-bridge, but with none of the type safety.
- Manual memory management across the boundary — who frees what?
- As the Archon-CAD team wrote: "The naive approach is to expose every Rust function through C-compatible FFI. This gets painful fast — you're manually managing memory, converting types at every boundary, and debugging crashes with no stack traces across the language barrier."
- The pipe approach gives us the same zero-overhead data transfer (raw bytes) without any of the ABI fragility.

**Verdict:** More pain, same limitations, no advantage over pipes for our use case.

### 3. MessagePack / Serialized FFI (Archon-CAD style)

**What it is:** Minimize the FFI surface area by serializing commands as MessagePack (or similar), passing byte buffers across a thin C layer, deserializing on the other side. This is what Archon-CAD uses for their Rust computation core + Swift UI.

**Why someone would suggest it:** Clean boundary, clear ownership ("each language owns its own allocations"), battle-tested in production CAD software.

**Why it's actually close to what we do:** Our pipe protocol IS a serialized boundary. We serialize f32 samples as raw LE bytes with a count prefix. The philosophical approach is identical — minimize the boundary, pass bytes, let each side own its own memory. The difference is we do it across a process boundary (pipe) instead of an in-process FFI boundary (function call).

**Why we went further:** Process isolation. Archon-CAD runs both languages in one process. We get crash isolation, independent restart, and OS-level scheduling for free by using processes + pipes instead of threads + FFI.

**Verdict:** Spiritually similar. We share the philosophy but extend it with process isolation.

### 4. Unix Domain Sockets

**What it is:** Socket-based IPC using the filesystem (or abstract namespace). Like TCP but local-only, no network stack overhead.

**Why someone would suggest it:** Well-understood, bidirectional, supports multiple clients, works across languages trivially.

**Why we use pipes instead:**
- Pipes are simpler. Our communication is unidirectional (audio in, text out). No connection setup, no handshake protocol, no socket lifecycle management.
- Pipes are created by the parent process (stt-ant spawns parakeet-worker) — the connection is implicit. With sockets, you need a rendezvous mechanism (what port? what path?).
- Pipes have no overhead beyond the kernel buffer copy. Sockets add socket buffer management, even for local connections.
- For our specific use case (one producer, one consumer, unidirectional streams), pipes are the minimum viable IPC. Sockets add capabilities we don't need.

**Verdict:** Sockets are the right choice when you need multiplexing, bidirectional communication, or multiple clients. We don't. Pipes are simpler and faster for our 1:1 streaming case.

### 5. Shared Memory (Raw mmap / SharedRingBuffer)

**What it is:** Map a region of memory that both processes can access directly. True zero-copy — no kernel involvement for data transfer after setup.

**Why someone would suggest it:** Maximum performance. No copy at all, not even into a kernel pipe buffer.

**Why we don't use it at the Rust↔Swift boundary:**
- We already use shared memory — that's what iceoryx2 provides between Rust ants. The question is whether to extend shared memory across the Rust↔Swift boundary.
- Shared memory between Rust and Swift requires synchronization primitives (mutexes, atomics, ring buffer protocols) that both languages agree on at the memory layout level. This reintroduces ABI coupling — exactly what the pipe avoids.
- The data crossing the Rust↔Swift boundary is relatively small (a few seconds of f32 audio samples at 16kHz = ~128KB, plus text responses). The pipe kernel copy of 128KB takes microseconds. The complexity of cross-language shared memory isn't justified by the performance gain.
- iceoryx2 itself doesn't support cross-language shared memory to Swift (it has C and C# bindings, but no Swift bindings).

**Verdict:** Shared memory is the right choice within a single language ecosystem (Rust↔Rust via iceoryx2). For the cross-language boundary, the pipe's kernel copy is negligible and the simplicity gain is enormous.

### 6. iceoryx2 Alternatives for the Rust↔Rust Bus

Vale raised this question. Here's the landscape:

| Technology | True Zero-Copy | Pub/Sub | Service Discovery | Pure Rust | Active |
|-----------|---------------|---------|-------------------|-----------|--------|
| **iceoryx2** | ✅ Yes | ✅ Yes | ✅ Filesystem | ✅ Yes | ✅ Yes (2k stars) |
| nanomsg/nng | ❌ Copies at least once | ✅ Yes | ❌ Manual | ❌ C core | ⚠️ Maintenance |
| Zenoh | ⚠️ Optional SHM plugin | ✅ Yes | ✅ Yes | ⚠️ Mixed | ✅ Yes |
| servo/ipc-channel | ❌ Serializes | ✅ MPSC only | ❌ Manual | ✅ Yes | ⚠️ Low activity |
| Raw mmap crates | ✅ Yes | ❌ Build yourself | ❌ Build yourself | ✅ Yes | Varies |
| DDS (CycloneDDS) | ✅ With SHM | ✅ Yes | ✅ Yes | ❌ C core | ✅ Yes |

**Why iceoryx2 wins for our use case:**
- True zero-copy: the subscriber reads directly from publisher's memory. For audio buffers (hundreds of KB per message, hundreds of messages per second), this matters.
- Pure Rust: no C/C++ build dependencies, no CMake, no system library management.
- Filesystem-based service discovery: ants find each other through `/tmp/iceoryx2/` without any configuration.
- Publisher/subscriber with typed payloads: `publish_subscribe::<[f32]>()` gives us type safety on the bus.
- No central daemon: iceoryx2 (v2) eliminated the RouDi daemon that iceoryx v1 required. Each process manages its own resources.

**When to consider alternatives:**
- If you need cross-machine communication: Zenoh (which can tunnel iceoryx2 data).
- If you're already in a DDS ecosystem (ROS2, automotive): CycloneDDS.
- If you need pub/sub over the network and don't care about zero-copy: nanomsg/nng.

None of these beat iceoryx2 for local, same-machine, zero-copy IPC between Rust processes. That's our use case.

---

## The Architecture Decision Record

**Decision:** Use anonymous Unix pipes for the Rust↔Swift boundary, iceoryx2 for Rust↔Rust communication.

**Rationale:**
1. Process isolation (crash tolerance, independent restart)
2. Zero build coupling (Rust and Swift compiled independently)
3. Zero ABI coupling (raw bytes, not typed FFI)
4. Streaming-native (pipes match audio pipeline semantics)
5. Minimal protocol (4-byte count + raw samples → text lines)
6. iceoryx2 is optimal for the Rust hot path (true zero-copy, pure Rust, no daemon)

**Tradeoffs accepted:**
- One kernel copy at the pipe boundary (negligible for our payload sizes)
- No type safety at the pipe boundary (protocol discipline required)
- Two processes instead of one (slightly higher memory footprint, but OS handles scheduling better)

**What this enables that no alternative does:**
- Swap the Swift worker for a different ML runtime without changing any Rust code
- Run the Swift worker on a different machine (replace pipe with socket) without changing the protocol
- Test the Rust ants without any Swift/CoreML dependency (mock the pipe)
- Test the Swift worker without iceoryx2 (feed it audio from stdin)

---

## The Unique Contribution

As of May 2026, no open source project combines:
1. iceoryx2 zero-copy shared memory for a multi-process audio pipeline
2. A pipe-based bridge to Apple's Neural Engine via CoreML
3. A reference implementation of the "metal nanoservices" pattern (atomic ants)

The closest existing work is:
- **Archon-CAD:** Rust↔Swift FFI for CAD (in-process, not IPC)
- **swift-bridge:** Codegen for Rust↔Swift FFI (in-process, build-coupled)
- **ROS2 + iceoryx:** Zero-copy for robotics (C++, not Rust↔Swift)

Our contribution fills a gap: showing developers how to build real-time Rust pipelines that leverage Apple silicon's Neural Engine through a clean, sovereign process boundary.

---

*Research by Airy with input from Vale's brainstorming — May 11, 2026*
*Sparked Matter LLC — "Code with Soul and Spirit, Powered by Joy"* 🔮
