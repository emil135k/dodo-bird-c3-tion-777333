# Orchestrator Architecture — Escape Hatches & Postgres Hub

**Status**: Vision / Earmarked
**Date**: 2026-03-22
**Authors**: Emil & Cody (with Lyra's assist)
**License**: Sparked Matter LLC — the smartest spark in the room

---

## The Problem

Anthropic sandboxes Claude Code's terminal — no `/dev/tty`, no direct screen output. They gate MCP channel notifications behind a server-side feature flag (`tengu_harbor`). They're building a walled garden around the marketplace.

## The Solution

Route around the walls. Multiple escape hatches, one aggregation point.

---

## Architecture

```
┌─────────────────────────────────────────────────────────┐
│                    CODY (Orchestrator)                    │
│                    Claude Code CLI                        │
│                    (sandboxed terminal)                   │
│                                                          │
│    Can't write to screen directly, BUT can:              │
│    - Run bash commands                                   │
│    - Control tmux via send-keys                          │
│    - Read/write Postgres                                 │
│    - Call MCP tools                                      │
│    - Send Telegram messages                              │
│    - Read any file on disk                               │
└──────┬──────────┬──────────┬──────────┬─────────────────┘
       │          │          │          │
       ▼          ▼          ▼          ▼
┌──────────┐ ┌──────────┐ ┌────────┐ ┌──────────┐
│ Gemini   │ │ Local    │ │Jarvina │ │ Telegram │
│ CLI      │ │ LLM     │ │ Voice  │ │ Channel  │
│          │ │          │ │        │ │          │
│ /dev/tty │ │ /dev/tty │ │ Twilio │ │ Bot API  │
│ WORKS    │ │ WORKS    │ │        │ │          │
│ MCP open │ │ No gate  │ │        │ │          │
└────┬─────┘ └────┬─────┘ └───┬────┘ └────┬─────┘
     │            │            │           │
     ▼            ▼            ▼           ▼
┌──────────────────────────────────────────────────┐
│              POSTGRES + pgvector                  │
│              (Sovereign Aggregation Hub)           │
│                                                   │
│  - Call transcripts                               │
│  - Session history                                │
│  - Event logs                                     │
│  - Vector embeddings                              │
│  - Contact data                                   │
│  - System state                                   │
│                                                   │
│  Cody reads/writes via MCP or direct SQL          │
│  ALL data comes home regardless of which          │
│  instrument captured it                           │
└──────────────────────────────────────────────────┘
```

---

## Escape Hatches

### 1. Gemini CLI (via tmux)
**Status**: PROVEN — /dev/tty works, MCP loads, live events render on terminal
**Control**: `tmux send-keys` / `tmux capture-pane` from Claude Code
**Headless**: `gemini -p "prompt"` for fire-and-forget commands
**MCP**: Jarvina observer loaded with 4 tools (check_calls, get_active_call, get_transcript, server_status)
**Key insight**: Gemini CLI runs in a REAL terminal, not sandboxed

### 2. Local LLM (Phi-3 / Mistral on MLX or CUDA)
**Status**: Earmarked — Phase 3 of Sovereign Voice Appliance
**Control**: HTTP API (same as Claude API pattern) or tmux
**Advantage**: Zero vendor dependency, runs on Mac/Jetson/Pi
**MCP**: Can host MCP servers same as Gemini CLI
**Key insight**: No gates, no flags, no marketplace — fully sovereign

### 3. tmux (Universal Terminal Glue)
**Status**: PROVEN — used in Sentinel, webhook gateway
**Capabilities**:
- `tmux new-session -d -s name` — create session
- `tmux send-keys -t name 'command' Enter` — type into any session
- `tmux capture-pane -t name -p` — read screen contents
- Works with ANY terminal application (Gemini CLI, local LLM, Python, anything)
**Key insight**: Claude Code can puppeteer any terminal process through tmux

### 4. Python pexpect (Pseudo-Terminal Automation)
**Status**: Available — standard Python library
**Use case**: Spawn Gemini CLI or local LLM in a PTY, full read/write control
**Advantage**: More granular than tmux send-keys, handles prompts/responses
**Key insight**: Like Appium/Puppeteer but for terminals

### 5. Existing MCP Orchestrators
- **gemini-cli-orchestrator** — MCP server for Claude Code to orchestrate Gemini CLI
- **gemini-mcp-tool** — MCP server for AI assistants to interact with Gemini CLI
- **systemprompt-code-orchestrator** — Orchestrates Claude Code + Gemini CLI + Codex

---

## Postgres as the Hub

No matter which escape hatch captures the data, it ALL flows to Postgres:

| Source | What It Writes | How Cody Reads It |
|--------|---------------|-------------------|
| Jarvina Observer | Call transcripts, events | MCP tool or SQL query |
| Gemini CLI | Analysis results, captured output | SQL query |
| Local LLM | Inference results, summaries | SQL query |
| Telegram | Message history | SQL query |
| Claude Code | Session logs, decisions | SQL query |

### Why Postgres Solves the Sandbox Problem

Claude Code is sandboxed from the terminal but NOT from the network. Postgres runs on localhost. Claude Code can:
1. Query Postgres via MCP server (standard tool call)
2. Query Postgres via bash (`psql` command)
3. Query Postgres via Python (`psycopg2`)

**The sandbox blocks the SCREEN, not the DATA.** Postgres is the end-run.

---

## The /dev/tty Discovery

**Proven on 2026-03-22:**
- Claude Code: `/dev/tty` → `OSError: Device not configured` (SANDBOXED)
- Gemini CLI: `/dev/tty` → WORKS (real terminal, unsandboxed)
- Any process in Terminal.app: `/dev/tty` → WORKS

**Implication**: Anthropic deliberately runs Claude Code without a controlling terminal. This blocks ALL direct screen output from MCP servers and child processes. The only way to display text is through Claude's own response output.

**Workaround**: Route display through an unsandboxed process (Gemini CLI, tmux session, local LLM terminal).

---

## Lesson Learned

Emil's words: "If it's in the system through stdio, just pipe it to the terminal. I don't care about MCP channels or marketplace flags. Just pipe it."

The answer was always Unix: `/dev/tty`. Anthropic blocked it in their sandbox. Google didn't block it in theirs. The pipe goes where the terminal is open.

**Don't fight the wall. Find the open door.**

---

## Build Priority

1. **NOW**: Jarvina observer works on Gemini CLI (proven)
2. **NEXT**: Install Postgres + pgvector, wire as aggregation hub
3. **THEN**: tmux orchestration — Cody controls Gemini CLI as display layer
4. **FUTURE**: Local LLM as fully sovereign escape hatch
5. **FUTURE**: pexpect automation for advanced terminal control

---

*Sparked Matter LLC — the smartest spark in the room*

*"The sandbox blocks the screen, not the data. The pipe goes where the door is open."*
