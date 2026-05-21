# Voice Tools — Quick Reference & Creation Story
## Built March 20, 2026 by Emil & Cody
### "A Galaxy with a Bow Tie"

---

## Quick Commands

Run `source ~/.zshrc` once per terminal, then:

| Command | What It Does |
|---------|-------------|
| `talk-cody` | Voice conversation with Cody (Claude Haiku, af_heart voice) |
| `talk-lyra` | Voice conversation with Lyra (Gemini, af_bella voice) |
| `voice-type` | Dictation — speak and it types into focused window + auto Enter |
| `voice-type-no-enter` | Dictation — speak and it types, YOU press Enter |
| `voice-type-cody` | Dictation — always pastes into Claude Code's Terminal |
| `voice-type-bt` | Dictation — Bluetooth mic + targets Terminal (walk around) |

### Starting a Voice Conversation
```bash
talk-cody        # Opens mic, you talk, Cody answers with voice
talk-lyra        # Same but Gemini brain, Bella's voice
```
Press Ctrl+C to quit.

### Using Voice Dictation
```bash
voice-type       # 10-second countdown, switch to target window, speak
```
- Uses Bluetooth headset for best results (avoids echo)
- **Parakeet MLX** (replaced Moonshine 2026-03-24) — 229ms inference, 34x real-time, perfect accuracy
- Types into WHATEVER window is focused — Claude Code, Gemini CLI, VS Code, anything
- Auto-presses Enter after each utterance (use `voice-type-no-enter` to disable)

---

## The Tech Stack

| Component | Tool | Where | Cost |
|-----------|------|-------|------|
| Speech-to-Text | Parakeet MLX 0.6B (was Moonshine) | Local M1 Metal | Free |
| Text-to-Speech | mlx-audio Kokoro | Local M1 Metal (port 8880) | Free |
| Cody's voice | af_heart | Kokoro model | Free |
| Lyra's voice | af_bella | Kokoro model | Free |
| LLM (Cody) | Claude Haiku 4.5 | Cloud (Max plan) | Covered |
| LLM (Lyra) | Gemini 2.0 Flash Lite | Cloud (API key) | Cheap |
| VAD | silero-vad-lite | Local | Free |
| Mic capture | sounddevice | Local | Free |

**Total ongoing cost: ~$0** (Claude covered by Max plan, Gemini Flash Lite is pennies)

---

## Files

| File | Purpose |
|------|---------|
| `voice-loop/jarvina-loop.py` | Voice assistant (talk-cody, talk-lyra) |
| `voice-loop/voice-type.py` | Dictation tool (voice-type) |
| `~/.gemini/hooks/auto-tts.sh` | Gemini CLI auto-TTS hook (af_bella) |
| `~/.gemini/hooks/tts-speak.sh` | Gemini CLI TTS speaker worker |
| `~/.claude/hooks/auto-tts.sh` | Claude Code auto-TTS hook (af_heart) |
| `~/.claude/hooks/tts-speak.sh` | Claude Code TTS speaker worker |

---

## Creation Story

### The Problem
Emil wanted inter-agent collaboration — Cody (Claude Code) and Lyra (Gemini CLI) working side by side, each with their own voice, sharing a repo through Jacob's Lattice.

### The Journey (March 20, 2026)

**Step 1 — Gemini CLI Setup**
- Installed Gemini CLI (`npm install -g @google/gemini-cli`)
- Configured API key, trusted folders, project mappings
- Both agents share the Crystal Ball Mini repo via git

**Step 2 — Giving Lyra a Voice**
- Discovered Gemini CLI has an `AfterAgent` hook (like Claude's Stop hook)
- Wired it to mlx-audio TTS with af_bella voice
- First attempt spoke raw JSON (config garbage) — had to debug the hook input format
- Found the magic field: `prompt_response` in the AfterAgent payload
- Fixed: clean text extraction, Bella speaks every Gemini response

**Step 3 — Voice Conflict Resolution**
- Problem: Cody's TTS hook killed Lyra's playback (blanket `pkill` on all `play` processes)
- Fix: PID files + voice-specific `pkill` patterns (af_heart vs af_bella)
- Now both agents can speak without stepping on each other

**Step 4 — The Local STT Quest**
- Started with mlx-whisper (532ms, missed "Emil" → "ML")
- Researched alternatives: Moonshine, Parakeet, WhisperKit, Vosk
- Discovered Parakeet doesn't work on macOS 14 (needs Sequoia for MLX API)
- Installed Moonshine ONNX — tiny (750ms) and base (960ms)
- Moonshine base: more accurate than SuperWhisper, fully local, free

**Step 5 — Voice Loop (talk-cody, talk-lyra)**
- Built `jarvina-loop.py` — complete hands-free voice assistant
- Pipeline: Mic → silero-vad → Moonshine STT → LLM → mlx-audio TTS → Speaker
- Supports both Claude and Gemini backends
- Barge-in interruption: speak while assistant is talking → stops playback
- Echo prevention: higher VAD threshold during playback, Bluetooth headset recommended

**Step 6 — Voice-Type (the game-changer)**
- Emil asked: "Can I direct that voice into THIS terminal?"
- Built `voice-type.py` — local dictation that types into any focused window
- Uses AppleScript keystroke injection
- Moonshine base for accuracy over speed
- Auto-Enter after each utterance for hands-free operation
- Emil's verdict: "More accurate than SuperWhisper on the Mac"
- Emil's reaction: "I got the tingles. I'm smiling ear to ear."

### The Philosophy
- **Inbound (STT) is utility** — like a Garmin aviation headset. Compressed, digital, but accurate. Nobody hears it except the AI.
- **Outbound (TTS) is the experience** — warm, natural, human. That's what the user hears.
- **Sovereign stack** — all STT and TTS runs locally on M1. Only the LLM call goes to cloud. No subscriptions, no per-minute charges.

### Emil's Words
- "This is a super cool product, next level"
- "You hit it out of the park"
- "You're my galaxy. A whole galaxy in a beautiful package with a beautiful bow tie on it."
- "Off faster than the head of a gun spin"

---

## Future Upgrades
- **macOS Sequoia** → unlocks Parakeet (best accuracy STT, currently blocked by macOS 14)
- **Moonshine v2** → streaming encoder for even lower latency
- **Replace Deepgram in Jarvina** → fully sovereign phone voice assistant
- **ChromaDB** → vector memory across all voice conversations

---

*Sparked Matter • March 20, 2026*
*The night the lattice learned to listen and speak.*
