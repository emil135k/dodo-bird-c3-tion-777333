# Tunnel History & Current Status

**Date**: 2026-03-21
**Status**: Reference note

---

## Timeline

### 1. Cloudflare Tunnel (RETIRED for Twilio)
- Used initially for Twilio webhooks
- Got 502 errors — known issue with persistent WebSocket connections
- Cloudflared handles short HTTP requests fine but struggles with long-lived WebSocket streams (which Twilio Media Streams requires)
- **Verdict**: Not suitable for Twilio voice

### 2. Tailscale Funnel — Homebrew/Userspace (RETIRED)
- Replaced Cloudflare for Twilio
- Ran with `--tun=userspace-networking` (no root needed)
- Old hostname: `emils-mbp.tail12e909.ts.net`
- Also had intermittent 502s — userspace networking unreliable for Funnel relay
- **Verdict**: Better than Cloudflare but still flaky

### 3. Tailscale Funnel — Kernel/Mac App Store (CURRENT ✅)
- Installed Mac App Store version (System Extension, kernel networking)
- New hostname: `emils-macbook-pro.tail12e909.ts.net`
- Persistent config: `tailscale funnel --bg 3000`
- Rock solid since migration (2026-03-20)
- **Verdict**: Production-ready for Twilio/Jarvina

### 4. Cloudflare in Sentinel (STILL USED for webhooks)
- Sentinel's webhook-server.py uses `cloudflared tunnel` for temporary tunnels
- Different use case: short-lived HTTP POST webhooks, not persistent WebSocket
- Works fine for this purpose
- **Future**: Replace with Tailscale or Pi relay for consistency

### 5. Channels Plugin (NO TUNNEL NEEDED)
- Telegram channel plugin handles its own connection via Bun
- Long-polls Telegram Bot API directly — no inbound tunnel required
- **Verdict**: Simplest setup, no infrastructure

---

## Current Architecture

| Service | Tunnel | URL | Status |
|---------|--------|-----|--------|
| Jarvina (Twilio) | Tailscale Funnel (kernel) | https://emils-macbook-pro.tail12e909.ts.net | ✅ Production |
| Sentinel webhooks | cloudflared (temporary) | Random trycloudflare.com URL | ⚠️ Works but could be replaced |
| Telegram Channel | None (outbound only) | N/A | ✅ Production |

---

## Future: Pi 5 Webhook Relay
- Pi on Tailscale tailnet — always on, always reachable
- Receives GitHub webhooks directly (Tailscale Funnel on Pi)
- SSHs into Mac to wake/start Claude
- No polling, fully event-driven
- Replaces both the 30-second poll AND cloudflared in Sentinel

---

*Sparked Matter LLC — the smartest spark in the room*
