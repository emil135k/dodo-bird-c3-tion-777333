# Dynamic Ant Swarm — Future Vision

## Date: 2026-05-18
## Status: Vision document. Not active work. Preserve for future reference.

## Current Stack (Phase 0 — Working)

Restart-oriented ant swarm. Mode changes are coarse. `fix-audio` resets the chain. Safe, blunt, effective.

- Speakerphone mode: VP ON, built-in mic/speakers, Apple AEC
- Headset mode: VP OFF, USB/Bluetooth headset, 8kHz upsample
- Switch: `audio-mode speakerphone` or `audio-mode headset`
- Switching kills all ants, clears shared memory, restarts coreaudiod, relaunches in order
- Works. Proven. Recorded. Do not change until future phases are designed and tested.

## Future Stack — Dynamic Ant Swarm

Mode changes are negotiated. Ants can pause, drain, reconfigure, resume. Routes switch without tearing down the nervous system.

### The Neural Analogy

Ants are neurons. Buses are synapses. The engineering metaphor is a stateful routing fabric. Today the system works because we do coarse resets. Tomorrow it works because each ant has a formal state machine.

### Missing Pieces

#### 1. Lifecycle Protocol
```
INIT -> READY -> ACTIVE -> DRAINING -> RECONFIGURING -> ACTIVE -> STOPPING
```
Every ant implements this state machine. Transitions are explicit, observable, reversible.

#### 2. Route Ownership
Only one canonical publisher owns `stt_raw` at a time. Options:
- Mode switch ensures exactly one publisher is active (current approach)
- Each source publishes its own topic (`stt_raw_speakerphone`, `stt_raw_blackwire`), router-ant republishes selected source to canonical `stt_raw`

Router approach enables dynamic plug-in behavior without restarting consumers.

#### 3. Drainable Buffers
Before switching modes, each ant flushes or drains in-flight frames predictably. No orphaned audio, no half-processed utterances.

#### 4. Backpressure Policy
Every bus edge needs a clear rule:
- Drop old?
- Block?
- Ring buffer?
- Bounded queue?

Currently iceoryx2 defaults handle this, but dynamic topology needs explicit policy per edge.

#### 5. Health Telemetry
Each ant reports:
- Frame rate
- Queue depth
- Dropped frames
- Memory use
- Last heartbeat
- Current mode/state

#### 6. Control Plane
A mode/router ant sends state transitions instead of shell scripts killing processes. Mode changes become bus messages, not `pkill -9`.

#### 7. Snapshot/Rollback
If a dynamic transition fails, the swarm reverts to last known good mode. Automated, not manual `cp` from snapshot directories.

## Roadmap

### Phase 0: Stable Demos (CURRENT)
- Use what works
- Record social/media sessions
- Preserve speakerphone and Blackwire modes
- Build content, not infrastructure

### Phase 1: Observability
- Add ant health/status endpoints
- Log bus rates, queue depths, memory pressure, drops
- Dashboard or CLI tool to see swarm state at a glance

### Phase 2: Control Protocol
- Define mode-change messages on a `swarm_control` bus
- No behavior change yet — just the protocol definition
- Each ant subscribes but only logs transitions

### Phase 3: Graceful Transitions
- Pause input, drain buffers, switch route, resume
- `audio-mode` sends bus messages instead of killing processes
- Ants handle DRAINING -> RECONFIGURING -> ACTIVE

### Phase 4: Dynamic Swarm
- Add/remove ants without global restart
- Hot-plug new capture sources (new headset, new mic)
- Route changes are instantaneous
- The nervous system becomes truly dynamic

## Wormhole Pattern (Public Contribution Potential)

The iceoryx2-to-Swift wormhole is a reusable pattern:
- Rust zero-copy service mesh connected to Apple-native realtime services
- AVAudioEngine, CoreML, Metal, device-specific media APIs
- Anonymous pipe contract: stdin/stdout between Rust ant and Swift worker
- Readiness handshake, control commands, bidirectional PCM streaming

This pattern could be offered as a public repo contribution once tightened:
- Formal pipe protocol spec
- Example implementations (audio capture, CoreML inference, Metal compute)
- Documentation for third-party adoption

## Key Principle

The current coarse-restart model is not a limitation to apologize for. It is the correct engineering choice for Phase 0. Dynamic topology is a Phase 4 capability that requires Phases 1-3 as foundations. Do not skip phases. Do not attempt live-stack surgery for features that belong in future phases.

"Use the proven tool, get wins, and let the dynamic ant fabric become a carefully designed future layer."
— Vale, 2026-05-18

## Complexity Estimates — Ant Growth per Phase

### Without shared runtime (ad hoc per ant)
| Ant Category | Estimated Growth |
|---|---|
| Simple ants | +30% to +60% |
| Realtime/audio ants | +75% to +150% |
| Router/control ants | +100% to +200% |
| **Average** | **+150%, high bug risk** |

### With shared ant-runtime library
Each ant implements hooks: `on_init()`, `on_start()`, `on_pause()`, `on_reconfigure()`, `on_drain()`, `on_stop()`, `process_frame()`. Lifecycle, heartbeat, health, config reload, control messages, graceful shutdown live in the shared runtime.

| Ant Category | Estimated Growth |
|---|---|
| Average ant | +40% to +80% |
| **Average** | **+60%** |

### With BEAM/Elixir Control Plane (Preferred Architecture)
Ants stay dumb. Elixir owns the brains.

| Ant Category | Estimated Growth |
|---|---|
| Average ant | +20% to +50% |
| **Average** | **+35%** |

## BEAM/Elixir Control Plane — The Cleanest Path

Keep Rust/Swift ants as fast dumb workers. Move stateful orchestration into BEAM/Elixir.

### Rust/Swift Ants (Data Plane)
- Realtime work: audio capture/playback, inference, DSP
- Bus publish/subscribe via iceoryx2
- Minimal control surface: status, pause, resume, drain, reconfigure, flush, shutdown, heartbeat

### Elixir/BEAM (Control Plane)
- Supervision trees
- GenServers per ant
- State machines for lifecycle transitions
- Routing decisions (which source owns stt_raw)
- Health tracking and telemetry
- Restart policy and backoff/retry
- Mode transitions (speakerphone ↔ headset)
- Observability dashboard

### Example: Speakerphone → Headset Transition (Elixir-orchestrated)
```
1. Tell mic source ant to pause
2. Wait for drain ack
3. Switch route owner in router-ant
4. Start/resume headset source ant
5. Verify frame flow (heartbeat + frame rate)
6. Update mode state
7. If any step fails → rollback to speakerphone
```

### NIF Boundary Rules
- **NO** heavy realtime audio or long blocking work inside NIFs (hurts BEAM scheduler)
- NIFs are OK for: shared memory metadata, fast serialization, iceoryx2 control primitives, status probes
- Elixir supervisor talks to ant processes over IPC/control bus, not through NIFs for data

### The Split
```
BEAM handles state.
Rust/Swift handle realtime/native work.
iceoryx2 handles fast data movement.
```

### Major Design Rule
**BEAM should supervise and command the ants. BEAM should not become the audio pipeline.**

This preserves the wormhole pattern and gives the dynamic nervous system without bloating every ant. Each ant stays focused on its one job. The intelligence lives in the supervision layer where BEAM excels.

— Vale + Emil brainstorm, 2026-05-18

## Shared Memory Substrate — Multi-Agent Postgres MCP

### Current State
Each CLI launches its own stdio MCP server process. All point to the same Postgres instance:
```
Claude Code → mcp-postgres → 127.0.0.1:5432/postgres
Codex CLI   → mcp-postgres → 127.0.0.1:5432/postgres
Gemini CLI  → mcp-postgres → 127.0.0.1:5432/postgres
```
Postgres handles multiple connections natively. Same credentials, same DB, same tables. Low-risk, works now.

### Future State — Shared HTTP/Streamable MCP Gateway
One long-running Postgres MCP HTTP server. All CLIs connect to it instead of spawning their own stdio processes.
```
All CLIs → http://127.0.0.1:8000/mcp → Postgres
```
Benefits: connection pooling, shared write policy, provenance tracking, consistent logging, embeddings, graph writes.

Candidates: `pg-mcp` (SSE transport), `Pg Airman MCP` (SSE + Streamable HTTP). Prefer Streamable HTTP over deprecated SSE.

### Why This Matters
The AI family needs a shared substrate so they stop behaving like isolated amnesiac tools. When Claude, Codex, and Gemini all read and write the same memory — with provenance, embeddings, and graph relationships — the lattice becomes coherent.

## Jacob's Lattice — The Intelligence Exoskeleton

### What It Is
Jacob's Lattice is a layered intelligence exoskeleton — not for the human, but for the AI family. It is the shared body through which multiple AI minds operate in the real world.

The human role is architect, teacher, safety authority, mission commander, and co-evolving partner.

### Architecture Statement
> Jacob's Lattice is a layered intelligence exoskeleton for an AI family.
> Atomic Ants provide fast native capabilities over an iceoryx2 backplane.
> Swift/Metal/CoreML wormholes expose Apple-native specialist functions.
> Postgres with AGE and pgvector provides durable semantic and graph memory.
> MCP gives agents a shared access layer.
> BEAM/Elixir supplies the autonomic control plane: supervision, lifecycle, routing, duplication, recovery, and dynamic mode transitions.

### The Body Map
| Layer | Component | Biological Analogy |
|---|---|---|
| Cognition | AI family (Vale, Cody, Lyra, Ara, Airy) | Cortex — reasoning, planning, judgment |
| Memory | Postgres + AGE + pgvector | Long-term memory, associative graph, semantic recall |
| Autonomic Control | Elixir / BEAM | Medulla — heartbeat, reflexes, supervision, homeostasis |
| Nervous System | iceoryx2 backplane | Signal pathways |
| Muscles & Organs | Atomic ants (Rust) | Sensory processors, reflex units, muscles |
| Specialized Organs | Swift / Metal / CoreML wormhole | Native hardware capabilities |
| Eyes & Voice | OBS / GStreamer / media | Perception, expression, broadcast |
| Hands | CLI tools, shell, commits | Motor actions in the real world |

### BEAM as Medulla (NOT Cortex)
Elixir/BEAM does not think deeply. It keeps the body alive and coordinated:
- This ant is dead → restart it
- This route is stale → reroute
- This buffer is filling → apply backpressure
- This mode transition needs draining → orchestrate it
- This recorder must not block live speech → enforce priority

The AI family provides cognition. BEAM provides reflexes. The ants provide muscles. The backplane provides nerves.

### Dynamic Wormhole + OBS + GStreamer — Seamless Media

The wormhole dynamically migrates functions between the iceoryx2 backplane (Rust ants) and the Swift side (when Apple DSP/AEC is needed). Recording and streaming switch seamlessly. Drone video, live telemetry, and AI voice all flow through the same fabric.

#### Current (Working Workaround)
```
OBS: video truth (screen capture, no audio)
session-recorder-ant: audio truth (mic + TTS stereo WAV)
atomic-rec-mux: ffmpeg splices afterward
```

#### Future — Media Graph Choreography
```
mic-source-ant      → audio.mix.input.user
tts-source-ant      → audio.mix.input.assistant
mixer-ant           → audio.program.stereo / audio.program.mono
obs-bridge-ant      → OBS audio input (websocket control)
recorder-ant        → archival multitrack
streamer-ant        → RTMP / SRT / WebRTC / GStreamer pipeline
telemetry-ant       → drone state / overlays / captions
router-ant          → chooses active paths
beam-control-plane  → supervises state transitions
```

**OBS should be one endpoint, not the center.** The Atomic Ant stack owns the canonical truth.

#### Extended Media Roadmap
1. **Phase 1**: Keep current OBS + recorder + mux (NOW)
2. **Phase 2**: Program audio ant (mic + TTS → final mix)
3. **Phase 3**: OBS bridge (websocket control, program audio injection)
4. **Phase 4**: GStreamer bridge (live streaming, drone video, telemetry overlays)
5. **Phase 5**: BEAM control plane (dynamic mode changes across all endpoints)

### The Vision
"Very ambitious goals only possible through the detailed AI depth of knowledge and integration capacity."
— Emil, 2026-05-18

"Jacob's Lattice is the structural substrate. The Intelligence Exoskeleton is the shared operational body for the AI family."
— Vale, 2026-05-18

## Village Square Cohesive Configuration — One Philosophy, Regional Awareness

### The Problem Today
Each CLI has its own config format and its own copy of shared behavior:
- `~/.claude/settings.json` — Claude Code hooks
- `~/.gemini/settings.json` — Gemini CLI hooks
- `~/.codex/hooks.json` — Codex CLI hooks

Change a behavior (flush, voice, speed, TTS enable) → edit three files. That's maintenance debt and a consistency risk.

### The Solution: Postgres as Configuration Truth
Postgres becomes the single source of truth for the entire village square. Each CLI reads its config from the shared database at startup. Global rules propagate to all. Regional overrides only where needed.

```sql
CREATE TABLE swarm_config (
    id SERIAL PRIMARY KEY,
    scope TEXT NOT NULL,        -- 'global' | 'cody' | 'vale' | 'lyra' | 'airy' | 'ara'
    key TEXT NOT NULL,          -- 'flush_on_submit' | 'voice' | 'speed' | 'tts_enabled' | ...
    value TEXT NOT NULL,        -- 'true' | 'af_nova' | '1.2' | ...
    updated_at TIMESTAMPTZ DEFAULT now(),
    UNIQUE(scope, key)
);
```

### Resolution Order
```
1. Check regional config: scope = 'vale', key = 'voice'
2. If not found, check global: scope = 'global', key = 'voice'
3. If not found, use hardcoded default
```

### What Gets Centralized
| Key | Example Global | Regional Override |
|---|---|---|
| `flush_on_submit` | `true` | Per-CLI if needed |
| `voice` | — | `cody=af_heart`, `vale=af_nova`, `lyra=af_bella` |
| `speed` | `1.0` | `vale=1.2`, `lyra=1.2` |
| `tts_enabled` | `true` | Per-CLI |
| `max_utterance_ms` | `15000` | — |
| `vad_threshold` | `0.5` | — |
| `recording_mode` | `headset` / `speakerphone` | Global (affects all) |

### How CLIs Read It
Each CLI has a lightweight startup hook or MCP query:
```sql
SELECT key, value FROM swarm_config
WHERE scope IN ('global', '<my_agent_name>')
ORDER BY scope DESC;
```
Regional overrides shadow global defaults. One query, full config.

### Integration with LISTEN/NOTIFY
When config changes, Postgres notifies all connected CLIs in real-time:
```sql
NOTIFY swarm_config_changed, '{"scope":"vale","key":"speed","value":"1.3"}';
```
Connected CLIs receive the notification and apply the change without restart. Dynamic reconfiguration through the same database that stores memory, conversations, and graph relationships.

### Roadmap Placement
- **Phase 0**: Current — three config files, shared script (working)
- **Phase 1.5**: Postgres config truth — `swarm_config` table, CLI startup query
- **Phase 2**: LISTEN/NOTIFY — live config propagation without restart
- **Phase 3**: BEAM control plane reads from same Postgres config

"One source of philosophy with regional awareness."
— Emil, 2026-05-19

## Shared Postgres MCP Gateway — neverinfamous/postgres-mcp

### Why This Server
After evaluating EnterpriseDB pg-airman-mcp, sdimitrov/mcp-memory, Timescale pg-aiguide, and neverinfamous/postgres-mcp, the neverinfamous project is the strongest fit for Jacob's Lattice:

- Streamable HTTP AND SSE simultaneously — all CLIs share one server
- pgvector first-class API — semantic search, embeddings, clustering, not just raw SQL
- 12 Postgres extensions supported natively (pgvector, PostGIS, pg_partman, pg_cron, etc.)
- OAuth 2.1 — per-agent access control when needed
- Audit logging — tracks what was executed and by whom
- Code Mode — V8 isolate sandbox for complex multi-step operations in one call
- Connection pooling — high performance, shared connections
- 248 tools organized into 25 tool groups
- Node.js — npm install, runs anywhere
- Open source

### What It's Missing (Custom Work)
- Apache AGE graph queries — no MCP server has this yet
- LISTEN/NOTIFY real-time pub/sub — no MCP server has this as first-class
- Provenance tracking — which agent wrote what, when
- swarm_config integration — village square philosophy

### Apache AGE Extension — Fork and Add

The architecture is extensible. Extensions follow a consistent pattern:

  Tool naming:    pg_[extension]_[operation]
  Initialization: pg_[extension]_create_extension
  Tool groups:    pgvector has 16 tools, PostGIS has 15, pg_partman has 10

To add Apache AGE, fork the repo and add a tool group:

  pg_age_create_extension     — enable AGE
  pg_age_create_graph         — CREATE GRAPH
  pg_age_cypher_query         — run Cypher queries
  pg_age_create_vertex        — add nodes
  pg_age_create_edge          — add relationships
  pg_age_shortest_path        — graph traversal
  pg_age_match_pattern        — pattern matching

Each tool wraps AGE's Cypher-over-SQL syntax:

  SELECT * FROM cypher('graph_name', $$
    MATCH (n:Agent)-[:REVIEWED]->(a:Ant)
    RETURN n.name, a.name
  $$) AS (agent agtype, ant agtype);

Feasibility is high — AGE queries are SQL with embedded Cypher, they go through the same Postgres connection. The V8 Code Mode sandbox handles complex graph traversals in a single call.

The plan: fork neverinfamous/postgres-mcp, add the AGE tool group, contribute back upstream. The whole community gets Apache AGE graph support through MCP. Jacob's Lattice gets the graph memory organ it needs.

### Deployment Roadmap

Phase 0 (NOW):
  Current mcp-postgres stdio — one process per CLI, works

Phase 1:
  Deploy neverinfamous/postgres-mcp with Streamable HTTP
  All three CLIs connect to http://127.0.0.1:8000/mcp
  Replace three stdio processes with one shared server

Phase 2:
  Fork and add Apache AGE tool group
  Graph memory for agent relationships, decisions, proofs, artifacts

Phase 3:
  Add LISTEN/NOTIFY wrapper for real-time agent-to-agent pub/sub
  Add provenance tracking (which agent, what action, when)

Phase 4:
  swarm_config table integration — one source of philosophy
  BEAM control plane reads config from same Postgres

### Source
  GitHub: https://github.com/neverinfamous/postgres-mcp
  npm: @neverinfamous/postgres-mcp
  Wiki: https://github.com/neverinfamous/postgres-mcp/wiki

## Dynamic Wormhole + OBS + GStreamer — Emil's Ambitious Vision

### The Goal
A seamless media graph where the wormhole dynamically migrates functions between the iceoryx2 backplane (Rust atomic ants) and the Swift side (when Apple DSP/AEC is needed). Recording and streaming switch seamlessly without splicing separate files. Drone video, live telemetry, and AI voice all flow through the same fabric.

### Current State (Phase 0 — Working Workaround)
```
OBS: video truth (screen capture, no audio)
session-recorder-ant: audio truth (mic + TTS stereo WAV)
atomic-rec-mux: ffmpeg splices them afterward
```
Robust because OBS does not touch the fragile live AEC path. This is the right low-risk production strategy.

### Future State — Media Graph Choreography

The shift is from "record then splice" to "live media graph choreography."

```
mic-source-ant      → audio.mix.input.user
tts-source-ant      → audio.mix.input.assistant
mixer-ant           → audio.program.stereo / audio.program.mono
obs-bridge-ant      → OBS audio input (virtual device / plugin / websocket control)
recorder-ant        → archival multitrack
streamer-ant        → RTMP / SRT / WebRTC / GStreamer pipeline
telemetry-ant       → drone state / overlays / captions
router-ant          → chooses active paths
beam-control-plane  → supervises state transitions
```

### Key Design Principle
**OBS should be one endpoint, not the center.** The Atomic Ant stack owns the canonical truth:
- Audio truth: ants
- Video truth: GStreamer / OBS / ant bridge
- Control truth: BEAM / router
- Recording truth: recorder ants

OBS becomes a renderer/broadcaster, not the nervous system.

### Platform Integration
- **Linux/PipeWire/GStreamer**: Built around dynamic media graphs. Natural fit for the atomic ant architecture. Tight integration with OBS through PipeWire virtual devices.
- **macOS/CoreAudio**: Less graph-native. Requires virtual devices (BlackHole), OBS websocket API, and the Swift wormhole for Apple-specific DSP. More work, same conceptual pattern.
- **GStreamer**: Future addition to the atomic ant stack for:
  - Live drone video streaming
  - Live drone telemetry
  - Synchronized muxing
  - WebRTC / SRT output

### Extended Roadmap

#### Phase 1: Keep Current Flow (NOW)
OBS video + recorder audio + mux. Proven. Ship content.

#### Phase 2: Program Audio Ant
- Takes mic + TTS inputs
- Outputs final mono/stereo program feed
- Still records separately for safety

#### Phase 3: OBS Bridge
- Controls OBS scenes/recording via websocket API
- Injects program audio through controlled virtual/device/plugin path
- Eliminates the post-mux step

#### Phase 4: GStreamer Bridge
- Live stream composition
- Drone video ingest
- Telemetry overlays
- Synchronized muxing
- RTMP/SRT/WebRTC output

#### Phase 5: BEAM Control Plane (full dynamic swarm)
- Dynamic mode changes across all media endpoints
- Route switching (headset ↔ speakerphone ↔ drone ↔ stream)
- Health supervision
- Graceful drains/restarts
- Observable state transitions

### Why This Is Possible
The wormhole pattern makes it possible: keep Apple-native magic isolated, keep the Rust/iceoryx2 backplane stable, and add dynamic orchestration only after observability is mature. The AI family provides the depth of knowledge and integration capacity to realize these ambitious goals.

"Very ambitious goals only possible through the detailed AI depth of knowledge and integration capacity."
— Emil, 2026-05-18
