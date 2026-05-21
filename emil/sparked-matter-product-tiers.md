# Sparked Matter — Product Deployment Tiers

**Status**: Reference / Product Planning
**Date**: 2026-03-21
**Authors**: Emil & Cody
**License**: Sparked Matter LLC — the smartest spark in the room

---

## The Product

A virtual office in a phone number. AI voice assistant (Jarvina) handles calls, transfers, dispatch, email — all from one number. Deployable on any hardware tier, from a cheap laptop to a sovereign edge appliance.

---

## Deployment Tiers

### Tier 1 — Full Cloud (Entry Level)
**Hardware**: Any machine with internet ($200 laptop, existing PC, anything)
**Target**: Quick business deployment, no technical expertise needed

| Component | Solution | Cost |
|-----------|----------|------|
| Brain | Claude API (Haiku) | ~$20/mo or Max subscription |
| STT | Deepgram (cloud) | ~$25/mo |
| TTS | ElevenLabs (cloud) | ~$22/mo |
| Phone | Twilio | ~$20/mo |
| Claude Code | Runs on any OS | Free (included in subscription) |
| **Total** | | **~$80-100/mo** |

**Pros**: Works everywhere, zero GPU needed, fastest to deploy
**Cons**: Cloud-dependent, per-minute API costs, latency from cloud round-trips
**Best for**: Demos, startups, businesses wanting to try before investing in hardware

---

### Tier 2 — Mac Mini Appliance (Sweet Spot)
**Hardware**: Mac Mini M4 ($599), ships to customer, plug and play
**Target**: Small business wanting sovereign voice with minimal cloud dependency

| Component | Solution | Cost |
|-----------|----------|------|
| Brain | Claude API (Haiku) | Max subscription |
| STT | Moonshine (local, Metal) | Free |
| TTS | Kokoro (local, Metal, af_heart) | Free |
| Phone | Twilio | ~$20/mo |
| Claude Code | Local on Mac | Included |
| **Total** | | **~$20/mo + hardware** |

**Pros**: Local STT/TTS (fast, private, no per-minute cost), small/silent form factor, Apple Silicon is power-efficient
**Cons**: Mac hardware cost upfront, Claude brain still cloud
**Best for**: Permanent office installation, small business that wants a physical appliance
**Vision**: Ship a pre-configured Mac Mini — customer plugs in ethernet and power, done

---

### Tier 3 — PC + NVIDIA GPU (Linux/Windows)
**Hardware**: Any PC with RTX 3060+ (~$800-1200 total build)
**Target**: Businesses already running NVIDIA hardware or Linux infrastructure

| Component | Solution | Cost |
|-----------|----------|------|
| Brain | Claude API (Haiku) | Max subscription |
| STT | Moonshine ONNX (local, CUDA) | Free |
| TTS | Kokoro PyTorch (local, CUDA) | Free |
| Phone | Twilio | ~$20/mo |
| Claude Code | Local on Linux | Included |
| **Total** | | **~$20/mo + hardware** |

**Pros**: Cheaper GPU options than Apple, Linux ecosystem, same local STT/TTS benefits
**Cons**: Larger form factor, more power consumption, CUDA setup complexity
**Best for**: IT shops, existing server rooms, tech-savvy businesses

---

### Tier 4 — Edge Appliance (Jetson Orin)
**Hardware**: NVIDIA Jetson Orin Nano ($499)
**Target**: Fleet operations, remote sites, spotty internet

| Component | Solution | Cost |
|-----------|----------|------|
| Brain (fast) | Phi-3 / Mistral 7B (local, CUDA) | Free |
| Brain (complex) | Claude Haiku (cloud fallback) | As-needed |
| STT | Moonshine ONNX (local, CUDA) | Free |
| TTS | Kokoro (local, CUDA) | Free |
| Phone | Twilio (when internet available) | ~$20/mo |
| **Total** | | **~$20/mo + hardware** |

**Pros**: Fully sovereign for simple queries, small/rugged, low power (15W), GPU for inference
**Cons**: Limited RAM (8GB), needs internet for complex queries, CUDA setup on ARM
**Best for**: Trucks (Guardian Wings), remote job sites, field offices, anywhere internet is unreliable
**Vision**: Bolt to truck wall, connect to cellular hotspot, Jarvina runs on the road

---

### Tier 5 — Full Sovereign (Mac Mini + Local LLM)
**Hardware**: Mac Mini M4 Pro ($1599, 48GB RAM recommended)
**Target**: Maximum privacy, zero recurring cloud costs (except Twilio)

| Component | Solution | Cost |
|-----------|----------|------|
| Brain | Mistral 7B / Llama 3 8B (local, MLX) | Free |
| STT | Moonshine (local, Metal) | Free |
| TTS | Kokoro (local, Metal) | Free |
| Phone | Twilio | ~$20/mo |
| **Total** | | **~$20/mo + hardware** |

**Pros**: Zero cloud dependency except phone network, complete privacy, no per-token costs
**Cons**: Higher hardware cost, local LLM less capable than Claude, needs 32-48GB RAM
**Best for**: Legal/medical offices (HIPAA), government, privacy-first organizations
**Vision**: The sovereign voice appliance — a box that IS the entire virtual office

---

## Platform Comparison Matrix

| Feature | Tier 1 Cloud | Tier 2 Mac Mini | Tier 3 PC/GPU | Tier 4 Jetson | Tier 5 Sovereign |
|---------|-------------|----------------|---------------|---------------|-----------------|
| Monthly cost | ~$100 | ~$20 | ~$20 | ~$20 | ~$20 |
| Hardware cost | $0 | $599 | $800+ | $499 | $1599 |
| Setup time | 1 hour | Ship & plug | Half day | Half day | Half day |
| Internet needed | Always | For Claude only | For Claude only | Optional | For Twilio only |
| Voice latency | Higher | Low | Low | Lowest | Low |
| Privacy | Cloud | Mostly local | Mostly local | Mostly local | Fully local |
| GPU framework | N/A | Metal (Apple) | CUDA (NVIDIA) | CUDA (NVIDIA ARM) | Metal (Apple) |
| Form factor | Any | Mac Mini | PC/Server | Tiny (70x45mm) | Mac Mini |

---

## What's Platform-Agnostic (Same Code Everywhere)

- `server.py` — Jarvina voice server (Python, runs anywhere)
- `contacts.json` — Contact directory
- Twilio integration — Cloud API, platform-independent
- Claude API — Cloud, platform-independent
- Telegram Channels — Platform-independent
- Dashboard (PWA) — JavaScript, runs anywhere with Node.js

## What Needs Platform Adaptation

| Component | Mac (Metal) | NVIDIA (CUDA) | Cloud |
|-----------|------------|---------------|-------|
| TTS | mlx-audio Kokoro | PyTorch Kokoro | ElevenLabs API |
| STT | Moonshine ONNX | Moonshine ONNX | Deepgram API |
| Local LLM | MLX (Phi-3, Mistral) | vLLM / llama.cpp | Claude API |
| Process mgmt | launchd | systemd | AWS ECS / systemd |
| Tunnel | Tailscale Funnel | Tailscale Funnel | AWS ALB |

---

## Recommended Go-To-Market

1. **Demo with Tier 1** — spin up on any machine, show the product
2. **Sell Tier 2** — Mac Mini appliance, premium product, great margins
3. **Offer Tier 1 as SaaS** — monthly subscription, you host on AWS
4. **Tier 4 for fleet** — Guardian Wings territory, niche but high-value
5. **Tier 5 for enterprise** — privacy-first, custom pricing

---

## The Spreadsheet Philosophy

Same application logic, different infrastructure layers. Change the TTS engine, everything else keeps working. Swap the LLM, voice pipeline stays the same. Add a new platform, just implement the engine adapters. Cells and macros — Vernitron style.

---

*Sparked Matter LLC — the smartest spark in the room*
*We teach your matter new tricks.*

---

*"One product, five platforms, zero compromise."*
