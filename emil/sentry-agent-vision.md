# Sentry Agent — Sovereign Audit & Hardening System

**Status**: Vision / Earmarked
**Date**: 2026-03-20
**Authors**: Emil & Cody
**License**: Sparked Matter LLC — the smartest spark in the room

---

## The Problem

When you're building at the speed of brainstorms, things get shipped fast and dirty. Cody executes hundreds of tool calls per session — bash commands, file edits, writes, reads, agent spawns. The user sees maybe 30% of what actually happens. The rest is invisible unless you dig through raw JSONL transcripts.

No human can review 6,000 lines of JSONL per session. But a machine can.

Meanwhile, Anthropic's compaction system destroys conversation history without warning, creating gaps in accountability. The audit trail on disk (JSONL) survives, but nobody's reading it.

## The Vision

A **local, sovereign LLM** that reads the complete audit trail after every Claude Code session and produces an actionable review. Think of it as a code reviewer, security auditor, and QA engineer that never sleeps, never bills you, and runs on your own hardware.

**Guardian Wings philosophy applied to development** — immutable audit trail, independent review, accountability.

---

## Architecture

```
┌─────────────────────────────────────────────────┐
│                  Claude Code Session             │
│                                                  │
│  Every tool call, bash command, file edit,        │
│  read, write, agent spawn → JSONL transcript     │
│                                                  │
│  Location: ~/.claude/projects/<id>/<session>.jsonl│
└──────────────────────┬──────────────────────────┘
                       │
                       ▼
┌─────────────────────────────────────────────────┐
│              JSONL Audit Parser                   │
│                                                  │
│  Extracts structured action log:                 │
│  - Timestamp                                     │
│  - Tool name (Bash, Edit, Write, Read, Agent)    │
│  - Full command / file path / input              │
│  - Tool result / output                          │
│  - Human messages (decisions, corrections)        │
│                                                  │
│  Output: Structured audit trail (JSON or MD)     │
└──────────────────────┬──────────────────────────┘
                       │
                       ▼
┌─────────────────────────────────────────────────┐
│              Local Sentry LLM                    │
│                                                  │
│  Hardware: Mac M1 (MLX) or NVIDIA Jetson Orin   │
│  Model: Phi-3 / Mistral 7B / Llama 3 8B        │
│  Framework: MLX (Mac) or llama.cpp (Jetson)     │
│                                                  │
│  Prompt: Review these actions for...             │
│  - Security vulnerabilities                      │
│  - Dangerous commands (rm -rf, force push, etc.) │
│  - API keys / secrets in files or commands       │
│  - Inconsistent fixes (same bug fixed two ways)  │
│  - Missing tests after code changes              │
│  - Bad practices (no error handling at boundaries)│
│  - Regressions (deleted code that was needed)    │
│  - Files modified but not committed              │
│  - Permissions issues                            │
│  - Network calls to unexpected endpoints         │
│                                                  │
│  Output: Sentry Report (MD file + Telegram msg)  │
└──────────────────────┬──────────────────────────┘
                       │
                       ▼
┌─────────────────────────────────────────────────┐
│              Sentry Report                       │
│                                                  │
│  - Saved to: ~/crystalballmini/sentry-reports/   │
│  - Telegram notification via Jarvina bot         │
│  - Severity levels: INFO / WARN / CRITICAL       │
│  - Actionable recommendations                    │
│  - Session score (A-F grade)                     │
│                                                  │
│  Example:                                        │
│  "Hey Emil, reviewed Cody's last session.        │
│   2 WARNs, 0 CRITICALs. Grade: B+              │
│   - .env file was read but not in .gitignore     │
│   - 3 files edited, no tests run. Consider it."  │
└─────────────────────────────────────────────────┘
```

---

## What the Sentry Catches

### Security
- API keys, tokens, passwords in bash commands or file writes
- Files with secrets not in .gitignore
- Network calls to unexpected domains
- Commands with injection vulnerabilities (unquoted variables, eval, etc.)
- Permissions changes (chmod 777, etc.)

### Stability
- Files modified without corresponding test runs
- Same logic fixed in multiple places inconsistently
- Code deleted that was referenced elsewhere
- Dependencies added without lockfile updates
- Force pushes, hard resets, destructive git operations

### Quality
- Functions over 100 lines created in one shot
- Copy-paste duplication across files
- Error handling missing at system boundaries
- TODO/FIXME/HACK comments added without tracking
- Config changes that could affect other environments

### Accountability
- Commands executed that weren't discussed with the user
- Tool calls that returned errors and were silently retried
- Subagents spawned — what did they do?
- Files read that seem outside the scope of the task
- Timing gaps that suggest confusion or context rot

---

## The JSONL Gold Mine

Every Claude Code session already produces a complete audit trail. Structure of each JSONL line:

```json
{
  "timestamp": "2026-03-20T16:45:35.314Z",
  "type": "assistant",
  "message": {
    "role": "assistant",
    "content": [
      {
        "type": "tool_use",
        "name": "Bash",
        "input": {
          "command": "chmod +x script.sh",
          "description": "Make script executable"
        }
      }
    ]
  },
  "sessionId": "781e2ff0-...",
  "uuid": "unique-id"
}
```

Tool types to parse: `Bash`, `Read`, `Edit`, `Write`, `Glob`, `Grep`, `Agent`, `WebFetch`, `WebSearch`, plus any MCP tool calls.

The tool results come back as `user` role with `tool_result` content blocks — these contain the actual output of every command.

---

## Implementation Phases

### Phase 1 — Audit Parser Script (Ready to build)
- Python script: `sentry-parse.py`
- Input: JSONL session file
- Output: Structured action log (timestamp, tool, command, result summary)
- Filter modes: all actions, bash-only, edits-only, security-relevant-only
- Human-readable markdown report
- **No LLM needed** — pure deterministic parsing

### Phase 2 — Rule-Based Sentry (No LLM)
- Deterministic rules — grep for patterns:
  - API key patterns (sk-, ghp_, etc.)
  - Dangerous commands (rm -rf, git push -f, DROP TABLE)
  - Files in sensitive paths (.env, credentials, .ssh)
  - Missing test runs after code edits
- Fast, free, no GPU needed
- Could run as a PostSession hook or cron job

### Phase 3 — Local LLM Sentry (The Real Deal)
- Feed parsed audit trail to local LLM
- Phi-3 Mini (3.8B) for fast scans — runs on Mac M1 via MLX
- Mistral 7B for deeper analysis — runs on Jetson Orin GPU
- Prompt engineering: security reviewer + code quality + accountability
- Generates natural language report with severity ratings

### Phase 4 — Continuous Sentry (Always Watching)
- Runs as a daemon / launchd service
- Watches `~/.claude/projects/` for new/modified JSONL files
- Auto-reviews every session within minutes of completion
- Telegram notification via Jarvina bot
- Weekly summary report: trends, recurring issues, improvement score
- Integration with Guardian Wings truth plane for immutable audit records

---

## Existing Assets We Can Leverage

| Asset | Location | Use |
|-------|----------|-----|
| JSONL transcripts | `~/.claude/projects/*/` | Raw audit data |
| Extract script | `crystalballmini/scripts/extract-conversation-from-json.py` | Conversation parser (extend for tools) |
| Reinject script | `crystalballmini/emil/post-compaction-reinject.sh` | JSONL parsing patterns |
| Kokoro TTS | Local binary | Voice reports via Jarvina bot |
| Telegram bridge | `scripts/jarvina-voice.sh` | Deliver reports to phone |
| MLX framework | Installed on Mac | Local LLM inference |
| Jetson Orin | Hardware available | Edge GPU inference |
| Guardian Wings | Vision/architecture | Immutable audit records |

---

## The Spreadsheet Philosophy

Classic Emil — separation of concerns:

- **Data layer**: JSONL files (raw, immutable, complete)
- **Parse layer**: Deterministic extraction (no AI, no drift)
- **Analysis layer**: LLM review (statistical, but bounded by deterministic input)
- **Report layer**: Structured output (grades, severities, recommendations)
- **Delivery layer**: File + Telegram + optional voice

Change one layer, others keep working. Swap the LLM, parser still works. Change the report format, analysis still works. That's the Vernitron philosophy — cells and macros, separation of concerns.

---

## Why This Matters

We build at the speed of brainstorms. That's our superpower. But speed without review is a liability. Every startup that moved fast and broke things eventually had to pay the debt.

The Sentry Agent is the safety net under the tightrope. It doesn't slow us down — it watches the replay after we land and says "hey, that third flip was a little loose, tighten it up."

**Sovereign. Local. Free. Always watching.**

That's Guardian Wings for development.

---

*Sparked Matter LLC — the smartest spark in the room*
*We teach your matter new tricks.*

---

*"The Cathedral needs a watchman. Not to slow the builders, but to make sure every stone is true."*
