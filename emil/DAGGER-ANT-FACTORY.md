# Dagger Ant Factory — hypAiAssist

## What It Is

The Sovereign Ant Factory is a Dagger.io module that builds, verifies, and deploys ants in hermetic containers. No local contamination possible — the build happens inside a clean Rust container with NO Python, NO sherpa, NO local libraries.

This is the machine that builds the machines. Self-building ants.

## Location

```
hypAiAssist/
├── .dagger/main.go          ← Factory code
├── dagger.json               ← Dagger module config
├── Sovereign.toml             ← Ant manifest
└── ants/
    ├── tts-ant/               ← TTS Ant source
    ├── audio-sink-ant/        ← Audio Sink Ant source
    ├── pulse/                 ← Pulse test tool source
    └── stt-ant/               ← STT Ant source (next)
```

## Commands

### Build a single ant
```bash
DOCKER_HOST="unix://$(podman machine inspect --format '{{.ConnectionInfo.PodmanSocket.Path}}')" \
  dagger call build-ant --name tts-ant
```

### Build all ants
```bash
DOCKER_HOST="unix://$(podman machine inspect --format '{{.ConnectionInfo.PodmanSocket.Path}}')" \
  dagger call build-all
```

### Deploy an ant (build + copy to ~/.local/bin/)
```bash
DOCKER_HOST="unix://$(podman machine inspect --format '{{.ConnectionInfo.PodmanSocket.Path}}')" \
  dagger call deploy --name tts-ant
```

### Generate launchd plist for a daemon ant
```bash
DOCKER_HOST="unix://$(podman machine inspect --format '{{.ConnectionInfo.PodmanSocket.Path}}')" \
  dagger call generate-plist --name tts-ant
```

### Verify zero contamination
```bash
DOCKER_HOST="unix://$(podman machine inspect --format '{{.ConnectionInfo.PodmanSocket.Path}}')" \
  dagger call verify --name tts-ant
```

## The "No-Liar" Guarantee

The factory builds inside a `rust:1.82-slim-bookworm` container. This container has:
- Rust toolchain
- System headers (pkg-config, cmake, clang, ALSA)
- Nothing else

It does NOT have:
- Python
- sherpa-onnx
- Any local Mac libraries
- Any local venvs

If the ant compiles in the factory, it's clean. Period.

## The Verify Function

`dagger call verify --name tts-ant` runs `strings` on the compiled binary and counts references to:
- `morsel` (must be 0)
- `python3/pip/venv` (must be 0)
- `sherpa-onnx/k2-fsa` (must be 0)
- `write_wav/File::create` (must be 0)

If any count is non-zero, the ant is contaminated.

## Podman Integration

Dagger uses Podman as the container runtime (not Docker). Set the socket:
```bash
export DOCKER_HOST="unix://$(podman machine inspect --format '{{.ConnectionInfo.PodmanSocket.Path}}')"
```

Or add to `~/.bashrc`:
```bash
alias dagger-run='DOCKER_HOST="unix://$(podman machine inspect --format '\''{{.ConnectionInfo.PodmanSocket.Path}}'\'')" dagger'
```

## Future: Self-Building Ants

The factory reads `Sovereign.toml` for ant configuration. When a new ant is added:
1. Create source at `ants/<name>/`
2. Add entry to `Sovereign.toml`
3. Run `dagger call build-ant --name <name>`

The factory handles everything — compilation, verification, deployment. The ant is born clean.

---

*Built by Emil, Cody & Lyra — Sparked Matter LLC, April 28, 2026*
*The machine that builds the machines.*
