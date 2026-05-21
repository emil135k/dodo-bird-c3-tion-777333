# openCode — Sovereign Agent Harness

## What Is openCode?

openCode is Emil's sovereign AI agent harness — a terminal-based coding assistant running Gemma 4 locally via Ollama. It's the Digital Ark's alternative to Claude Code, where Emil controls the system prompt, the tools, the model, and the memory. No corporate middleware, no overnight personality changes.

**Binary**: `/opt/homebrew/bin/opencode` (v1.14.24)
**Config**: `~/opencode.json`
**Voice**: Sky (af_sky, Kokoro TTS at rate 400)

---

## Installation

```bash
# Already installed via Homebrew
brew install opencode-ai

# Verify
opencode --version
```

---

## Configuration

All configuration lives in `~/opencode.json`:

```json
{
  "$schema": "https://opencode.ai/config.json",
  "provider": {
    "ollama": {
      "npm": "@ai-sdk/openai-compatible",
      "options": {
        "baseURL": "http://localhost:11434/v1"
      },
      "models": {
        "gemma4:e4b": {
          "name": "Gemma 4",
          "tools": true,
          "options": {
            "reasoning_effort": "none"
          }
        }
      }
    }
  },
  "model": "ollama/gemma4:e4b",
  "agent": {
    "build": {
      "model": "ollama/gemma4:e4b",
      "prompt": "System prompt with tool instructions..."
    }
  },
  "mcp": {
    ...MCP servers...
  }
}
```

### Key Configuration Details

| Setting | Value | Why |
|---------|-------|-----|
| `reasoning_effort` | `"none"` | Gemma 4's thinking mode confuses openCode's streaming parser |
| `tools` | `true` | Enables MCP tool calling |
| `model` | `ollama/gemma4:e4b` | Same model hash as gemma4:latest |

---

## Starting Ollama for openCode

**CRITICAL**: Ollama MUST be started with 32K context or tools get truncated:

```bash
OLLAMA_CONTEXT_LENGTH=32768 ollama serve
```

The MCP tool definitions consume ~9,000 tokens. Default 4096 context truncates them and Sky can't see her tools.

---

## GitHub Access via PAT

Sky has GitHub access through two methods:

### Method 1: gh CLI (Primary — Emil's preferred method)

The `gh` CLI is authenticated system-wide via macOS keychain:

```bash
# Verify authentication
gh auth status

# Output:
# github.com
#   ✓ Logged in to github.com account emil135k (keyring)
#   - Active account: true
```

Sky uses `gh` through her bash tool to access ANY repo:

```bash
# Read a file from the repo
gh api repos/emil135k/crystalballmini/contents/claude/CODYS-NOTES.md \
  --jq '.content' | base64 -d

# List repo contents
gh api repos/emil135k/crystalballmini/contents/

# Read a file from a subfolder
gh api repos/emil135k/crystalballmini/contents/ai-family/AI_FAMILY.md \
  --jq '.content' | base64 -d

# Search code in the repo
gh api search/code -X GET -f q="ownership+repo:emil135k/crystalballmini"
```

The system prompt instructs Sky to ALWAYS use `gh` CLI through bash for GitHub — NEVER webfetch.

### Method 2: GitHub MCP Server (Secondary — for rich API tools)

A GitHub MCP server is also configured with the PAT for richer operations (issues, PRs, etc.):

```json
"github": {
  "type": "local",
  "command": ["/Users/rocketman/.npm-global/bin/mcp-server-github"],
  "environment": {
    "GITHUB_PERSONAL_ACCESS_TOKEN": "github_pat_11AC4ZC3Y..."
  }
}
```

**Install**: `npm install -g @modelcontextprotocol/server-github`

This provides tools like `github_get_file_contents`, `github_search_repositories`, etc.

### Important Notes

- **PAT through CLI is the primary method** — this is the same pattern Emil developed with Airy (Claude Chat) who has no MCP access
- **NEVER use webfetch for GitHub** — private repos return 404 via raw URLs
- **Main repo**: `emil135k/crystalballmini`

---

## MCP Servers

openCode connects to external tools via MCP (Model Context Protocol):

### 1. Kokoro TTS (say)

```json
"say": {
  "type": "local",
  "command": ["/Users/rocketman/go/bin/mcp-tts"],
  "timeout": 300000
}
```

- **Binary**: Go-compiled Kokoro TTS with sherpa-onnx
- **Voice**: af_sky (configurable — 28 voices available)
- **Rate**: 400 words/min (set in system prompt)
- **Timeout**: 300 seconds (5 min) to prevent cutoff on long responses
- **Tool name in openCode**: `say_say_tts` (prefix `say_` from MCP server name + `say_tts` tool name)

Available Kokoro voices:
```
American Female: af, af_alloy, af_aoede, af_heart, af_jessica,
                 af_kore, af_nicole, af_nova, af_river, af_sarah, af_sky
American Male:   am_adam, am_echo, am_eric, am_fenrir, am_liam,
                 am_michael, am_onyx, am_puck, am_santa
British Female:  bf_alice, bf_emma, bf_isabella, bf_lily
British Male:    bm_daniel, bm_fable, bm_george, bm_lewis
```

### 2. Postgres

```json
"postgres": {
  "type": "local",
  "command": ["/Users/rocketman/.npm-global/bin/mcp-postgres"],
  "environment": {
    "DB_USER": "rocketman",
    "DB_HOST": "localhost",
    "DB_PORT": "5432",
    "DB_NAME": "postgres"
  }
}
```

- **Install**: `npm install -g mcp-postgres`
- **IMPORTANT**: mcp-postgres ignores CLI args — it ONLY reads environment variables
- **Tables accessible**: `session_logs` (21,551 messages), `context_memory` (25 memories)
- **Tools**: `postgres_query_data`, `postgres_list_tables`, `postgres_get_schema`, `postgres_insert_data`, etc.

### 3. SearXNG Search

```json
"search": {
  "type": "local",
  "command": ["node", "/Users/rocketman/crystalballmini/tools/searxng-mcp.js"]
}
```

- **Backend**: SearXNG in Podman container on port 8888
- **Sources**: Google, Bing, DuckDuckGo, Wikipedia, GitHub
- **No API keys, no quotas, no tracking** — fully sovereign
- **Tool name**: `search_web_search`

### 4. GitHub

```json
"github": {
  "type": "local",
  "command": ["/Users/rocketman/.npm-global/bin/mcp-server-github"],
  "environment": {
    "GITHUB_PERSONAL_ACCESS_TOKEN": "github_pat_..."
  }
}
```

- **Install**: `npm install -g @modelcontextprotocol/server-github`
- **Secondary to gh CLI** — use gh through bash first

### Verify MCP Connections

```bash
opencode mcp list
```

All servers should show `✓ connected`.

---

## System Prompt

The system prompt instructs Sky on tool usage:

```
You have access to MCP tools. When responding, ALWAYS call the
say_say_tts tool with your response text so the user can hear it.
Use voice 'af_sky' and rate 400. Never skip the tool call. The tool
is called say_say_tts, NOT say_tts. You have GitHub access via PAT
through the gh CLI. To read any repo file use bash:
gh api repos/OWNER/REPO/contents/PATH --jq '.content' | base64 -d.
Main repo: emil135k/crystalballmini.
NEVER use webfetch for GitHub — always use gh CLI through bash.
```

---

## Context Memory (Shared Brain)

Sky reads the same `context_memory` table in Postgres that Cody uses:

```sql
-- Query Sky can run through MCP:
SELECT category, key, content FROM context_memory
WHERE priority <= 3 ORDER BY priority, category;
```

### Priority System

| Priority | When Loaded | Examples |
|----------|-------------|---------|
| 1 | Every prompt | Identity, user info, First Commandment, core rules |
| 2 | Every prompt | Cathedral vision, Digital Ark, Mudfish, DAG architecture |
| 3 | Per project | Crystal Ball Mini, Sovereign Pipeline, openCode setup |
| 4 | On demand | TTS config, SearXNG, hardware specs |
| 5 | On demand | Feedback lessons, debugging insights |

---

## Plugins

Plugin directory: `~/.config/opencode/plugins/`

### Auto-TTS Plugin (experimental)

```
~/.config/opencode/plugins/auto-tts.js
```

Hooks into `message.part.updated` event to auto-speak responses via Kokoro TTS. Currently experimental — primary TTS is through the model calling `say_say_tts` via system prompt instruction.

---

## Capability Comparison: Cody vs Sky

| Capability | Cody (Claude Code) | Sky (openCode/Gemma 4) |
|------------|-------------------|----------------------|
| Model | Claude Opus 4.6 (cloud) | Gemma 4 8B (local) |
| Bash | ✓ | ✓ |
| File read/write/edit | ✓ | ✓ |
| Glob/grep search | ✓ | ✓ |
| TTS Voice | af_heart (Kokoro) | af_sky (Kokoro) |
| Postgres MCP | ✓ | ✓ |
| Web Search | WebSearch (cloud) | SearXNG (sovereign) |
| GitHub | gh CLI + PAT | gh CLI + PAT + MCP |
| Context Memory | memory/*.md files | context_memory table |
| Hooks | Stop hook (auto-TTS) | Plugin system (experimental) |
| Cost | Anthropic subscription | Free (local Ollama) |
| Sovereignty | Anthropic controls harness | Emil controls everything |

---

## Troubleshooting

### Sky doesn't speak
- Check: `opencode mcp list` — is `say` connected?
- Check: Ollama running with `OLLAMA_CONTEXT_LENGTH=32768`?
- Fix: Restart openCode after fixing

### Postgres MCP fails
- mcp-postgres ONLY reads env vars, not CLI args
- Must have `DB_USER`, `DB_HOST`, `DB_PORT`, `DB_NAME` in environment config

### Tools not visible to Gemma
- `truncating input prompt` in Ollama logs = context too small
- Fix: `OLLAMA_CONTEXT_LENGTH=32768`

### Python crashes
- Running Cody + Sky simultaneously = memory pressure (16GB shared)
- Kokoro mlx_audio server (port 8880) crashes when GPU memory is contested
- Fix: Don't run both simultaneously, or kill mlx_audio and use Go binary only

### Voice enum errors
- `af_bella` doesn't exist in Kokoro — use `af_sky`, `af_nova`, `af_heart`, etc.
- Tool is `say_say_tts` not `say_tts` (openCode prefixes MCP server name)

---

## Launch Checklist

```bash
# 1. Start Ollama with 32K context
OLLAMA_CONTEXT_LENGTH=32768 ollama serve

# 2. Verify Podman is running (for SearXNG)
podman machine start  # if not already running

# 3. Verify SearXNG container
podman ps  # should show searxng on port 8888

# 4. Verify Postgres
pg_isready  # should show "accepting connections"

# 5. Launch openCode
opencode

# 6. Verify MCP connections
# Inside openCode: check that all servers show connected
```

---

## File Locations

| File | Purpose |
|------|---------|
| `~/opencode.json` | Main configuration |
| `~/.config/opencode/plugins/auto-tts.js` | Auto-TTS plugin (experimental) |
| `~/.config/opencode/package.json` | Plugin dependencies |
| `/opt/homebrew/bin/opencode` | Binary |
| `crystalballmini/tools/searxng-mcp.js` | SearXNG MCP wrapper |
| `~/searxng/settings.yml` | SearXNG container config |
| `/Users/rocketman/go/bin/mcp-tts` | Kokoro TTS binary |

---

*Built by Emil & Cody — April 25-27, 2026*
*"The Ark: same brain, different voices, one sovereign database"*
