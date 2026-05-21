# Remote Launch Guide

**Start Cody from your phone when the laptop is unattended.**

---

## Quick Start

From your phone on GitHub, create a file called `start-cody.json` in the **repo root** and paste one of these:

### Fresh new session
```json
{"mode": "new"}
```

### Resume the most recent session
```json
{"mode": "resume", "session": "last"}
```

### Resume a specific session
```json
{"mode": "resume", "session": "PASTE-SESSION-ID-HERE"}
```
Get the session ID from `cody-sessions.md` in the repo root.

---

## Even Easier

Just create an empty file in the repo root:

- **`.restart`** → fresh new session
- **`.resume`** → resume the most recent session

No JSON needed. Just an empty file and commit.

---

## How to Find Session IDs

Open **`cody-sessions.md`** in the repo root on GitHub. It lists your last 20 sessions with:
- Date
- Project
- First message (so you can tell them apart)
- Session ID (copy this into `start-cody.json`)

---

## What Happens

1. You create the signal file on GitHub and commit
2. The laptop picks it up within ~15 seconds
3. Any existing Cody session gets closed
4. A new Terminal window opens with the right session
5. The signal file gets cleaned up automatically

---

## Ready-Made Templates

In `signal-templates/` you'll find pre-made JSON files. Open one, copy its contents, create `start-cody.json` at the repo root, paste, commit.

| Template | What it does |
|----------|-------------|
| `start-new.json` | Fresh session |
| `start-resume-last.json` | Resume most recent |
| `start-resume-pick.json` | Resume specific (edit the ID) |

---

## Requirements

- Laptop must be powered on and logged in (can be lid-closed or sleeping)
- The sentinel daemon runs automatically on boot
- Works over WiFi — laptop doesn't need to be on the same network as your phone
