# Jarvina Voice Assistant — Launch Guide
## Quick Reference for Emil

---

## Launch Command (THE ONLY WAY)

```bash
bash ~/crystalballmini/scripts/jarvina-launch.sh
```

That's it. One command. It does EVERYTHING:

1. Kills all old processes
2. Closes old terminal windows
3. Starts Tailscale Funnel
4. Verifies funnel URL
5. Updates .env with tunnel URL
6. Ensures mlx-audio TTS is running (port 8880)
7. Launches Jarvis voice server (port 3000)
8. Launches debug log tail
9. Launches TShark network monitor
10. Launches email worker
11. Tiles all windows
12. Health check
13. Verifies TTS engine
14. Smoke tests Kokoro TTS

## RULES
- **NEVER restart one component** — always run the full script
- **NEVER launch windows manually** — the script does it
- **If it fails** — read the error, fix it, run the script again from the top

---

## What It Launches (6 Windows)

| Window | Purpose |
|--------|---------|
| Tailscale Funnel | Tunnel from internet to localhost:3000 |
| Jarvis Server | Voice server (Python, port 3000) |
| Debug Log | Real-time `debug-twilio.log` tail |
| TShark | Network packet inspector on port 3000 |
| Email Worker | O365 email integration |
| Claude (you) | This session — stays at bottom |

---

## Architecture

```
Phone Call → Twilio → Tailscale Funnel → localhost:3000 → Jarvis Server
                                                            ├── Deepgram STT (speech → text)
                                                            ├── Claude Haiku (brain)
                                                            └── mlx-audio TTS (text → speech)
                                                                └── Jitter Buffer → Twilio → Phone
```

## Key Config
- **Server**: `twilio/jarvis/server.py` (port 3000)
- **LLM**: Claude Haiku 4.5 (`claude-haiku-4-5-20251001`)
- **TTS**: mlx-audio on port 8880, Kokoro model, af_heart voice
- **STT**: Deepgram nova-2, endpointing=600ms
- **Tunnel**: Tailscale Funnel → `https://emils-macbook-pro.tail12e909.ts.net`
- **Phone**: FROM +18136076219 (Twilio), TO +18133340414 (Emil)

## Env Files
- `twilio/jarvis/.env` — Jarvis server config
- `twilio/.env` — Twilio/TTS config

## Troubleshooting
- **Health check fails**: Server may need extra seconds to start. Check `curl http://localhost:3000/health`
- **Funnel DNS not resolving**: Tailscale userspace networking may need restart. Check `tailscale status`
- **TTS not working**: Verify mlx-audio: `curl http://localhost:8880/v1/models`
- **Jitter/audio issues**: Check debug log in the tailed window
- **Server crashes**: Check `/tmp/cody-server.log`

---

## Related Voice Tools (NEW — March 20, 2026)

These are LOCAL voice tools, separate from Twilio phone calls:

| Command | What It Does |
|---------|-------------|
| `talk-cody` | Voice chat with Cody (Claude + af_heart) |
| `talk-lyra` | Voice chat with Lyra (Gemini + af_bella) |
| `voice-type` | Dictation — types into any window |

See `emil/voice-tools-guide.md` for details.

---

*Sparked Matter • Last updated March 20, 2026*
