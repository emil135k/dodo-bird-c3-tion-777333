# Jarvina v5 — Twilio Phone Bridge Audio Flow (Jetson Orin Nano)

```mermaid
flowchart TD

    %% ─── PHONE NETWORK ───────────────────────────────────────────────────────
    subgraph PHONE["📞 Phone Network"]
        CALLER["Phone Caller"]
        PSTN["PSTN"]
        TWILIO_CLOUD["Twilio Cloud"]
    end

    %% ─── TWILIO BRIDGE (Rust, port 5000) ────────────────────────────────────
    subgraph BRIDGE["Twilio Bridge  ·  Rust · port 5000"]
        WS_IN["WebSocket RX"]
        B64_DEC["base64 decode"]
        MULAW_DEC["mu-law decode\n⚙ Rust (no GStreamer)"]
        UPSAMP["upsample\n8 kHz → 16 kHz"]
        PW_OUT_BRIDGE["pw_stream\nDirection::Output\nnode: twilio-caller"]

        WS_IN --> B64_DEC --> MULAW_DEC --> UPSAMP --> PW_OUT_BRIDGE

        GREETING_WAV["Greeting WAV\n(Kokoro, pre-generated\nmu-law at startup)"]
        OUT_BUF["Outbound Buffer"]

        GREETING_WAV -->|"pushed on call start"| OUT_BUF

        UNIX_SOCK_RX["Unix socket RX\n/tmp/jarvina-tts.sock"]
        DOWNSAMP["downsample\n16 kHz → 8 kHz"]
        MULAW_ENC["mu-law encode\n⚙ Rust (no GStreamer)"]
        B64_ENC["base64 encode → JSON"]
        WS_OUT["WebSocket TX"]

        UNIX_SOCK_RX --> OUT_BUF --> DOWNSAMP --> MULAW_ENC --> B64_ENC --> WS_OUT
    end

    %% ─── PIPEWIRE MATRIX ─────────────────────────────────────────────────────
    subgraph PW["PipeWire Matrix  ·  Jetson"]
        PW_ROUTER["PipeWire Router"]
        PW_IN_JARVINA["pw_stream\nDirection::Input\nnode: jarvina-rust"]
        PW_OUT_JARVINA["pw_stream\nDirection::Output\nnode: jarvina-rust-out"]
        SONY["Sony Speaker\n(when connected)"]

        PW_OUT_BRIDGE -->|"F32LE @ 16 kHz"| PW_ROUTER
        PW_ROUTER -->|"inbound audio"| PW_IN_JARVINA
        PW_OUT_JARVINA -->|"local playback"| PW_ROUTER
        PW_ROUTER --> SONY
    end

    %% ─── JARVINA VOICE AGENT ─────────────────────────────────────────────────
    subgraph JARVINA["Jarvina Voice Agent  ·  Rust"]
        VAD["VAD\nSilero"]
        STT["STT\nParakeet CUDA"]
        LLM["LLM\nClaude Haiku API"]
        TTS["TTS\nKokoro CUDA"]
        RESAMP["resample\n24 kHz → 16 kHz\n(streaming callback)"]
        UNIX_SOCK_TX["Unix socket TX\n/tmp/jarvina-tts.sock\n⚠ bypasses PipeWire"]
        PW_LOCAL["pw_stream\nDirection::Output\n(local monitor)"]

        PW_IN_JARVINA -->|"F32LE @ 16 kHz"| VAD
        VAD -->|"speech frames"| STT
        STT -->|"transcript"| LLM
        LLM -->|"response text"| TTS
        TTS -->|"F32LE @ 24 kHz"| RESAMP
        RESAMP -->|"F32LE @ 16 kHz"| UNIX_SOCK_TX
        RESAMP -->|"F32LE @ 16 kHz"| PW_LOCAL
        PW_LOCAL --> PW_OUT_JARVINA
    end

    %% ─── EXTERNAL CONNECTIONS ────────────────────────────────────────────────
    CALLER <-->|"voice"| PSTN
    PSTN <-->|"MULAW/8kHz"| TWILIO_CLOUD
    TWILIO_CLOUD <-->|"WebSocket\nMedia Stream"| WS_IN
    WS_OUT --> TWILIO_CLOUD

    UNIX_SOCK_TX -->|"outbound path\nbypasses PipeWire\navoids capture starvation"| UNIX_SOCK_RX

    %% ─── STYLES ──────────────────────────────────────────────────────────────
    classDef rustNode   fill:#8B2500,color:#FFD0B0,stroke:#FF6030
    classDef pwNode     fill:#003D6B,color:#B0D8FF,stroke:#4499CC
    classDef aiNode     fill:#1A4D00,color:#B0FFB0,stroke:#44CC44
    classDef phoneNode  fill:#3D2B00,color:#FFE0A0,stroke:#CCAA44
    classDef socketNode fill:#4B0082,color:#E0C0FF,stroke:#9955CC

    class WS_IN,B64_DEC,MULAW_DEC,UPSAMP,PW_OUT_BRIDGE,OUT_BUF,DOWNSAMP,MULAW_ENC,B64_ENC,WS_OUT,GREETING_WAV rustNode
    class PW_ROUTER,PW_IN_JARVINA,PW_OUT_JARVINA,SONY pwNode
    class VAD,STT,LLM,TTS,RESAMP,PW_LOCAL aiNode
    class CALLER,PSTN,TWILIO_CLOUD phoneNode
    class UNIX_SOCK_TX,UNIX_SOCK_RX socketNode
```

## Architecture Notes

| Concern | Decision |
|---|---|
| Codec | Mu-law encode/decode in pure Rust — no GStreamer |
| Inbound routing | PipeWire only: `twilio-caller` → `jarvina-rust` |
| Outbound routing | Unix domain socket `/tmp/jarvina-tts.sock` — bypasses PipeWire entirely |
| Why bypass PipeWire outbound? | Avoids capture starvation: PipeWire output would loop back into the capture graph and starve the inbound VAD pipeline |
| Local speaker | Parallel `jarvina-rust-out` pw_stream feeds Sony speaker independently — does not touch phone path |
| Greeting | Pre-generated Kokoro WAV, converted to mu-law, loaded at bridge startup, pushed into outbound buffer on call connect |
| TTS sample rate | Kokoro outputs 24 kHz F32LE; streaming callback resamples to 16 kHz before fork to socket + local pw_stream |
| CUDA | Parakeet STT and Kokoro TTS both run on Jetson GPU via CUDA |
