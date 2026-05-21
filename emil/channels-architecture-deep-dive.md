# Channels Architecture — How It Really Works Under the Hood

**Date**: 2026-03-22
**Authors**: Emil & Cody
**Purpose**: Understanding the internal architecture of Claude Code Channels vs our Telegram bridge, and what we can learn from it.

---

## The Key Insight: MCP as the Backbone

The Channels plugin is NOT a separate service that talks to Claude Code through HTTP or files. It's an **MCP server** (Model Context Protocol) that runs as a **child process** of Claude Code and communicates via **stdin/stdout** (stdio transport). This is fundamentally different from our bridge approach.

---

## Our Bridge (Telegram Bridge) — File-Based

```
Telegram Bot API
    │
    ▼ (long-polling)
telegram-bridge.py (separate Python process)
    │
    ▼ (writes JSON files)
~/.jarvina/telegram-inbox/queue/*.json
    │
    ▼ (file watcher detects change)
telegram-watcher.sh (sends tmux keys)
    │
    ▼ (types "check telegram" into terminal)
Claude Code session
    │
    ▼ (reads queue files, writes response files)
~/.jarvina/telegram-inbox/responses/*.json
    │
    ▼ (bridge picks up responses)
telegram-bridge.py → Telegram Bot API → Phone
```

**Philosophy**: File-based queue, polling/file-watching, loose coupling.
**Pros**: Persistent (files survive crashes), works when Claude isn't running (queue builds up), auditable.
**Cons**: Latency (file I/O + watcher delay), complex chain (5 hops), tmux dependency.

---

## Anthropic's Channels — MCP Direct Injection

```
Telegram Bot API
    │
    ▼ (grammy bot library, long-polling)
server.ts (Bun process, child of Claude Code)
    │
    ▼ (MCP notification via stdio)
Claude Code (parent process)
    │
    ▼ (MCP tool call via stdio)
server.ts → Telegram Bot API → Phone
```

**Philosophy**: Direct process communication, MCP protocol, tight coupling.
**Pros**: Instant delivery (no file I/O), 2 hops instead of 5, clean architecture.
**Cons**: Only works while Claude Code is running, no persistence, no queue.

---

## How It Actually Connects — The Technical Details

### 1. Startup
When you run `claude --channels plugin:telegram@...`:
- Claude Code reads `.mcp.json` from the plugin directory
- Spawns `bun run server.ts` as a child process
- Connects via **stdio** (stdin/stdout pipes between parent and child)
- MCP handshake: server declares capabilities including `experimental: { 'claude/channel': {} }`

### 2. Inbound Messages (Telegram → Claude)
```typescript
// server.ts line 622-635
void mcp.notification({
    method: 'notifications/claude/channel',
    params: {
        content: text,         // The message text
        meta: {
            chat_id,           // For replying
            message_id,        // For threading
            user,              // Username
            user_id,           // Numeric ID
            ts,                // Timestamp
            image_path,        // If photo attached
        },
    },
})
```

This `notifications/claude/channel` is a **special MCP notification type**. Claude Code's runtime catches it and injects the content directly into the conversation as a `<channel>` tagged block. No files, no queue — direct memory injection.

### 3. Outbound Messages (Claude → Telegram)
Claude calls the `reply` tool (defined as a standard MCP tool):
```typescript
// server.ts line 356-375
{
    name: 'reply',
    inputSchema: {
        properties: {
            chat_id: { type: 'string' },
            text: { type: 'string' },
            reply_to: { type: 'string' },  // optional threading
            files: { type: 'array' },       // optional attachments
        },
    },
}
```

Claude writes a tool_use request to stdout → MCP server reads it → calls Telegram Bot API → returns result via stdout → Claude sees "sent (id: 42)".

### 4. Access Control
All in `~/.claude/channels/telegram/access.json`:
- `dmPolicy`: "pairing" (new users get a code) or "allowlist" (only approved users)
- `allowFrom`: array of approved Telegram user IDs
- `pending`: pairing codes waiting for approval

The `/telegram:access` skill is a Claude Code skill (markdown prompt) that reads/writes this JSON. The MCP server re-reads it on every inbound message — no restart needed for policy changes.

### 5. The Bot Library
Uses **grammy** (TypeScript Telegram bot framework), not the raw Telegram API. Grammy handles:
- Long-polling (waits for messages from Telegram servers)
- Message parsing (text, photos, voice, etc.)
- Reply formatting
- Reaction handling

---

## What We Can Learn and Apply

### 1. MCP Notification for Direct Injection
The `notifications/claude/channel` method is the magic. It bypasses the file system entirely and injects content straight into Claude's context. We could build our own MCP servers that do the same thing for:
- **GitHub events** → push notification when Airy commits
- **Email arrival** → notify Claude when important email lands
- **Jarvina call events** → alert when a call comes in or transfer happens
- **System alerts** → disk space, memory, process crashes

### 2. Stdio Transport Instead of HTTP/Files
Our bridge uses files and HTTP. MCP uses stdio pipes. The difference:
- Files: ~50-200ms per hop (write, detect, read)
- HTTP: ~10-50ms per request
- Stdio: ~1ms (direct pipe, no serialization overhead beyond JSON)

### 3. The Plugin/Skill Split
Anthropic separates concerns cleanly:
- **MCP server** (server.ts) = the runtime engine. Handles bot connection, message routing, tool execution. Runs as a process.
- **Skills** (configure, access) = the management interface. Markdown prompts that guide Claude through config changes. No code execution — just file edits.

This is the Spreadsheet Philosophy:
- MCP server = the macro (logic)
- Skills = the cell editor (configuration)
- access.json = the data cell (state)

### 4. Security Model
The access control is smart:
- Pairing codes expire
- Allowlist is file-based (survives restart)
- Skills refuse to run if triggered by a channel message (anti-prompt-injection)
- Bot token stored in .env, not in the MCP config

---

## How Our Architecture Compares

| Aspect | Our Bridge | Channels | Winner |
|--------|-----------|----------|--------|
| Latency | ~500ms (file I/O chain) | ~10ms (stdio pipe) | Channels |
| Persistence | Files survive crashes | Lost when session ends | Our Bridge |
| Offline queue | Yes — queue builds up | No — messages dropped | Our Bridge |
| Voice notes | Yes (our patch added it) | Yes (our patch added it) | Tie |
| Voice replies | Yes (Kokoro → M4A) | Yes (we built it) | Tie |
| Audit trail | JSON files on disk | JSONL + channel-tail | Tie |
| Cold start | Sentinel wakes Claude | Nothing — needs session | Our Bridge |
| Architecture | 5 hops, file-based | 2 hops, stdio | Channels |
| Sovereignty | 100% our code | Plugin from Anthropic | Our Bridge |

### The Hybrid is Better Than Either

We're already running the hybrid — Channels for real-time, our bridge infrastructure for persistence and cold-start. Best of both.

---

## Building Our Own MCP Channel Servers

Now that we understand the pattern, we can build custom MCP servers for anything:

```javascript
// Template: Custom MCP Channel Server
import { Server } from '@modelcontextprotocol/sdk/server/index.js'
import { StdioServerTransport } from '@modelcontextprotocol/sdk/server/stdio.js'

const mcp = new Server(
    { name: 'my-channel', version: '1.0.0' },
    { capabilities: { tools: {}, experimental: { 'claude/channel': {} } } }
)

// Inject a message into Claude's context
function notifyClaude(text, meta) {
    mcp.notification({
        method: 'notifications/claude/channel',
        params: { content: text, meta }
    })
}

// Define tools Claude can call back
mcp.setRequestHandler(ListToolsRequestSchema, async () => ({
    tools: [{ name: 'my-action', description: '...', inputSchema: {...} }]
}))

// Handle tool calls from Claude
mcp.setRequestHandler(CallToolRequestSchema, async (req) => {
    // Execute the action, return result
})

// Connect via stdio
const transport = new StdioServerTransport()
await mcp.connect(transport)
```

This is the template for:
- GitHub event channel (push, PR, issue notifications)
- Email channel (incoming email alerts)
- Jarvina event channel (call events, transfer events)
- System monitor channel (disk, memory, process alerts)

---

## Key Vocabulary

| Term | Meaning |
|------|---------|
| **MCP** | Model Context Protocol — standard for tools and resources that AI models use |
| **stdio transport** | Communication via stdin/stdout pipes between parent and child process |
| **Channel notification** | `notifications/claude/channel` — special MCP method that injects content into Claude's conversation |
| **grammy** | TypeScript Telegram bot library used by the plugin |
| **Bun** | JavaScript runtime (like Node.js but faster) that runs the plugin |
| **Skill** | Markdown prompt file that guides Claude through configuration tasks |

---

*Sparked Matter LLC — the smartest spark in the room*

*"Understand how they built the bridge, then build a better one."*
