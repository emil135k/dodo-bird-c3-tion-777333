# QML Dashboard — Sovereign PBX Control Panel

## Overview

Two dashboards exist for the Sovereign PBX:

1. **Web Dashboard** (`dashboard/server.js`) — Node.js PWA, currently in repo and working
2. **QML Dashboard** (`dashboard/sovereign.qml`) — Qt 6.11 native app, built 2026-04-14, **never committed to git** (lost)

This document covers both.

---

## Web Dashboard (Node.js — ACTIVE)

### Files
| File | Purpose |
|------|---------|
| `dashboard/server.js` | HTTP server + embedded HTML/CSS/JS (416 lines) |
| `dashboard/config.json` | Service definitions, Tailscale/Telegram config |

### What It Does
- **Service monitor**: Shows Jarvina Voice Server, Kokoro TTS, Health Watchdog
- **LED status lights**: Green (healthy), Red (down), Grey (disabled) with pulse animations
- **Toggle switches**: Enable/disable services via launchd (load/unload plist)
- **Infrastructure panel**: Tailscale hostname, Funnel port, Telegram bots
- **Auto-refresh**: Polls `/api/status` every 10 seconds
- **PWA**: Installable on phone via Safari "Add to Home Screen"

### API Endpoints
| Method | Path | Action |
|--------|------|--------|
| GET | `/` | Serve HTML control panel |
| GET | `/manifest.json` | PWA manifest |
| GET | `/api/status` | JSON: all services + statuses + config |
| POST | `/api/toggle` | `{ id, enable }` — load/unload launchd service |

### How to Start
```bash
node ~/crystalballmini/dashboard/server.js
# → Sparked Matter Control Panel running on http://localhost:3001
# → Access from phone via Tailscale: http://emils-macbook-pro:3001
```

### How to Stop
```bash
# Find and kill the process
kill $(lsof -ti:3001)
# OR
pkill -f "node.*dashboard/server.js"
```

### Access from Phone
- Same network: `http://emils-macbook-pro.local:3001`
- Tailscale: `http://emils-macbook-pro:3001`
- The page is mobile-optimized (viewport meta, touch-friendly toggles)

### Config Format (`config.json`)
```json
{
  "services": {
    "jarvina-server": {
      "name": "Jarvina Voice Server",
      "plist": "com.sparkedmatter.jarvina-server",
      "port": 3000,
      "healthUrl": "http://localhost:3000/health",
      "enabled": true
    }
  },
  "tailscale": { "hostname": "...", "funnelPort": 3000 },
  "telegram": { "channelBot": "...", "voiceBot": "..." }
}
```

Adding a new service: add an entry to `services` with `name`, `plist` (launchd label), `port`, `healthUrl`, `enabled`. The dashboard auto-discovers from config.

---

## QML Dashboard (Qt 6.11 — NEEDS REBUILD)

**Status**: Built 2026-04-14, tested live, never committed. The file `dashboard/sovereign.qml` was overwritten when the web dashboard was committed. Needs to be recreated from the journal reconstruction or from the JSONL session data.

### What It Was
- Native Qt Quick app, dark theme (#1a1a2e), 202 lines of QML
- Title: "SOVEREIGN PBX — Jarvina — Four Ants, One BEAM"
- Talked to BEAM via HTTP on localhost:5050

### Features (as built)
- **Brain selector**: LOCAL (Ollama/nemotron-nano), CLOUD (Haiku API), AUTO (smart fallback)
- **Voice controls**: TALK button (starts mic capture via VoiceLoop GenServer), CALL button (Twilio outbound)
- **Status display**: Ears engine, Cortex mode, voice state (idle/talking/calling)
- **Color coding**: Green = local, Red = cloud, Yellow = auto
- **Refresh button**: Manual poll of `/api/status`

### Dependencies
| Component | Location | Version |
|-----------|----------|---------|
| Qt runtime | `~/Qt/6.11.0/bin/qml` | 6.11.0 |
| Qt libs | `~/Qt/6.11.0/lib/` | 6.11.0 |
| QML modules | `~/Qt/6.11.0/qml/` | QtQuick, QtQuick.Controls, QtQuick.Layouts |

### Qt Installation (how it was done)
1. Emil downloaded 4 component `.7z` files on iPhone (unlimited data)
2. AirDropped to Mac → `~/Downloads/`
3. Extracted with `7z` (p7zip via Homebrew):
```bash
brew install p7zip
mkdir -p ~/Qt/6.11.0
cd ~/Downloads
7z x -o"$HOME/Qt/6.11.0" -y "6.11.0-*qtbase-MacOS-*.7z"
7z x -o"$HOME/Qt/6.11.0" -y "6.11.0-*qtdeclarative-MacOS-*.7z"
7z x -o"$HOME/Qt/6.11.0" -y "6.11.0-*qtsvg-MacOS-*.7z"
7z x -o"$HOME/Qt/6.11.0" -y "6.11.0-*qttools-MacOS-*.7z"
```
4. Verify: `~/Qt/6.11.0/bin/qml --version` → `Qml Runtime 6.11.0`

### How to Start (when rebuilt)
```bash
# 1. Start the BEAM server (port 5050)
cd ~/crystalballmini/mac-pbx
ANTHROPIC_API_KEY="..." mix run --no-halt &

# 2. Launch the QML dashboard
~/Qt/6.11.0/bin/qml ~/crystalballmini/dashboard/sovereign.qml &
```

### How to Stop
```bash
# Stop QML dashboard
pkill -f "qml.*sovereign"

# Stop BEAM server
kill $(lsof -ti:5050)
```

### BEAM API Endpoints (were in router.ex, now removed)
These need to be re-added to `mac-pbx/lib/mac_pbx/router.ex`:
| Method | Path | Action |
|--------|------|--------|
| GET | `/api/status` | JSON: ears engine, cortex mode, larynx voices, devices |
| POST | `/api/mode/local` | Switch Cortex to Ollama |
| POST | `/api/mode/cloud` | Switch Cortex to Haiku |
| POST | `/api/mode/auto` | Switch Cortex to auto-fallback |
| POST | `/api/talk` | Start VoiceLoop GenServer (mic capture) |
| POST | `/api/stop` | Stop VoiceLoop |
| POST | `/api/call` | Twilio outbound (was stub) |
| POST | `/api/hangup` | Hang up (was stub) |

### Supporting Elixir Modules (also need rebuild)
- `MacPbx.VoiceLoop` — GenServer: mic → Ears → Cortex → Larynx → speaker
- `Jarvina.Cortex` — GenServer: LLM routing (local/cloud/auto modes)
- `Patchbay.Native.start_capture/1` — NIF: cpal mic capture to ring buffer
- `Patchbay.Native.read_samples/2` — NIF: drain ring buffer (was returning empty — WIP bug)

---

## What Needs to Happen to Restore QML Dashboard

1. Recreate `dashboard/sovereign.qml` (full source is in journal at `[2026-04-14 ~21:00]`)
2. Re-add dashboard API endpoints to `router.ex`
3. Rebuild `VoiceLoop` and `Cortex` GenServers
4. Fix `read_samples()` ring buffer drain bug
5. Commit everything this time

---

*Last updated: 2026-04-16*
*Sparked Matter LLC — Cathedral v1*
