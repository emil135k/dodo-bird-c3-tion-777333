# BEAM Ant Supervisor Architecture

Canonical doc: `/Users/rocketman/crystalballmini/emil/beam-ant-supervisor-architecture.md`

## Purpose

Replace fragile one-shot swarm shell scripts with a small Elixir/BEAM sidecar that supervises the Atomic Ant processes while keeping realtime audio and fast data movement inside Rust, Swift, and iceoryx2.

This is not a rewrite of the ants. It is a supervision layer.

## Core Rule

```text
launchd starts BEAM.
BEAM supervises ants.
Rust/Swift move realtime audio.
iceoryx2 carries fast data.
```

Do not put audio frames through BEAM in this phase.

## Why This Exists

The current shell-script model is brittle:

- startup order is encoded as sleeps
- crashes can be silent
- restarts are manual
- status is scattered across `/tmp/*-stdout.log`
- ants can drift into inconsistent states
- launch behavior is hard to reason about

BEAM gives us a natural supervision tree without making each ant heavy.

## Phase 1 Scope

Implemented scaffold:

```text
hypAiAssist/beam/ant_supervisor/
```

Launch scripts:

```text
hypAiAssist/scripts/start-beam-ant-supervisor.sh
hypAiAssist/scripts/install-beam-ant-supervisor-launchagent.sh
hypAiAssist/scripts/uninstall-beam-ant-supervisor-launchagent.sh
```

LaunchAgent template:

```text
hypAiAssist/launchd/com.atomic.ant-supervisor.plist
```

Phase 1 responsibilities:

- detect ant processes
- start/stop the core swarm
- preserve startup order
- write status to `/tmp/atomic-ant-supervisor.status`
- keep log paths stable
- keep STT debug capture enabled
- leave realtime audio untouched

## Current Core Ant Catalog

```text
speaker-ctl-ant
tts-ant
stt-ant
silero-ant
patchbay-ant
router-ant
type-ant
```

Not yet included by default:

```text
obs-ctl-ant
obs-mirror-ant
llm-ant
digi-ant
phone-silero-ant
web-ant
```

Those should be added as optional groups after the core sidecar proves stable.

## Manual Commands

From:

```bash
/Users/rocketman/crystalballmini/hypAiAssist/beam/ant_supervisor
```

Compile:

```bash
mix compile
```

Read status without taking ownership:

```bash
mix ant.status
```

Start core swarm under BEAM:

```bash
mix ant.start
```

Stop core swarm:

```bash
mix ant.stop
```

Run supervisor continuously:

```bash
ANT_SUPERVISOR_AUTOSTART=1 mix run --no-halt
```

## launchd Role

`launchd` should only bootstrap BEAM:

```text
launchd
  -> start-beam-ant-supervisor.sh
    -> mix run --no-halt
      -> AntSupervisor.Manager
        -> ants
```

Do not create one LaunchAgent per ant unless there is a specific reason. That recreates the same fragmented process-control problem.

Install:

```bash
hypAiAssist/scripts/install-beam-ant-supervisor-launchagent.sh
```

Uninstall:

```bash
hypAiAssist/scripts/uninstall-beam-ant-supervisor-launchagent.sh
```

## Future Direction

Phase 2:

- add optional ant groups: OBS, LLM, Twilio
- add restart policies per ant
- add `/tmp` or Unix-socket status API
- add structured JSON status output
- add heartbeat timestamps from ant stubs

Phase 3:

- add iceoryx2 status/control topics
- add per-ant harness commands
- connect BEAM to those control surfaces
- avoid parsing logs as the primary status mechanism

Phase 4:

- BEAM dashboard / local web UI
- Postgres/Jacob's Lattice event sink
- supervisor event history
- per-ant certification status

## Non-Negotiables

- BEAM does not touch realtime audio frames in Phase 1.
- Ants remain independently runnable.
- Shell scripts become thin wrappers, not the architecture.
- launchd starts BEAM; BEAM owns swarm lifecycle.
- Every supervised action must be visible in logs or status output.
