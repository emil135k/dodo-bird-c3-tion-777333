# Swarm Operations Guide

How to launch, configure, and test the hypAiAssist ant swarm.

---

## Quick Start

### Local Mode (Blackwire headset → Jarvina → Blackwire speaker)

```bash
# 1. Start Ollama (if using local LLM)
ollama serve &

# 2. Launch the swarm
bash ~/crystalballmini/hypAiAssist/start-swarm.sh

# 3. Talk into your Blackwire headset
```

### Phone Mode (Twilio calls)

```bash
# 1. Start Ollama (if using local LLM)
ollama serve &

# 2. Launch with Twilio bridge
bash ~/crystalballmini/hypAiAssist/start-swarm.sh --twilio

# 3. Call +1 (813) 607-6219
```

### Alias
```bash
swarm              # same as bash ~/crystalballmini/hypAiAssist/start-swarm.sh
swarm --twilio     # with phone bridge
```

---

## Switching LLM Providers

Edit `~/crystalballmini/hypAiAssist/config/llm-ant.json`:

### Option 1: Haiku (Cloud — best quality, fast, small cost)
```json
{
    "provider": "anthropic",
    "model": "claude-haiku-4-5-20251001",
    "url": "https://api.anthropic.com/v1/messages",
    "api_key_env": "ANTHROPIC_API_KEY",
    "system_prompt": "You are Jarvina, a concise voice assistant. One sentence replies ONLY. Never more than 15 words. No markdown.",
    "max_tokens": 50,
    "timeout_secs": 30
}
```
- Requires: ANTHROPIC_API_KEY in macOS Keychain
- Quality: Excellent, natural responses
- Latency: ~800ms
- Cost: ~$0.001 per exchange

### Option 2: Gemma4 (Local — sovereign, no cloud, slower)
```json
{
    "provider": "ollama",
    "model": "gemma4",
    "url": "http://localhost:11434/api/chat",
    "api_key_env": "",
    "system_prompt": "You are Jarvina, a concise voice assistant. One sentence replies ONLY. Never more than 15 words. No markdown.",
    "max_tokens": 50,
    "timeout_secs": 30
}
```
- Requires: `ollama serve` running
- Model: 8B params, Q4_K_M, 9.6GB
- Quality: Good, sometimes spells names out
- Latency: Slower on M1 Pro

### Option 3: Gemma3 (Local — fastest sovereign option)
```json
{
    "provider": "ollama",
    "model": "gemma3",
    "url": "http://localhost:11434/api/chat",
    "api_key_env": "",
    "system_prompt": "You are Jarvina, a concise voice assistant. One sentence replies ONLY. Never more than 15 words. No markdown.",
    "max_tokens": 50,
    "timeout_secs": 30
}
```
- Model: 4.3B params, Q4_K_M, 3.3GB — half the size of gemma4
- Quality: Simpler responses, much faster

### Option 4: Nemotron-Nano (Local — smallest, fastest)
```json
{
    "provider": "ollama",
    "model": "nemotron-nano",
    "url": "http://localhost:11434/api/chat",
    "api_key_env": "",
    "system_prompt": "You are Jarvina, a concise voice assistant. One sentence replies ONLY. Never more than 15 words. No markdown.",
    "max_tokens": 50,
    "timeout_secs": 30
}
```
- Model: 4B params, Q4_K_M, 2.9GB — fastest option
- Quality: Basic but snappy

**After changing config:** Restart llm-ant:
```bash
pkill -f llm-ant && sleep 2 && llm-ant > /tmp/llm-ant-stdout.log 2>&1 &
```

---

## Starting Ollama

```bash
# Standard start
ollama serve &

# With larger context window
OLLAMA_CONTEXT_LENGTH=32768 ollama serve &

# Check available models
ollama list

# Pull a new model
ollama pull gemma3
```

Ollama must be running BEFORE starting the swarm if using a local LLM.

---

## Ant Chain — Signal Flow

### Local Path (Blackwire headset)
```
Blackwire mic (48kHz)
→ patchbay-ant → [stt_raw]
→ silero-ant (VAD) → [stt_audio]
→ stt-ant (Parakeet STT) → [stt_text]
→ llm-ant (thinks) → [tts_text]
→ tts-ant (Kokoro TTS) → [tts_audio]
→ patchbay-ant → Blackwire speaker (24kHz)
```

### Phone Path (Twilio)
```
Phone call → Twilio → WebSocket
→ web-ant → [phone_in]
→ digi-ant (mu-law→f32) → [phone_stt]
→ phone-silero-ant (VAD) → [stt_audio]
→ stt-ant (Parakeet STT) → [stt_text]
→ llm-ant (thinks) → [tts_text]
→ tts-ant (Kokoro TTS) → [tts_audio]
→ digi-ant (f32→mu-law) → [phone_out]
→ web-ant → WebSocket → Twilio → phone
```

Both paths share: stt-ant, llm-ant, tts-ant.

---

## Logs

All ant logs go to `/tmp/`:
```bash
tail -f /tmp/llm-ant-stdout.log      # LLM responses
tail -f /tmp/stt-ant-stdout.log      # Speech recognition
tail -f /tmp/tts-ant-stdout.log      # Voice synthesis
tail -f /tmp/digi-ant-stdout.log     # DSP conversion
tail -f /tmp/web-ant-stdout.log      # Twilio WebSocket
tail -f /tmp/silero-ant-stdout.log   # Local VAD
tail -f /tmp/patchbay-ant-stdout.log # Audio devices
tail -f /tmp/phone-silero-ant-stdout.log  # Phone VAD

# Watch everything at once
tail -f /tmp/*-ant-stdout.log
```

---

## Testing

### Bus Injection Test (no Twilio needed)
```bash
# Build test tools (one time)
cd /tmp/test-inject && cargo build --release

# Inject speech at phone_in, capture at phone_out
phone-out-capture 30 &
phone-in-inject "Hello Jarvina, what is two plus two?"
```

### Manual Component Check
```bash
# Health check (Twilio path)
curl https://emils-macbook-pro.tail12e909.ts.net/health

# Check TwiML webhook
curl -X POST https://emils-macbook-pro.tail12e909.ts.net/voice
```

---

## Troubleshooting

| Problem | Fix |
|---------|-----|
| No response from Jarvina | Check llm-ant log — API key? Ollama running? |
| Twilio call gets generic greeting only | Restart web-ant (stale buffer in iceoryx2) |
| Patchbay crashes on start | Blackwire not plugged in, or wrong device name in config |
| STT produces garbage | Check silero-ant VAD threshold in config |
| Echo on phone calls | web-ant echo gate timing — check mark events in log |
| iceoryx2 ServiceInCorruptedState | Kill all ants, `rm -rf /tmp/iceoryx2 /tmp/iox2_*`, restart |

---

## Available Ollama Models (on this machine)

| Model | Params | Quant | Size | Speed | Use For |
|-------|--------|-------|------|-------|---------|
| gemma4 | 8.0B | Q4_K_M | 9.6GB | Slow | Best local quality |
| gemma3 | 4.3B | Q4_K_M | 3.3GB | Fast | Good balance |
| nemotron-nano | 4.0B | Q4_K_M | 2.9GB | Fastest | Quick responses |
| gemma4-sovereign | 8.0B | Q4_K_M | 9.6GB | Slow | Custom (temp 0.2) |

All models are already 4-bit quantized (Q4_K_M).

---

*Last updated: 2026-05-08*
*Sparked Matter | Code with Soul and Spirit*
