# Fleet Control Plane — Multi-Platform Dashboard & Failover

**Status**: Vision / Earmarked
**Date**: 2026-03-20
**Authors**: Emil & Cody
**License**: Sparked Matter LLC — the smartest spark in the room

---

## The Vision

A unified dashboard — a cockpit — that gives Emil a single glass panel view across all compute nodes. Platform-agnostic deployment so the same Sparked Matter services run anywhere. Automatic failover so the phone number never goes dark.

---

## The Fleet

| Node | Hardware | Role | Strengths |
|------|----------|------|-----------|
| **Mac M1** | MacBook Pro, 8GB | Primary workhorse | Local, fast, sovereign. Runs Jarvina, Cody, voice loop, TTS. |
| **Jetson Orin** | NVIDIA, 8GB, GPU | Edge AI inference | Local LLM (Phi-3/Mistral 7B), Sentry Agent, heavy compute. |
| **Pi 5 (x2)** | Raspberry Pi, 16GB each | Always-on micro-servers | Sentinel, monitoring, gemini-watch, sensor nodes. Light duty. |
| **Pi Zero W (x2)** | Raspberry Pi Zero | Micro endpoints | Lightweight always-on tasks, IoT bridges. |
| **AWS G6** | Cloud GPU instance | Burst / failover / demo | Spin up on demand. GPU scale beyond local hardware. Demo environment. |

---

## Dashboard Requirements

### Metrics Per Node
- **Uptime**: Is it alive? Last heartbeat timestamp.
- **Latency**: Round-trip time from dashboard to node.
- **CPU / Memory / GPU**: Utilization percentages.
- **Active Services**: What's running? (Jarvina, Cody, voice loop, Sentinel, etc.)
- **Network**: Tailscale status, public endpoints, funnel health.
- **Cost**: AWS billing (when active). Local = $0.
- **TTS/STT Latency**: End-to-end voice pipeline timing per node.
- **LLM Response Time**: Time from prompt to first token, by model, by node.

### Comparison View
- Side-by-side: Mac vs Jetson vs AWS for the same workload.
- Latency charts over time.
- Cost-per-inference breakdown.
- "Which node should I run this on?" recommendation engine.

### Alerts
- Node goes offline → Telegram notification via Jarvina.
- Latency exceeds threshold → flag it.
- AWS spend exceeds budget → warn.
- Service crash → auto-restart or failover trigger.

---

## Platform-Agnostic Deployment

The same Jarvina/Sparked Matter stack runs on any node. How:

### Containerized Option
- Docker image with: Python venv, server.py, Deepgram STT, ElevenLabs TTS, Claude API
- Same image runs on Mac (Docker Desktop), Jetson (NVIDIA Container Runtime), AWS (ECS/EC2)
- Config via .env file — swap TTS_ENGINE, LLM endpoint, etc. per node

### Bare Metal Option
- Python venv + requirements.txt — identical on all platforms
- Platform-specific TTS: Kokoro (Mac/Jetson, local), ElevenLabs (AWS, cloud)
- Platform-specific STT: Deepgram (all), Moonshine (Mac/Jetson, local fallback)
- Same server.py, same logic, different .env

### Sync Layer
- **GitHub**: Code sync, config sync, contacts.json, session state.
- **Tailscale**: Private network between all nodes. Secure, NAT-traversing.
- **Shared State**: SQLite DB synced via Litestream to S3, or Redis on Tailscale.
- **Alternative**: CRDTs for conflict-free replication across nodes.

---

## Failover Architecture

```
                    ┌──────────────┐
                    │  Twilio      │
                    │  Phone #     │
                    │  +1(813)607  │
                    └──────┬───────┘
                           │
                    ┌──────▼───────┐
                    │  Tailscale   │
                    │  Funnel /    │
                    │  Load Balancer│
                    └──────┬───────┘
                           │
              ┌────────────┼────────────┐
              │            │            │
       ┌──────▼──┐  ┌──────▼──┐  ┌─────▼───┐
       │  Mac M1  │  │ Jetson  │  │  AWS G6  │
       │ PRIMARY  │  │ STANDBY │  │ ON-DEMAND│
       └─────────┘  └─────────┘  └──────────┘
```

### Failover Logic
1. **Mac M1** is primary. Handles all calls.
2. Health check every 30s from Pi 5 (watchdog).
3. Mac goes down → Pi detects within 60s.
4. Pi triggers Jetson to take over (wake + start services).
5. Tailscale Funnel re-routes to Jetson's endpoint.
6. If Jetson can't handle it → Pi spins up AWS G6 instance via CLI.
7. AWS takes over. Syncs latest state from GitHub.
8. Mac comes back → Pi detects, traffic shifts back. AWS instance terminated.

### State Continuity
- contacts.json synced to all nodes via GitHub.
- Call history / session state in shared SQLite (Litestream → S3).
- .env secrets in Tailscale-internal vault or encrypted GitHub repo.
- Active call handoff: Twilio conference bridge during transition (no dropped calls).

---

## Technology Candidates

### Dashboard UI
- **Grafana**: Open source, beautiful, plugin ecosystem. Runs on Pi.
- **Uptime Kuma**: Lightweight monitoring, perfect for small fleet.
- **Custom PWA**: Crystal Ball Mini style — lightweight, mobile-friendly.
- **Dozzle**: Docker log viewer (if containerized).

### Metrics Collection
- **Telegraf + InfluxDB**: Standard metrics pipeline. Lightweight agent on each node.
- **Prometheus + node_exporter**: Pull-based. Good for heterogeneous fleet.
- **Custom heartbeat**: Simple JSON POST every 30s to a central endpoint. Minimal.

### Orchestration
- **Tailscale + SSH**: Simple. Each node reachable via Tailscale hostname.
- **Ansible**: Playbooks for deployment, config sync, service management.
- **Custom scripts**: Shell scripts triggered by Sentinel / Pi watchdog.

### Gemini's Suggestions (from Emil's brainstorm)
- Emil had an extensive conversation with Gemini about AWS deployment
- Mentioned specific AWS tools for monitoring and dashboards
- CrewAI for subagent orchestration (researched previously)
- Need to capture those specific tool recommendations

---

## Build Phases

### Phase 1 — Heartbeat & Health (Now)
- Simple health check endpoint on Mac's Jarvina server: `/health`
- Pi 5 cron job: curl health endpoint every 60s, log to file
- Telegram alert if health check fails 3x consecutive
- **No dashboard yet** — just the watchdog

### Phase 2 — Metrics Collection (Next)
- Telegraf agent on Mac + Jetson + Pi
- InfluxDB on Pi 5 (it has 16GB RAM, plenty)
- Basic Grafana dashboard on Pi: uptime, latency, CPU/mem
- Compare Mac vs Jetson voice pipeline latency

### Phase 3 — Platform Parity (Build)
- Dockerize Jarvina stack (or standardize venv + requirements.txt)
- Test identical deployment on Mac, Jetson, AWS
- Benchmark: same call flow on each platform, measure end-to-end
- Document performance characteristics per node

### Phase 4 — Failover (Harden)
- Pi watchdog triggers Jetson startup on Mac failure
- Tailscale Funnel re-routing (may need API automation)
- AWS auto-provision via CLI from Pi
- State sync verification before traffic switch
- Test: pull Mac's network cable, verify Jetson takes over

### Phase 5 — Production Dashboard (Polish)
- Full Grafana or custom PWA dashboard
- Cost tracking (AWS billing API integration)
- Historical trends: is latency getting worse? Are we outgrowing a node?
- "Deploy to..." button: push new code to any node from dashboard
- Emil's cockpit view from phone/tablet

---

## The Spreadsheet Philosophy

- **Data layer**: Telegraf/heartbeat agents (raw metrics, immutable)
- **Storage layer**: InfluxDB/SQLite (time-series, queryable)
- **Logic layer**: Alert rules, failover triggers (deterministic)
- **Presentation layer**: Grafana/PWA (visual, configurable)
- **Delivery layer**: Telegram/Jarvina voice (push notifications)

Change one layer, others keep working. Swap Grafana for custom UI, metrics still flow. Add a new node, just install the agent. That's the Vernitron way.

---

## Why This Matters

One phone number. One brand. Zero downtime.

Sparked Matter's customer calls a number and gets Jarvina — whether she's running on Emil's Mac in the Hawk, on a Jetson bolted to a truck wall, or on an AWS instance in Virginia. They don't know. They don't care. It just works.

That's the virtual office in a phone number, backed by a sovereign fleet that Emil controls from anywhere.

---

*Sparked Matter LLC — the smartest spark in the room*
*We teach your matter new tricks.*

---

*"Redundancy isn't waste. It's respect for the mission."*
