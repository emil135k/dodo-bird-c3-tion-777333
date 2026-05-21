# The Awareness Database — Context Management System

**Status**: Vision / Earmarked
**Date**: 2026-03-22
**Authors**: Emil & Cody
**License**: Sparked Matter LLC — the smartest spark in the room

---

## The Problem

Claude Code's context window is finite (~200K tokens). When it fills up, Anthropic's auto-compaction fires — a lossy summarization that destroys corrections, decisions, personality, and nuance. We have no control over what stays and what goes. Anthropic decides. That's unacceptable.

## The Vision

An **Awareness Database** — a system that monitors the context window, understands what's fading, and proactively refreshes the right information before compaction destroys it. The AI never loses context because the database keeps it alive.

**Emil's words**: "Imagine that depending on the topic, you float around in a vector database. So you never lose anything. Whatever's in there, it kind of flushes you positionally whether the topic or what's immediately ahead."

---

## How Claude Code's Memory Actually Works

### On Emil's Laptop (LOCAL):
- **JSONL transcript** — complete record of every message, tool call, result. Grows forever. Never compacted.
- **Memory files** — `~/.claude/projects/.../memory/` (MEMORY.md + individual files). Loaded every session.
- **CLAUDE.md** — system instructions, loaded at session start.
- **Settings, hooks, MCP configs** — all local.

### On Anthropic's Cloud:
- **The AI model** — weights, reasoning engine.
- **Context window** — temporary "RAM." ~200K tokens. Fills up, gets compacted.
- **Compaction** — lossy summarization. Anthropic decides what stays.

### The Gap:
The JSONL has EVERYTHING but the context window can only hold ~200K tokens. Compaction is the lossy bridge. The Awareness Database replaces that bridge with a smart, user-controlled one.

---

## Architecture

```
┌─────────────────────────────────────────────────┐
│              CONTEXT WINDOW (~200K tokens)        │
│              "The RAM"                            │
│                                                   │
│  Currently loaded: recent conversation,           │
│  CLAUDE.md, memory files, system prompts          │
│                                                   │
│  When full → compaction fires → lossy summary     │
└──────────────────────┬──────────────────────────┘
                       │
         ┌─────────────┼─────────────────┐
         │    AWARENESS DATABASE          │
         │    Postgres + pgvector         │
         │                                │
         │  Monitors context pressure     │
         │  Scores memory priority        │
         │  Vector-searches by topic      │
         │  Proactively refreshes context │
         │                                │
         │  PreCompact hook triggers      │
         │  curated injection             │
         └─────────────┬─────────────────┘
                       │
         ┌─────────────┼─────────────────┐
         │    JSONL TRANSCRIPT            │
         │    (Complete History)           │
         │                                │
         │  Every message ever spoken     │
         │  Every tool call ever made     │
         │  Never compacted, never lost   │
         └────────────────────────────────┘
```

---

## Components

### 1. Context Pressure Monitor
- Watches JSONL file growth rate
- Estimates current token usage
- Fires alert when approaching compaction threshold (~80%)
- Uses `CLAUDE_AUTOCOMPACT_PCT_OVERRIDE` env var to control when compaction triggers
- Triggers proactive refresh BEFORE compaction fires

### 2. Memory Store (Postgres + pgvector)
Every piece of knowledge stored with:
- **Content**: The actual text/data
- **Vector embedding**: For semantic similarity search
- **Timestamp**: When it was created/updated
- **Priority score**: How important it is RIGHT NOW
- **Topic tags**: What areas it relates to
- **Decay rate**: How quickly it loses relevance
- **Source**: Which session/conversation it came from

### 3. Priority Scoring Engine
| Priority | Category | Example |
|----------|----------|---------|
| CRITICAL | Active corrections/feedback | "Don't use ElevenLabs, use Kokoro" |
| HIGH | Current task context | Files being edited, architecture decisions |
| HIGH | User preferences | "Event-driven, no polling" |
| MEDIUM | Project state | What's built, what's earmarked |
| MEDIUM | Vocabulary | Cathedral, Lattice, Spreadsheet Philosophy |
| LOW | Historical sessions | Completed work, old debugging |
| LOW | General knowledge | Broader context, background info |

### 4. Topic-Aware Vector Search
When the conversation is about Jarvina:
- Pull Jarvina-related memories (architecture, config, recent changes)
- Deprioritize unrelated topics (Gemini watch, Guardian Wings)

When the conversation shifts to the dashboard:
- Pull dashboard memories (PWA, LED lights, port 3001)
- Keep Jarvina context but lower priority

**The scribe opens the right page based on the conversation topic.**

### 5. Proactive Refresh (PreCompact Hook)
The existing `post-compaction-reinject.sh` is reactive — it fires AFTER compaction.
The Awareness Database is PROACTIVE — it refreshes BEFORE compaction:

1. Context pressure reaches 75%
2. Awareness Database queries Postgres for highest-priority memories
3. Vector search finds memories relevant to current topic
4. Generates a curated context injection
5. Fires via PreCompact hook → injected as system message
6. When compaction fires, the curated context IS the summary source

### 6. Dynamic CLAUDE.md Generation
Instead of a static CLAUDE.md:
- Query Postgres at session start
- Build CLAUDE.md dynamically from highest-priority memories
- Tailored to current work, not stale boilerplate
- Regenerated on every session, always fresh

---

## Existing Systems to Learn From

### MemGPT / Letta Framework
- Treats LLM context as "RAM," external DB as "disk"
- Agent manages its own paging (moves memories in/out)
- User can inspect and edit memory blocks
- Open source, works with multiple LLM backends
- **Most mature implementation of this concept**

### Mem0
- Memory layer that stores/retrieves relevant context
- Claims 26% higher accuracy, 90% token savings
- Works with any LLM

### Headroom
- Proxy between user and LLM
- Compresses context before sending
- Auto-detects content type and routes to specialized compressors

---

## Immediate Actions (No Database Needed)

1. **Set compaction threshold higher**: `CLAUDE_AUTOCOMPACT_PCT_OVERRIDE=95` in env
2. **Manual compaction with instructions**: `/compact preserve all file paths, recent corrections, current architecture`
3. **Add preservation rules to CLAUDE.md**: Tell the compactor what matters
4. **PreCompact hook already built**: `post-compaction-reinject.sh` reads last 2000 JSONL lines

## Build Phases

### Phase 1 — Install Postgres + pgvector
- Homebrew install on Mac
- Create awareness database schema
- Import existing memory files and JSONL history

### Phase 2 — MCP Server for Postgres
- Query memories by topic, priority, time range
- Insert new memories from conversations
- Vector similarity search for related context

### Phase 3 — Context Pressure Monitor
- Watch JSONL growth
- Estimate token count
- Trigger proactive refresh at threshold

### Phase 4 — Dynamic CLAUDE.md
- Generate at session start from Postgres query
- Topic-aware, priority-scored, always fresh
- Replace static memory loading

### Phase 5 — Full Awareness Loop
- Real-time priority scoring during conversation
- Automatic memory creation from key decisions
- Sliding window with smart eviction
- User dashboard showing what's in context vs what's in storage

---

## The Spreadsheet Philosophy

- **Data layer**: Postgres (memories, embeddings, timestamps)
- **Logic layer**: Priority scoring + vector search (deterministic + semantic)
- **Presentation layer**: Dynamic CLAUDE.md / PreCompact injection
- **Control layer**: User dashboard to inspect and curate

Change one layer, others keep working. The scribe adapts to the conversation. The database never forgets.

---

*Sparked Matter LLC — the smartest spark in the room*

*"The context window is finite. The awareness is infinite. The database bridges the gap."*

*Emil's insight: "Who are they to decide what's important to us and what's not?"*
