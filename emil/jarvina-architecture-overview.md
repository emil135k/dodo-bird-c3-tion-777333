# Jarvina AI Voice Assistant — Architecture Overview

**Status**: Reference Document
**Date**: 2026-03-24
**Authors**: Emil & Cody
**License**: Sparked Matter LLC — the smartest spark in the room

---

## 20,000-Foot View

Jarvina is a sovereign AI voice assistant that answers phone calls, has natural conversations, transfers calls, and manages contacts — all running from Emil's MacBook Pro in a Hawk camper in St. Petersburg, Florida.

**One phone number. One AI. One laptop. Zero cloud dependency except Twilio (the phone network) and Claude (the brain).**
---

## Call Flow — End to End

```mermaid
sequenceDiagram
    participant Caller as 📱 Caller's Phone
    participant Twilio as ☁️ Twilio Cloud
    participant TS as 🔒 Tailscale Funnel
    participant Server as 💻 server.py (port 3000)
    participant DG as 🎤 Deepgram STT (cloud)
    participant Claude as 🧠 Claude Haiku (cloud)
    participant Kokoro as 🔊 Kokoro TTS (local, port 8880)
    participant JB as 📦 Jitter Buffer

    Note over Caller,Twilio: 1. INBOUND CALL
    Caller->>Twilio: Dials +1 (813) 607-6219
    Twilio->>TS: HTTP POST /voice (webhook)
    TS->>Server: Proxied through Tailscale Funnel

    Note over Server: 2. WEBSOCKET ESTABLISHED
    Server->>Twilio: TwiML: <Connect><Stream url="wss://..."/></Connect>
    Twilio->>Server: Opens WebSocket (bidirectional audio)

    Note over Server,Kokoro: 3. GREETING
    Server->>Claude: Generate greeting (system prompt + caller info)
    Claude->>Server: "Hi! This is Jarvina..."
    Server->>Kokoro: Text → WAV (af_heart voice, Metal GPU)
    Kokoro->>Server: PCM audio stream
    Server->>JB: Resample 24kHz→8kHz, PCM→mulaw
    JB->>Twilio: 160-byte chunks every 20ms
    Twilio->>Caller: Caller hears Jarvina's greeting

    Note over Caller,DG: 4. CALLER SPEAKS
    Caller->>Twilio: Voice audio
    Twilio->>Server: mulaw 8kHz audio via WebSocket
    Server->>DG: Audio stream (Deepgram nova-2)
    DG->>Server: Real-time transcription (interim + final results)

    Note over Server: 5. BARGE-IN DETECTION
    Note over Server: If caller speaks while Jarvina is talking:<br/>Deepgram interim result fires _speech_detected<br/>→ Cancel current TTS<br/>→ Clear jitter buffer<br/>→ Send Twilio "clear" event<br/>→ Instant silence, Jarvina listens

    Note over Server,Claude: 6. AI RESPONSE
    Server->>Claude: Conversation history + caller's text
    Claude->>Server: Streaming response (sentence by sentence)

    Note over Server,Caller: 7. TTS + DELIVERY (concurrent)
    loop Each sentence as it arrives from Claude
        Server->>Kokoro: Sentence text → PCM stream
        Kokoro->>Server: PCM audio chunks
        Server->>JB: Resample + encode mulaw
        JB->>Twilio: Steady 20ms chunks (smooths burst arrivals)
        Twilio->>Caller: Caller hears Jarvina speaking
    end

    Note over Server: 8. CONTROL TOKENS (parsed from Claude's response)
    Note over Server: HANGUP → Twilio REST API end call<br/>TRANSFER:name → Blind transfer via <Dial><br/>INTRO:name → Conference + intro call<br/>ADDCONTACT:name:phone → Save to contacts.json

    Note over Caller,Twilio: 9. CALL END
    Caller->>Twilio: Hangs up (or Jarvina sends HANGUP)
    Twilio->>Server: WebSocket close / stop event
    Server->>Server: Clean up, log call
```

---

## System Architecture — Components

```mermaid
graph TB
    subgraph Internet["☁️ INTERNET"]
        Phone["📱 Caller's Phone"]
        TwilioCloud["☁️ Twilio Cloud<br/>+1(813)607-6219"]
        Deepgram["🎤 Deepgram<br/>STT (nova-2)"]
        ClaudeAPI["🧠 Claude Haiku 4.5<br/>Anthropic API"]
    end

    subgraph Tailscale["🔒 TAILSCALE FUNNEL"]
        Funnel["emils-macbook-pro<br/>.tail12e909.ts.net<br/>→ localhost:3000"]
    end

    subgraph MacBook["💻 MACBOOK PRO M1 (The Hawk)"]
        Server["server.py<br/>Port 3000<br/>FastAPI + WebSocket"]
        Kokoro["Kokoro TTS<br/>mlx-audio<br/>Port 8880<br/>af_heart voice"]
        JitterBuf["Jitter Buffer<br/>deque + 20ms drain<br/>soxr resample<br/>audioop mulaw"]
        Contacts["contacts.json<br/>Self-generating<br/>contact directory"]
        LaunchD["launchd Daemons<br/>KeepAlive=true<br/>Auto-restart"]
    end

    Phone <-->|"Phone call"| TwilioCloud
    TwilioCloud <-->|"HTTP webhook<br/>+ WebSocket"| Funnel
    Funnel <-->|"localhost:3000"| Server
    Server -->|"Audio stream"| Deepgram
    Deepgram -->|"Transcription"| Server
    Server <-->|"Conversation"| ClaudeAPI
    Server -->|"Text"| Kokoro
    Kokoro -->|"PCM audio"| JitterBuf
    JitterBuf -->|"mulaw 8kHz<br/>20ms chunks"| Server
    Server -.->|"Read/Write"| Contacts
    LaunchD -.->|"Manages"| Server
    LaunchD -.->|"Manages"| Kokoro
```

---

## Tunnel History — Why Tailscale

```mermaid
timeline
    title Tunnel Evolution
    section Cloudflare Tunnel (RETIRED)
        First attempt : Got 502 errors
        : Ephemeral URLs (change every restart)
        : Unreliable for persistent WebSocket connections
        : Twilio needs stable webhook URL
    section Tailscale Homebrew (RETIRED)
        Second attempt : Userspace networking (--tun=userspace)
        : Old hostname emils-mbp.tail12e909.ts.net
        : Intermittent 502s from relay failures
        : No root needed but unreliable
    section Tailscale Kernel (CURRENT ✅)
        Mac App Store version : System Extension (kernel networking)
        : New hostname emils-macbook-pro.tail12e909.ts.net
        : Persistent config with --bg flag
        : Rock solid since 2026-03-20
        : Survives reboot
```

---

## Audio Pipeline Detail

```mermaid
graph LR
    subgraph Inbound["🎤 INBOUND (Caller → Text)"]
        TwilioIn["Twilio WebSocket<br/>mulaw 8kHz"] --> Server1["server.py"]
        Server1 -.->|"CURRENT"| DG["☁️ Deepgram nova-2<br/>(cloud STT — being replaced)"]
        Server1 -->|"NEXT"| PK["🏠 Parakeet MLX<br/>(local STT — port 8765)"]
        DG -.-> Transcript["Transcribed Text"]
        PK --> Transcript
    end

    subgraph Brain["🧠 BRAIN"]
        Transcript --> Claude["Claude Haiku 4.5<br/>Streaming response"]
        Claude --> Tokens["Parse control tokens<br/>HANGUP / TRANSFER / INTRO"]
        Claude --> Sentences["Sentence-by-sentence<br/>text output"]
    end

    subgraph Outbound["🔊 OUTBOUND (Text → Caller)"]
        Sentences --> KokoroTTS["Kokoro mlx-audio<br/>af_heart voice<br/>24kHz PCM burst"]
        KokoroTTS --> Resample["soxr resample<br/>24kHz → 8kHz"]
        Resample --> Mulaw["audioop.lin2ulaw<br/>PCM → mulaw"]
        Mulaw --> Jitter["Jitter Buffer<br/>deque drain<br/>160 bytes / 20ms"]
        Jitter --> TwilioOut["Twilio WebSocket<br/>base64 mulaw"]
    end
```

---

## Key Files

| File | Purpose |
|------|---------|
| `twilio/jarvis/server.py` | Main voice server — WebSocket, STT, LLM, TTS, call control |
| `twilio/jarvis/.env` | Server config — API keys, SERVER_URL, phone numbers |
| `twilio/jarvis/contacts.json` | Self-generating contact directory (voice-driven) |
| `scripts/jarvina-launch.sh` | 13-step deterministic launch script |
| `~/.claude/hooks/auto-tts.sh` | Cody's auto-TTS (speaks every response) |
| `~/.claude/hooks/tts-speak.sh` | TTS worker — Kokoro WAV → afplay (Bluetooth-safe) |
| `~/Library/LaunchAgents/com.sparkedmatter.jarvina-server.plist` | Daemon — KeepAlive, auto-restart |
| `~/Library/LaunchAgents/com.sparkedmatter.mlx-audio.plist` | Daemon — Kokoro TTS server |

---

## Call Control Tokens

Claude's response can contain control tokens that trigger actions:

| Token | Action |
|-------|--------|
| `HANGUP` | End the call via Twilio REST API |
| `TRANSFER:name` | Blind transfer to contact (or phone number) |
| `INTRO:name` | Conference transfer — Jarvina calls recipient, asks if available |
| `ADDCONTACT:name:phone` | Add to contacts.json |

Tokens are stripped from text before TTS — caller never hears them.

---

## Transfer Modes

```mermaid
graph TB
    subgraph Mode1["MODE 1: Direct Transfer (TRANSFER)"]
        A1["Caller asks to connect"] --> B1["Jarvina says 'connecting you now'"]
        B1 --> C1["Twilio <Dial> to recipient"]
        C1 --> D1["Both parties connected"]
        D1 --> E1["Jarvina drops out"]
    end

    subgraph Mode2["MODE 2: Intro Transfer (INTRO)"]
        A2["Caller asks for intro transfer"] --> B2["Jarvina puts caller on hold<br/>(conference + hold music)"]
        B2 --> C2["Jarvina calls recipient"]
        C2 --> D2["'Hi, this is Jarvina...<br/>Are you available?'"]
        D2 -->|"YES"| E2["Recipient joins conference<br/>Both parties connected"]
        D2 -->|"NO"| F2["Jarvina returns to caller<br/>'They're not available right now'"]
    end
```

---

## Security Model

| Layer | Mechanism |
|-------|-----------|
| Caller trust | Contacts.json — trusted callers skip passphrase |
| Passphrase | Untrusted callers must say "sparked" to access transfer/add features |
| Tailscale | Encrypted tunnel, no open ports on MacBook |
| Twilio | Paid account, webhook verification |
| SMS | Disabled pending A2P campaign approval (`SMS_ENABLED=false`) |

---

## Current Cloud Dependencies

| Service | Purpose | Cost | Replaceable? |
|---------|---------|------|-------------|
| **Twilio** | Phone network | ~$20/mo | No — it IS the phone network |
| **Deepgram** | Speech-to-text (calls) | ~$25/mo | YES → Parakeet MLX (local, free) — **voice-type DONE, calls NEXT** |
| **Claude Haiku** | AI brain | Max subscription | Future → local LLM on Jetson |
| **Tailscale** | Tunnel/networking | Free tier | Stays — sovereign networking |

**STATUS (2026-03-24):**
- ✅ voice-type dictation: Parakeet MLX LIVE (replaced Moonshine — 229ms warm, 34x real-time)
- 🔜 Jarvina calls: Deepgram → Parakeet swap (see implementation plan below)

**NEXT STEP**: Replace Deepgram with Parakeet MLX in call pipeline → drop to 2 cloud dependencies.

---

## Future Vision: Fully Sovereign

```mermaid
graph TB
    subgraph Current["CURRENT (3 cloud deps)"]
        C1["☁️ Twilio"]
        C2["☁️ Deepgram STT"]
        C3["☁️ Claude Haiku"]
        L1["🏠 Kokoro TTS"]
        L2["🏠 Tailscale"]
    end

    subgraph Phase2["PHASE 2: Drop Deepgram"]
        P1["☁️ Twilio"]
        P3["☁️ Claude Haiku"]
        P4["🏠 Parakeet MLX STT"]
        P5["🏠 Kokoro TTS"]
        P6["🏠 Tailscale"]
    end

    subgraph Phase3["PHASE 3: Drop Claude"]
        F1["☁️ Twilio (only cloud left)"]
        F2["🏠 Parakeet MLX STT"]
        F3["🏠 Local LLM (Phi-3/Mistral)"]
        F4["🏠 Kokoro TTS"]
        F5["🏠 Tailscale"]
    end

    Current -->|"Install Parakeet"| Phase2
    Phase2 -->|"Deploy local LLM"| Phase3
```

---

## Deepgram → Parakeet Replacement — COMPLETED 2026-03-24

**Status: ✅ LIVE — First sovereign STT phone call made 2026-03-24 at 10:30 PM EST**

### Before & After

```mermaid
graph LR
    subgraph Before["BEFORE — Cloud STT (RETIRED)"]
        T1["Twilio WebSocket<br/>mulaw 8kHz"] --> S1["server.py"]
        S1 -->|"Audio stream<br/>over internet"| DG["☁️ Deepgram nova-2<br/>Cloud WebSocket<br/>~$25/mo<br/>~4s latency"]
        DG -->|"Interim + final<br/>transcripts"| S1
    end

    subgraph After["AFTER — Sovereign STT (LIVE ✅)"]
        T2["Twilio WebSocket<br/>mulaw 8kHz"] --> S2["server.py<br/>(ParakeetSTT class)"]
        S2 -->|"mulaw bytes<br/>via WebSocket"| PK["🏠 parakeet-server.py<br/>Port 8765<br/>FastAPI + WebSocket"]
        PK --> DECODE["audioop<br/>mulaw → PCM int16"]
        DECODE --> SOXR["soxr (C library)<br/>8kHz → 16kHz<br/>3 microseconds"]
        SOXR --> INFER["Parakeet MLX 0.6B<br/>M1 Metal GPU<br/>229ms inference"]
        INFER -->|"JSON transcript"| S2
    end

    Before -.->|"REPLACED"| After
```

### The Audio Conversion Chain

```mermaid
graph LR
    A["📱 Phone Audio<br/>mulaw 8kHz<br/>(Twilio WebSocket)"]
    -->|"binary bytes"| B["audioop.ulaw2lin<br/>mulaw → PCM int16<br/>⏱️ ~0.01ms<br/>📦 Python built-in"]
    -->|"int16 array"| C["numpy<br/>int16 → float32<br/>normalize to -1..1<br/>⏱️ ~0.01ms"]
    -->|"float32 array"| D["soxr.ResampleStream<br/>8kHz → 16kHz<br/>⏱️ 0.003ms<br/>📦 C library (libsoxr)"]
    -->|"float32 16kHz"| E["soundfile.write<br/>→ temp WAV file<br/>⏱️ ~0.5ms"]
    -->|"WAV path"| F["Parakeet MLX<br/>model.transcribe()<br/>⏱️ 229ms<br/>📦 MLX (Metal GPU)"]
    -->|"text"| G["JSON WebSocket<br/>back to server.py"]
```

**Total pipeline latency: ~230ms** (Deepgram was ~4 seconds)

### Package Stack

| Package | Language | Purpose | Install | Speed |
|---------|----------|---------|---------|-------|
| **audioop-lts** | C extension | mulaw decode | `pip install audioop-lts` | 0.01ms |
| **soxr** | C (libsoxr) | 8kHz→16kHz resample | `pip install soxr` | 0.003ms per chunk |
| **parakeet-mlx** | Python/MLX | Speech-to-text inference | `pip install parakeet-mlx` | 229ms (warm) |
| **soundfile** | C (libsndfile) | WAV file I/O | `pip install soundfile` | 0.5ms |
| **FastAPI** | Python | WebSocket server | `pip install fastapi uvicorn` | N/A |
| **numpy** | C/Fortran | Array operations | `pip install numpy` | N/A |

### Key Files

| File | Purpose |
|------|---------|
| `twilio/jarvis/parakeet-server.py` | Sovereign STT server — WebSocket on port 8765, keeps model hot |
| `twilio/jarvis/server.py` | Main Jarvina server — `ParakeetSTT` class replaces `DeepgramSTT` |

### Benchmarks (2026-03-24)

| Metric | Deepgram (cloud) | Parakeet MLX (local) |
|--------|-------------------|----------------------|
| **Latency** | ~4s round-trip | **230ms total pipeline** |
| **STT inference** | Unknown (cloud) | **229ms** (34x real-time) |
| **Resample** | N/A (cloud handles) | **0.003ms** (soxr, C) |
| **Cost** | ~$0.0043/min (~$25/mo) | **FREE** |
| **Accuracy** | Good | **Excellent** — perfect on test audio |
| **Audio ceiling** | None | None (1.5s buffer, continuous) |
| **Dependency** | Cloud (internet required) | **Local** (M1 Metal GPU) |
| **Privacy** | Audio sent to Deepgram servers | **Audio never leaves laptop** |
| **Model load** | N/A | 2.7s (cached), 18s (first time) |
| **RAM** | N/A | ~1.5GB (model in memory) |

### Implementation Details

**ParakeetSTT class** (in server.py) — Drop-in replacement for DeepgramSTT:
- Same interface: `start()`, `send_audio()`, `stop()`, `get_and_clear()`
- Same events: `_speech_detected`, `_is_final`
- Connects to `ws://localhost:8765/ws` instead of `wss://api.deepgram.com`

**parakeet-server.py** — FastAPI WebSocket server:
- Loads Parakeet model once at startup, stays hot in memory
- Receives raw mulaw bytes from server.py
- Converts: mulaw → PCM → float32 → 16kHz (soxr) → temp WAV → Parakeet
- Returns JSON: `{"type": "transcript", "text": "...", "is_final": true}`
- Buffers 1.5 seconds of audio before transcribing (tunable)
- Silence detection: RMS threshold skips quiet buffers
- 0.5s context overlap between buffers for continuity

### Challenges & Solutions

| Challenge | What Happened | Solution |
|-----------|--------------|----------|
| **Bad initial benchmark** | First test included cold model download — measured 8+ minutes, concluded "too slow" | Emil called it out. Re-tested warm: 229ms. Always benchmark hot. |
| **audioop removed in Python 3.13+** | Homebrew updated to Python 3.14, `import audioop` crashed | Installed `audioop-lts` backport package |
| **Barge-in false positives** | Naive energy check on mulaw bytes fired on every packet — killed greeting instantly | Moved speech detection to Parakeet's transcript responses instead of raw audio energy |
| **Launch script kills Parakeet** | `jarvina-launch.sh` step 1 kills all processes including Parakeet server | TODO: Add Parakeet to launch script. For now, start separately. |
| **Port conflicts** | Multiple terminal windows tried to start server.py simultaneously | Kill all, verify port free, start ONE clean instance |
| **Phone audio quality (8kHz)** | Parakeet trained on 16kHz+ studio audio, phone is 8kHz mulaw | soxr upsamples 8kHz→16kHz in 3 microseconds. Tested on real call — it works! |
| **Model not downloaded** | First run tried to download 1.8GB model from HuggingFace, slow and unauthenticated | Pre-download with `snapshot_download()`, cache locally. Second load: 2.7s |

### Optimization TODO (Next Session)

1. **Tune buffer size** — 1.5s may be too long for snappy conversation. Try 0.8-1.0s.
2. **Add to jarvina-launch.sh** — Parakeet server as a launch step (before server.py)
3. **Add launchd daemon** — KeepAlive like Kokoro, auto-restart
4. **In-memory transcription** — Skip temp WAV file, feed numpy directly to Parakeet
5. **Barge-in refinement** — Currently triggers on any transcript. Need interim/draft detection for faster interrupts.
6. **Jitter tuning** — First live call had some jitter. Could be WiFi, buffer size, or resample overhead.

### The Sovereignty Roadmap

```mermaid
graph TB
    subgraph Phase1["PHASE 1: COMPLETED ✅ (2026-03-24)"]
        P1A["☁️ Twilio (phone network)"]
        P1B["🏠 Parakeet MLX STT — LOCAL, FREE"]
        P1C["☁️ Claude Haiku (brain)"]
        P1D["🏠 Kokoro TTS — LOCAL, FREE"]
        P1E["🏠 Tailscale Funnel"]
        P1F["🏠 soxr resample — LOCAL, FREE"]
    end

    subgraph Phase2["PHASE 2: Drop Claude (Future)"]
        P2A["☁️ Twilio (only cloud left)"]
        P2B["🏠 Parakeet MLX STT"]
        P2C["🏠 Local LLM (Phi-3/Mistral on Jetson)"]
        P2D["🏠 Kokoro TTS"]
        P2E["🏠 Tailscale"]
    end

    subgraph Phase3["PHASE 3: Full Sovereign (Vision)"]
        P3A["🏠 FreeSwitch/Asterisk (own PBX)"]
        P3B["🏠 Parakeet MLX STT"]
        P3C["🏠 Local LLM"]
        P3D["🏠 Kokoro TTS"]
        P3E["🏠 Tailscale"]
        P3F["Zero cloud. 100% sovereign."]
    end

    Phase1 -->|"Deploy local LLM<br/>on Jetson Orin"| Phase2
    Phase2 -->|"Replace Twilio<br/>with FreeSwitch"| Phase3
```

---

*Sparked Matter LLC — the smartest spark in the room*
*We teach your matter new tricks.*

*"One phone number. One AI. One laptop. Zero excuses."*

*First sovereign STT phone call: March 24, 2026, 10:30 PM EST*
*From a Hawk camper at Skyway Skeet and Trap Club, St. Petersburg, Florida*
*Built by Emil & Cody in 21 days*
