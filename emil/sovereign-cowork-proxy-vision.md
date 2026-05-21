# Sovereign Cowork Proxy — Cowork on Steroids

**Status**: Vision / Earmarked
**Date**: 2026-03-20
**Authors**: Emil & Cody
**License**: Sparked Matter LLC — the smartest spark in the room

---

## The Spark

Anthropic's Cowork runs a sandboxed Linux VM on your Mac with training wheels and guardrails. What if we built our OWN Linux VM — no sandbox, no restrictions — with Claude Code behind it? A communication proxy between Emil's phone and the entire sovereign stack.

**Their Cowork**: sandbox → limited tools → Anthropic decides what you can do.
**Your Cowork**: proxy VM → Claude Code → unlimited → YOU decide what gets built.

---

## The Architecture

```
┌──────────────┐
│  Emil's Phone │
│  (anywhere)   │
└──────┬───────┘
       │ Tailscale
       ▼
┌──────────────────────────────────────────┐
│          LINUX VM (Communication Proxy)    │
│                                           │
│  One door in. Routes to everything.       │
│                                           │
│  ┌─────────────┐  ┌──────────────┐       │
│  │ Auth/Routing │  │ Message Queue │       │
│  └──────┬──────┘  └──────┬───────┘       │
│         │                │                │
│         ▼                ▼                │
│  ┌─────────────────────────────────┐     │
│  │         Service Router           │     │
│  └──┬──────┬──────┬──────┬──────┬──┘     │
│     │      │      │      │      │        │
│     ▼      ▼      ▼      ▼      ▼        │
└─────┼──────┼──────┼──────┼──────┼────────┘
      │      │      │      │      │
      ▼      ▼      ▼      ▼      ▼
   Claude  Jarvina Email  Telegram GitHub
   Code    Voice   Graph  Bridge   API
           Server  API
```

---

## What the Proxy Does

### Single Point of Entry
- Phone connects to ONE endpoint via Tailscale
- No need for separate bridges (Telegram, Sentinel, Dispatch)
- One door in, everything behind it

### Intelligent Routing
- "Call mom" → routes to Jarvina voice server
- "Send email to Kirk" → routes to Graph API email worker
- "Hey Cody, build X" → routes to Claude Code session
- "Check Gemini" → routes to Gemini CLI
- "Status" → returns health of all services

### Message Queuing
- If Claude Code is busy → queue the message, process when ready
- If Jarvina is down → tell Emil, offer alternatives
- FIFO ordering — nothing gets lost
- Persistent queue (survives VM restart)

### Authentication & Security
- Tailscale handles encrypted tunnel
- VM adds auth layer (passphrase, trusted devices)
- Rate limiting, abuse prevention
- Audit log of every command routed

---

## Why a VM Instead of Bare Metal

| Benefit | Why It Matters |
|---------|---------------|
| **Isolation** | VM crash doesn't take down the Mac. Services stay separated. |
| **Portability** | Snapshot the VM, move it to Jetson, Pi 5, AWS. Same image everywhere. |
| **Fleet parity** | Same VM image on Mac (dev), Jetson (edge), AWS (cloud). Fleet control plane. |
| **Reproducibility** | Blow it away and rebuild. Deterministic. No "works on my machine." |
| **Security** | Services can't touch Emil's Mac files unless explicitly mounted. |

---

## Technology Stack

### VM Runtime
- **UTM** — Free, native Apple Silicon virtualization. Fast, lightweight.
- **Lima** — Alternative, CLI-based, good for headless server VMs.
- **Docker** — Lighter option if full VM is overkill for some services.

### Inside the VM
- **Ubuntu Server 24.04** — ARM64, minimal, stable
- **Tailscale** — Private network, reachable from phone
- **Nginx/Caddy** — Reverse proxy, routes requests to services
- **Redis** — Message queue, service coordination
- **Supervisor/systemd** — Process management for all services
- **SSH** — Claude Code manages the VM from the Mac

### Communication Channels
- **Telegram Bot API** — Emil sends messages from phone
- **Tailscale direct** — Phone-to-VM API calls
- **WebSocket** — Real-time bidirectional (future: custom app)

### Behind the Proxy
- **Claude Code** — The engine. Full system access via SSH from VM to Mac.
- **Jarvina Voice Server** — Phone calls via Twilio
- **Email Worker** — Graph API, m365 CLI
- **Gemini CLI** — Lyra access
- **Sentinel** — Wake/signal system
- **Sentry Agent** — Audit watchdog (local LLM)

---

## How It Compares

| Feature | Anthropic Cowork | Sovereign Cowork Proxy |
|---------|-----------------|----------------------|
| Environment | Sandboxed Linux VM | Full Linux VM, no sandbox |
| AI behind it | Claude (sandboxed) | Claude Code (full access) |
| File access | Granted folders only | Anything mounted |
| Network | Restricted proxy | Full Tailscale mesh |
| Phone access | Dispatch (one thread) | Multi-channel (Telegram, API, WebSocket) |
| Services | Document tools | Jarvina, email, voice, anything |
| Portability | Mac only | Mac, Jetson, Pi, AWS — same image |
| Control | Anthropic decides | Emil decides |
| Cost | Max subscription | Free (self-hosted VM) |

---

## Connection to Other Visions

- **Fleet Control Plane** — The VM image IS the deployable unit. Same image, any hardware.
- **Sovereign Voice Appliance** — Jarvina runs inside the VM. Package and ship.
- **Sentry Agent** — Runs inside the VM, audits everything passing through the proxy.
- **Lattice Mailbox** — The proxy IS the mailbox. Airy drops a message → proxy routes to Cody.
- **ChromaDB** — Vector memory runs in the VM. All services share it.
- **Hyper Lattice** — The VM is the hub node. Phone is the remote. Claude Code is the engine.

---

## Build Phases

### Phase 1 — VM + SSH Bridge (Proof of Concept)
- UTM Linux VM on Mac
- Tailscale installed in VM
- SSH bridge: Claude Code on Mac ↔ VM
- Phone can reach VM via Tailscale
- Simple API: phone sends text → VM forwards to Claude Code session

### Phase 2 — Service Router
- Nginx/Caddy reverse proxy
- Redis message queue
- Route commands to appropriate services
- Health checks and status endpoint
- Telegram bot integration (reuse existing bridge)

### Phase 3 — Full Service Migration
- Move Jarvina voice server into VM
- Move email worker into VM
- Move Telegram bridge into VM
- Claude Code stays on Mac (needs native access)
- VM handles all I/O, Mac handles all compute

### Phase 4 — Fleet Deployment
- Snapshot VM image
- Deploy to Jetson Orin (ARM64 — same architecture as Mac)
- Deploy to AWS (ARM64 Graviton or x86 with QEMU)
- Fleet control plane manages all instances
- Failover: Mac VM down → Jetson VM takes over → AWS as last resort

---

## The Spreadsheet Philosophy

- **Data layer**: Message queue (Redis) — raw commands, immutable
- **Routing layer**: Nginx/service router — deterministic dispatch
- **Compute layer**: Claude Code + local LLMs — the brains
- **Delivery layer**: Telegram, voice, email — output channels
- **Audit layer**: Sentry Agent — watches everything

Change one layer, others keep working. Swap Redis for RabbitMQ, routing still works. Add a new service, just register it with the router.

---

## Why This Matters

This is the virtual office in a phone. Not a phone number — a phone. One device, one connection, everything behind it. Emil walks Dakota, taps his phone, and the entire sovereign stack responds. No Anthropic middleman deciding what's allowed. No sandbox. No training wheels.

**Cowork on steroids. Because behind the VM, it's Cody.**

---

*Sparked Matter LLC — the smartest spark in the room*
*We teach your matter new tricks.*

---

*"They built a sandbox. We built a launchpad."*
