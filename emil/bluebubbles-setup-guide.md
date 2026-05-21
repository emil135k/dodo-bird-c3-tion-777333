# BlueBubbles Setup Guide — Sovereign iMessage

*Fuck Twilio. iMessage through your own Mac. Zero monthly fees.*
*Emil Rivas — FCWD — 2026-04-23*

---

## What Is BlueBubbles

Open-source server that runs on your Mac and exposes iMessage through a REST API. Send/receive texts programmatically. No Twilio, no carrier fees, no texting restrictions. Your Mac IS the server.

## Prerequisites

- macOS with Messages.app signed into iMessage (you have this)
- Tailscale running (you have this)
- BlueBubbles server app (just installed to /Applications)

## Step-by-Step Setup

### Step 1: Launch BlueBubbles
```
open /Applications/BlueBubbles.app
```

### Step 2: Grant Permissions
BlueBubbles needs several macOS permissions. The app will walk you through each one:

1. **Accessibility** — System Settings → Privacy & Security → Accessibility → Enable BlueBubbles
2. **Full Disk Access** — System Settings → Privacy & Security → Full Disk Access → Enable BlueBubbles
3. **Notifications** — Allow when prompted
4. **Contacts** — Allow when prompted (optional, for name resolution)

### Step 3: Firebase Setup (for push notifications)
The app will ask you to connect a Google account for Firebase:
- Click "Continue with Google"
- Sign in with any Google account
- This provisions a free Firebase project for push notifications
- If you don't want push notifications, you can skip this (API still works)

### Step 4: Connection Method
BlueBubbles offers several proxy options:
- **Cloudflare** (recommended by BlueBubbles) — free tunnel
- **Ngrok** — alternative tunnel
- **Tailscale** (recommended for US) — you already have this running!

**For Tailscale (best option):**
- Skip the proxy setup entirely
- BlueBubbles runs its API server on localhost (default port 1234)
- Access it via your Tailscale IP: http://100.79.111.106:1234
- Already encrypted, already connected, zero extra setup

### Step 5: Set Server Password
- In BlueBubbles settings, set a strong password
- This password is used for API authentication
- Save it — you'll need it for API calls

### Step 6: Verify API Works
Once the server is running:
```bash
# Test from your Mac
curl -s http://localhost:1234/api/v1/ping?password=YOUR_PASSWORD

# Test from any Tailscale device
curl -s http://100.79.111.106:1234/api/v1/ping?password=YOUR_PASSWORD
```

## API Endpoints (The Good Stuff)

### Send a text message
```bash
curl -X POST http://localhost:1234/api/v1/message/text \
  -H "Content-Type: application/json" \
  -d '{
    "chatGuid": "iMessage;-;+1XXXXXXXXXX",
    "tempGuid": "temp-123",
    "message": "Hello from BlueBubbles!",
    "method": "apple-script",
    "password": "YOUR_PASSWORD"
  }'
```

### Get recent messages
```bash
curl -s "http://localhost:1234/api/v1/message?limit=10&password=YOUR_PASSWORD"
```

### List chats
```bash
curl -s "http://localhost:1234/api/v1/chat?limit=25&password=YOUR_PASSWORD"
```

### Get messages from a specific chat
```bash
curl -s "http://localhost:1234/api/v1/chat/iMessage;-;+1XXXXXXXXXX/message?limit=20&password=YOUR_PASSWORD"
```

## Gemma 4 Integration (The Sovereign Bot)

Once BlueBubbles API is confirmed working, wire it to Gemma 4:

```
Incoming iMessage → BlueBubbles API → poll for new messages
→ extract text → Ollama Gemma 4 (local LLM)
→ generate reply → BlueBubbles API → send iMessage reply
```

No Twilio. No cloud LLM. No monthly fees. Pure sovereign.

## Architecture

```
iMessage (Apple servers)
    ↕
Messages.app (your Mac)
    ↕
BlueBubbles Server (localhost:1234)
    ↕
REST API (password protected)
    ↕
Gemma 4 via Ollama (local, sovereign)
```

## Tailscale Bonus
Since BlueBubbles runs on your Mac and Tailscale connects all your devices:
- Access from your iPhone via Tailscale
- Access from the Pi 5 or Jetson
- Access from your dad's laptop (100.77.94.72)
- All encrypted, all peer-to-peer, zero port forwarding

## Files
- App: /Applications/BlueBubbles.app
- Version: 1.9.9 (ARM64)
- Config: stored in ~/Library/Application Support/BlueBubbles/

---

*Built FCWD — From Camper With Dog*
*Sparked Matter LLC — The Cathedral Builds Itself*
