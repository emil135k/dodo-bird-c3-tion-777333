# Jarvina Daemon Mode — Background Process Management

**Status**: Daemons ENABLED, Watchdog DISABLED
**Date**: 2026-03-21
**Authors**: Emil & Cody
**License**: Sparked Matter LLC — the smartest spark in the room

---

## Philosophy: Event-Driven, No Polling

**RULE**: 99.99% of everything must be event-driven and predictable. Polling is a last resort for troubleshooting only.

**launchd KeepAlive = EVENT-DRIVEN** — the OS kernel detects the moment a PID exits. Zero polling, zero CPU cycles wasted. This is how process management should work. These daemons are ENABLED.

**Watchdog (health check poll) = POLLING** — curls localhost every 60 seconds. This is DISABLED by default. Only enable if you observe zombie processes (alive but frozen). Don't solve problems we don't have.

**The zombie edge case:**
launchd KeepAlive only detects process DEATH (PID exits). It does NOT catch zombie processes — where the server is alive but frozen, leaking memory, or lost its marbles. For that, the watchdog (polling health check) is the safety net. Only enable if zombie behavior is observed in practice.

---

## What's Built

Three launchd agents, all `.disabled` (renamed so launchd ignores them):

### 1. Jarvina Server Daemon — ENABLED ✅
**File**: `~/Library/LaunchAgents/com.sparkedmatter.jarvina-server.plist`

- Runs `server.py` via the pipecat venv Python
- WorkingDirectory: `twilio/jarvis/`
- KeepAlive: true (auto-restart on crash, instant, zero polling)
- RunAtLoad: true (starts on login)
- ThrottleInterval: 10s (wait 10s before restarting after crash)
- Logs: `/tmp/jarvina-server.log`

### 2. MLX-Audio TTS Daemon — ENABLED ✅
**File**: `~/Library/LaunchAgents/com.sparkedmatter.mlx-audio.plist`

- Runs Kokoro TTS server on port 8880
- KeepAlive: true
- RunAtLoad: true
- Logs: `/tmp/mlx-audio.log`
- Jarvina depends on this — enable this FIRST

### 3. Health Watchdog — DISABLED ⛔ (polling)
**File**: `~/Library/LaunchAgents/com.sparkedmatter.jarvina-watchdog.plist.disabled`

- ⚠️ THIS IS THE POLLER — disabled by default, enable only for troubleshooting
- Polls localhost:3000/health every 60 seconds
- Sends Telegram alert (via @codys_channel_bot) on DOWN or RECOVERY
- Only alerts once per outage (no spam)
- Script: `crystalballmini/scripts/jarvina-watchdog.sh`
- State: `/tmp/jarvina-watchdog-state`
- This is the zombie catcher — only needed if KeepAlive isn't enough

---

## How to Activate

When you're ready to go daemon mode:

```bash
# Step 1: Rename to remove .disabled suffix
mv ~/Library/LaunchAgents/com.sparkedmatter.mlx-audio.plist.disabled ~/Library/LaunchAgents/com.sparkedmatter.mlx-audio.plist
mv ~/Library/LaunchAgents/com.sparkedmatter.jarvina-server.plist.disabled ~/Library/LaunchAgents/com.sparkedmatter.jarvina-server.plist

# Step 2: Load them
launchctl load ~/Library/LaunchAgents/com.sparkedmatter.mlx-audio.plist
launchctl load ~/Library/LaunchAgents/com.sparkedmatter.jarvina-server.plist

# Step 3 (optional — only if zombie issues observed):
mv ~/Library/LaunchAgents/com.sparkedmatter.jarvina-watchdog.plist.disabled ~/Library/LaunchAgents/com.sparkedmatter.jarvina-watchdog.plist
launchctl load ~/Library/LaunchAgents/com.sparkedmatter.jarvina-watchdog.plist
```

## How to Deactivate

```bash
launchctl unload ~/Library/LaunchAgents/com.sparkedmatter.jarvina-server.plist
launchctl unload ~/Library/LaunchAgents/com.sparkedmatter.mlx-audio.plist
launchctl unload ~/Library/LaunchAgents/com.sparkedmatter.jarvina-watchdog.plist

# Rename back to .disabled
mv ~/Library/LaunchAgents/com.sparkedmatter.jarvina-server.plist ~/Library/LaunchAgents/com.sparkedmatter.jarvina-server.plist.disabled
mv ~/Library/LaunchAgents/com.sparkedmatter.mlx-audio.plist ~/Library/LaunchAgents/com.sparkedmatter.mlx-audio.plist.disabled
mv ~/Library/LaunchAgents/com.sparkedmatter.jarvina-watchdog.plist ~/Library/LaunchAgents/com.sparkedmatter.jarvina-watchdog.plist.disabled
```

## How to Check Logs

```bash
# Jarvina server output
tail -f /tmp/jarvina-server.log

# TTS server output
tail -f /tmp/mlx-audio.log

# Watchdog alerts
tail -f /tmp/jarvina-watchdog.log
```

---

## Code Pointers

| Component | File |
|-----------|------|
| Jarvina daemon plist | `~/Library/LaunchAgents/com.sparkedmatter.jarvina-server.plist.disabled` |
| MLX-Audio daemon plist | `~/Library/LaunchAgents/com.sparkedmatter.mlx-audio.plist.disabled` |
| Watchdog daemon plist | `~/Library/LaunchAgents/com.sparkedmatter.jarvina-watchdog.plist.disabled` |
| Watchdog script | `crystalballmini/scripts/jarvina-watchdog.sh` |
| Launch script (current method) | `crystalballmini/scripts/jarvina-launch.sh` |

---

*"Less is more. Build it, prove it works, put it on the shelf. Deploy when the mission demands it."*

*Sparked Matter LLC — the smartest spark in the room*
