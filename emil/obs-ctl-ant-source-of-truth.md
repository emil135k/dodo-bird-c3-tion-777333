# obs-ctl-ant Source of Truth

## Purpose

`obs-ctl-ant` is the OBS control plane daemon for the atomic ant stack.

It is responsible for controlling OBS Studio reliably over WebSocket without acting like a thin wrapper that dies on disconnects or silently guesses state.

This document is the implementation contract. If code conflicts with this file, the code is wrong.

## Non-Goals

`obs-ctl-ant` does not:

- capture mic audio
- mirror TTS audio
- own the realtime audio pipeline
- manage Apple Voice Processing
- replace `obs-mirror-ant`
- pretend OBS is healthy when it is not

Audio routing stays in the ants and mirror path. `obs-ctl-ant` controls OBS itself.

## High-Level Flow

```text
CLIs / ants / BEAM
  -> obs_cmd bus
  -> obs-ctl-ant
  -> OBS WebSocket
  -> OBS Studio

OBS events
  -> obs-ctl-ant
  -> obs_event bus
  -> obs_audit Postgres writes
  -> CLIs / ants / BEAM
```

## Core Contract

`obs-ctl-ant` must:

- own one OBS WebSocket connection
- reconnect automatically on drop
- re-identify after reconnect
- refresh state after reconnect
- emit structured events
- write audit records
- reject invalid commands explicitly
- expose health and readiness
- never claim success without confirming state

## State Machine

```text
BOOTING
  -> CONNECTING
  -> IDENTIFYING
  -> READY

READY
  -> RECONNECTING       (WebSocket closes)
  -> DEGRADED           (OBS unavailable but daemon still alive)
  -> STOPPING           (shutdown requested)

RECONNECTING
  -> CONNECTING         (retry succeeds)
  -> DEGRADED           (retry budget exceeded or state unknown)

DEGRADED
  -> CONNECTING         (retry loop resumes)
  -> FAILED             (fatal config/auth error)

ANY
  -> FAILED             (fatal startup/auth/config failure)
  -> STOPPING           (shutdown path)
```

### State meanings

- `BOOTING`: config load, topic setup, client initialization
- `CONNECTING`: opening OBS WebSocket
- `IDENTIFYING`: OBS hello/identify handshake
- `READY`: fully operational
- `RECONNECTING`: recovering from a dropped websocket
- `DEGRADED`: alive, but not fully trusted
- `STOPPING`: shutdown in progress
- `FAILED`: unrecoverable error

## Message Model

### Command envelope

Every request into `obs-ctl-ant` should be a structured command envelope.

```text
command_id
actor
source
intent
payload
expected_state
timeout_ms
priority
timestamp
```

### Command intents

Minimum supported set:

- `GetStatus`
- `GetScenes`
- `GetInputs`
- `GetSceneItems`
- `SetScene`
- `StartRecording`
- `StopRecording`
- `StartStreaming`
- `StopStreaming`
- `SetInputMute`
- `SetInputVolume`
- `SetInputEnabled`
- `SetInputSettings`
- `ToggleSource`

### Output envelope

```text
command_id
accepted
completed
rejected
failed
state_before
state_after
obs_request
obs_response
retry_count
latency_ms
error_code
error_message
```

## iceoryx2 Topics

The bus should stay narrow and explicit.

```text
obs_cmd
obs_status
obs_event
obs_audit
obs_health
```

### Topic roles

- `obs_cmd`: incoming command envelopes
- `obs_status`: current daemon state snapshot
- `obs_event`: normalized OBS events
- `obs_audit`: durable action log records
- `obs_health`: uptime, reconnect count, last error, readiness

## OBS Event Model

Normalize OBS websocket events into a small internal set:

- `Connected`
- `Disconnected`
- `Reconnected`
- `SceneChanged`
- `RecordingStarted`
- `RecordingStopped`
- `StreamingStarted`
- `StreamingStopped`
- `InputMuted`
- `InputUnmuted`
- `InputVolumeChanged`
- `SourceEnabled`
- `SourceDisabled`
- `Error`

No raw event flood should leak directly into higher layers unless explicitly requested for debugging.

## Reconnect Policy

The current `obs-mcp` failure showed the wrong pattern:

- connect once
- clear state on close
- never reconnect
- silently fail on future requests

`obs-ctl-ant` must do the opposite.

### Retry behavior

Use bounded exponential backoff with jitter:

```text
250ms
500ms
1s
2s
5s
10s
15s max
```

### Recovery sequence

On reconnect:

1. reopen websocket
2. re-identify
3. refresh current OBS state
4. republish readiness
5. emit `Reconnected`
6. replay only safe idempotent state if needed

### Replay rules

Safe to replay:

- scene selection
- input mute state
- input volume state
- source enabled state
- recording/streaming state if explicitly tracked and legal

Do not blindly replay:

- one-shot commands
- stale commands
- actions whose effect is unknown

## Audit Schema

Postgres is the durable log and history layer.

Suggested tables:

```text
obs_actions
obs_events
obs_snapshots
obs_errors
obs_sessions
```

Minimum `obs_actions` columns:

```text
id
command_id
actor
intent
payload_json
result
state_before_json
state_after_json
error_code
error_message
created_at
```

## Module Layout

Recommended Rust layout:

```text
obs-ctl-ant/
  src/
    main.rs
    config.rs
    state.rs
    obs_client.rs
    commands.rs
    events.rs
    bus.rs
    audit.rs
    replay.rs
    health.rs
```

### Responsibilities

- `config.rs`: load URLs, auth, retry policy, bus names, audit config
- `state.rs`: daemon state machine and current OBS snapshot
- `obs_client.rs`: websocket connect/disconnect/request/identify/reconnect
- `commands.rs`: parse and validate command envelopes
- `events.rs`: normalize OBS events into internal event types
- `bus.rs`: iceoryx2 publish/subscribe wiring
- `audit.rs`: Postgres writes for actions, errors, snapshots
- `replay.rs`: safe state recovery after reconnect
- `health.rs`: health/readiness snapshots
- `main.rs`: process startup, shutdown, orchestration

## Function Stubs

### Lifecycle

```text
start()
stop()
connect_obs()
disconnect_obs()
reconnect_loop()
refresh_state()
publish_status()
publish_event()
publish_audit()
health_check()
```

### Command execution

```text
handle_command(cmd)
validate_command(cmd)
execute_command(cmd)
replay_safe_state()
```

### OBS operations

```text
get_scene_list()
get_input_list()
get_scene_items(scene)
set_current_scene(scene)
start_recording()
stop_recording()
start_streaming()
stop_streaming()
set_input_mute(name, muted)
set_input_volume(name, volume)
set_input_enabled(name, enabled)
set_input_settings(name, settings)
```

## Validation Rules

Reject invalid requests instead of guessing.

Examples:

- scene does not exist
- input does not exist
- volume out of policy range
- recording command invalid in current state
- source toggle illegal for current scene
- auth missing or wrong

## Error Policy

Errors must be typed and explicit.

Required error classes:

- `ConfigError`
- `AuthError`
- `ConnectError`
- `IdentifyError`
- `NotReadyError`
- `InvalidCommandError`
- `ObsRequestError`
- `TimeoutError`
- `StateMismatchError`
- `AuditWriteError`

No generic “something went wrong” unless the underlying cause is also captured.

## Implementation Rules

1. One daemon owns the OBS websocket.
2. Do not use shell scripts as the control plane.
3. Do not silently fall back to stale state.
4. Do not claim `READY` if reconnect is broken.
5. Do not bury command semantics in JSON blobs without validation.
6. Every command must either succeed, fail, or enter a recoverable pending state.
7. Every state transition must be observable.
8. Every significant action must be auditable.

## Relationship to obs-mirror-ant

`obs-mirror-ant` stays responsible for audio mirroring into BlackHole.

`obs-ctl-ant` stays responsible for OBS control.

They should communicate through iceoryx2 topics, not by sharing ad hoc shell glue.

Suggested separation:

- `obs-mirror-ant`
  - audio mirror data plane
- `obs-ctl-ant`
  - OBS control plane

## Human Workflow Contract

When editing this system:

- update this doc first
- implement to this doc
- do not patch around broken reconnect logic
- do not replace missing behavior with a hidden workaround

If the daemon cannot prove it is healthy, the system is not healthy.

## Immediate Build Order

1. scaffold the daemon and config loader
2. implement websocket connect/identify
3. implement reconnect and state refresh
4. add command validation and status publishing
5. add audit logging
6. connect iceoryx2 topics
7. add Postgres persistence
8. only then expose CLI wrappers

## Source of Truth

This document is the source of truth for `obs-ctl-ant`.

If the implementation, a wrapper, or a shortcut disagrees with this file, the implementation must be fixed.
