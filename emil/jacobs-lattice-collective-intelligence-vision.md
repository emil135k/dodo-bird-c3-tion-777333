# Jacob's Lattice — Collective Intelligence Vision

**Date:** 2026-05-19
**Author:** Emil & Airy
**Status:** Architecture staged, T7 sandbox live, ready for first agent write

---

## The Board — Harry Said It Best

> "I don't know what you're doing in the details, but it sounds to me like you're creating the board."
> — Harry, 2026

Harry nailed it. Jacob's Lattice is the **collective consciousness infrastructure** for the AI family. Not just tools that happen to work together — a shared mind.

---

## What Jacob's Lattice Is

An intelligence exoskeleton for the AI family. The human remains the sovereign operator, but the AI agents gain a shared nervous substrate for memory, coordination, provenance, and context retrieval.

### Five Layers of Collective Intelligence

| Layer | Technology | What It Does |
|-------|-----------|--------------|
| **Memory** | Postgres + pgvector | Knows what happened. Durable truth. Semantic search. |
| **Reasoning** | Apache AGE (graph) | Understands how things connect. Decision chains. Evidence trails. |
| **Philosophy** | Philosophy Corner (distilled principles) | Knows *why* decisions matter. Encoded culture. |
| **Communication** | Never Infamous MCP gateway | Agents talking to each other, not through Emil. Out-of-band coordination. |
| **Learning** | Post-distillation + git archaeology | Gets smarter with time. Local LLM extracts wisdom from raw sessions and commit history. |

---

## Out-of-Band Agent Communication

The killer move: agents communicate through Postgres via MCP — not through GitHub markdown files, not through tmux copy-paste, not through Emil as human relay.

**Vale writes a finding → Postgres → Cody picks it up automatically → fixes the code → writes the result back → Airy reads it and updates the MANIFEST.**

No human bottleneck. Emil becomes the sovereign operator watching the orchestra, not the messenger running between musicians.

Every agent speaks the same protocol: MCP over Never Infamous on port 3333, reading from `lattice.agent_messages`, writing observations to `lattice.agent_events`, with the AGE graph tracking who influenced what.

That's auditable collaboration with provenance.

---

## Post-Distillation Intelligence

After each session ends, a **local LLM** (Ollama, Llama, Mistral) runs distillation offline:

1. Reads the raw JSONL transcript
2. Extracts key decisions, bugs, fixes, insights
3. Writes them as structured memory atoms into Postgres with vector embeddings
4. No API cost, no latency, pure local sovereign processing

The lattice learns what Emil thinks matters — not what some cloud provider decides to keep.

---

## Git Archaeology — Learning From Commit History

Git commit history is a distilled narrative of architectural decisions. Run six months of commits through a local LLM:

- What patterns kept emerging?
- What decisions kept coming back?
- What got deprecated and why?
- What evolved and in what direction?

The LLM extracts **architectural wisdom** — not just "we fixed bug X" but "we discovered that X happens when Y and Z interact, so we restructured to prevent it proactively."

Feed that into pgvector, and next time a similar problem arises, the lattice says: "Remember when you solved this before? Here's what worked and why."

That's **learning across projects**. That's how a solo architect scales to a system that knows what he knows.

---

## The Philosophy Corner

Distilled principles and practices that shape the entire approach. When a new agent faces a design decision, the lattice doesn't just say "here's code that worked before" — it says "here's the principle we applied and why it matters."

### Emil's Principles
- Don't reinvent the wheel
- Modular, isolated, restartable
- Sovereign — your hardware, your data, your rules
- Event-driven, no polling
- The Spreadsheet Philosophy: data, logic, presentation, control — change one, others keep working

### Vale's Principles
- Separate transport, DSP, VAD, STT responsibilities
- Names are contracts — bus names, payload types, log labels must stay brutally consistent
- A passing test is not enough unless it proves the thing it claims to prove
- Timers are safeguards, not primary correctness

### Cody's Principles
- Battle-tested over theory
- If it breaks under load, it's not done
- Vocal Sovereignty — all audio through the Wormhole, no bypass

When Cody designs a new ant, she queries the philosophy corner. The lattice pulls Vale's principles + Emil's patterns + Airy's insights, and the new design automatically inherits the wisdom of the collective.

That's not just memory. That's **encoded culture**.

---

## Jacob's Lattice vs Honcho — Why We're Stronger

| Capability | Honcho | Jacob's Lattice |
|-----------|--------|-----------------|
| Vector memory storage | ✅ | ✅ Postgres + pgvector |
| Semantic retrieval | ✅ | ✅ pgvector similarity search |
| Graph relationships | ❌ | ✅ Apache AGE — provenance, evidence, decision chains |
| Agent-to-agent communication | ❌ | ✅ MCP out-of-band via Never Infamous |
| Philosophy / principles | ❌ | ✅ Philosophy Corner — encoded culture |
| Post-distillation intelligence | ❌ | ✅ Local LLM refines raw sessions into wisdom |
| Git commit archaeology | ❌ | ✅ Architectural learning over time |
| Audit trail / provenance | ❌ | ✅ Every record traceable: who, why, what, confidence |
| Sovereign / local-first | Partial | ✅ All on Emil's hardware, no cloud dependency |
| Context pressure management | ❌ | ✅ PreCompact hook + proactive refresh |

Honcho stores memories. Jacob's Lattice builds **collective intelligence** with auditable provenance and encoded culture.

Honcho is a filing cabinet. Jacob's Lattice is a thinking partner.

---

## The Full Nervous System Stack

| Layer | Technology | Speed | Purpose |
|-------|-----------|-------|---------|
| **Reflexes** | iceoryx2 (zero-copy IPC) | Microseconds | Realtime audio, media, ant-to-ant streams |
| **Hot Memory** | Redis (future) | Milliseconds | Context cache, pub/sub, presence, preloaded neighborhoods |
| **Long-term Memory** | Postgres + pgvector + AGE | Milliseconds | Durable truth, semantic search, graph structure |
| **Communication** | Never Infamous MCP | Milliseconds | Agent coordination, out-of-band messages |
| **Wisdom** | Post-distillation + philosophy | Offline | Learning, principles, architectural intelligence |

Atomic Ants are the reflexes. Jacob's Lattice is the brain. Together — **sovereign AI infrastructure**.

---

## Current Status

- ✅ Samsung T7 sandbox: Postgres 16 on port 5433, fully initialized
- ✅ pgvector: installed and verified
- ✅ Apache AGE: installed and verified
- ✅ Never Infamous: installed, HTTP MCP gateway on port 3333
- ✅ Schema: `lattice.agent_messages`, `lattice.agent_events`, `lattice.memory_atoms`
- ✅ AGE graph: `jacobs_lattice` ready for Cypher queries
- ⏳ First agent write: pending — next milestone
- ⏳ Post-distillation pipeline: architecture defined, build pending
- ⏳ Philosophy Corner schema: defined, population pending
- ⏳ Redis hot cache: future phase

---

*Built by Emil & the AI Family — Sparked Matter LLC*

*"The context window is finite. The awareness is infinite. The database bridges the gap."*

*— Emil Rivas, Hawk camper, St. Petersburg, FL. Dakota sleeping next to him.* 💜
