# T7 Postgres MCP Sandbox

> Canonical document: `/Users/rocketman/crystalballmini/emil/t7-postgres-mcp-sandbox.md` on branch `hypaiassist/iceoryx2`.
>
> This `emil/` copy is the master architecture note. Derived copies in sandbox worktrees should be synchronized from this file, not edited independently.

## Purpose

Build Jacob's Lattice database work on a clean external-drive sandbox before touching any local or production Postgres service.

This sandbox is the first concrete step toward an MCP-accessible memory and communication organ for the AI family. It should let Claude Code, Codex/Vale, Gemini CLI, and future agents collaborate through a shared database instead of relaying context through tmux copy/paste or human-mediated console ping-pong.

This is intentionally additive:

- Local Homebrew Postgres remains untouched.
- The sandbox runs on port `5433`, not `5432`.
- The data directory lives on the external APFS volume.
- Never Infamous is installed into the sandbox tree, not globally.

## Architecture Vision

Jacob's Lattice is an intelligence exoskeleton for the AI family. The human remains the sovereign operator, but the AI tools gain a shared nervous substrate for memory, coordination, provenance, and context retrieval.

The intended mapping:

- **Postgres**: durable source of truth for events, memories, decisions, artifacts, and agent communication.
- **pgvector**: semantic neighborhoods and similarity search over memories, project history, code decisions, notes, and distilled session context.
- **Apache AGE**: graph structure for relationships between agents, projects, files, concepts, decisions, tasks, failures, fixes, and evidence.
- **Never Infamous Postgres MCP**: shared MCP gateway so multiple CLI agents can talk to the same database through a common tool surface.
- **Redis**: future hot working-memory layer for fast context spheres, pub/sub, presence, and preloaded local neighborhoods from Postgres.
- **Elixir/BEAM**: future orchestration brain/medulla for process supervision, dynamic reset scopes, state transitions, and higher-level coordination.
- **iceoryx2**: realtime data backplane for Atomic Ant media/control streams, separate from durable memory.
- **Atomic Ants**: lightweight task daemons that should stay fast, specialized, and restartable.

This sandbox should not become a monolith. It is the durable lattice under the agents, not a replacement for the ant stack.

## Objectives

Short term:

- Prove a clean external-drive Postgres cluster on Samsung T7.
- Keep the local Postgres install available as a fallback.
- Verify `pgvector` and Apache AGE inside the sandbox.
- Install Never Infamous locally inside the sandbox tree.
- Create a first schema for AI-family communication and durable collaboration history.
- Keep all changes reviewable on a dedicated Git branch.

Medium term:

- Connect multiple CLI tools to the same MCP gateway.
- Capture agent messages, session summaries, decisions, file changes, and recovery notes.
- Store semantic memory atoms with embeddings.
- Represent cross-links in AGE: agent to task, task to files, decision to evidence, bug to fix, project to concept.
- Start with raw SQL/Cypher for graph operations before designing custom MCP tools.

Long term:

- Add custom AGE-facing MCP tools or a thin lattice-specific gateway.
- Add Redis as a hot context cache and pub/sub layer.
- Add agent-specific context hydration, where each CLI receives relevant memories and skills for the current work sphere.
- Support out-of-band AI-family coordination without relying on tmux or terminal copy/paste.

## Associations

The early lattice should keep these relationships explicit:

- **Agent -> Message**: who said what, to whom, and on which channel.
- **Agent -> Event**: tool use, commit, failure, recovery, verification, observation.
- **Memory -> Project**: which project or domain the memory belongs to.
- **Memory -> Embedding**: semantic vector used for retrieval.
- **Memory -> Graph Node**: optional AGE vertex for graph traversal.
- **Decision -> Evidence**: why a decision was made, what was measured, and what was ruled out.
- **Artifact -> File/Commit**: docs, scripts, branches, commits, recordings, and configuration changes.
- **Task -> State**: proposed, active, blocked, verified, abandoned, superseded.

This keeps the database from turning into a loose pile of memories. Every record should have enough provenance to answer: who wrote it, why it exists, what it affects, and how confident we are.

## Current Finding

The external physical drive is mounted as `/Volumes/SamsungT7`.

`/Volumes/T7` previously appeared as an empty stale mount directory. The real APFS volume was renamed from `/Volumes/I ` to `/Volumes/SamsungT7` to avoid path spaces and script ambiguity. You can override with:

```bash
export EXTERNAL_VOLUME="/Volumes/SamsungT7"
```

## Sandbox Layout

Default root:

```text
/Volumes/SamsungT7/jacobs-lattice-sandbox/
  postgres16/          # PostgreSQL 16 data directory
  logs/                # Postgres and MCP logs
  mcp-neverinfamous/   # local npm install of @neverinfamous/postgres-mcp
  run/                 # socket/runtime directory
```

## Ports

- Local/current Postgres: `5432`
- T7 sandbox Postgres: `5433`

## First-Stage Strategy

1. Initialize a fresh Postgres 16 cluster on the external drive.
2. Start it on `127.0.0.1:5433`.
3. Create `lattice_sandbox`.
4. Enable `pgvector` and Apache AGE.
5. Create first communication/history tables under schema `lattice`.
6. Use raw SQL for AGE/Cypher first.
7. Put Never Infamous in front as the MCP gateway.

Current first schema:

- `lattice.agent_messages`: out-of-band agent-to-agent messages.
- `lattice.agent_events`: structured history of actions, observations, failures, recoveries, and verifications.
- `lattice.memory_atoms`: durable memory records with optional `vector(1536)` embeddings.
- AGE graph `jacobs_lattice`: graph-side structure for relationships and traversals.

Apache AGE can be used through SQL-wrapped Cypher before we build direct MCP tools:

```sql
LOAD 'age';
SET search_path = ag_catalog, "$user", public;

SELECT *
FROM cypher('jacobs_lattice', $$
  MATCH (n)
  RETURN n
$$) AS (n agtype);
```

## Why Fresh Instead Of Relocate

The local cluster was only pre-staged and is already in a stale service state. Since there is no meaningful production data to preserve, a fresh external sandbox is cleaner than migrating inherited drift.

## Scripts

```bash
hypAiAssist/scripts/postgres-sandbox/init-t7-postgres.sh
hypAiAssist/scripts/postgres-sandbox/start-t7-postgres.sh
hypAiAssist/scripts/postgres-sandbox/stop-t7-postgres.sh
hypAiAssist/scripts/postgres-sandbox/bootstrap-lattice-db.sh
hypAiAssist/scripts/postgres-sandbox/install-neverinfamous.sh
hypAiAssist/scripts/postgres-sandbox/run-neverinfamous-http.sh
hypAiAssist/scripts/postgres-sandbox/install-neverinfamous-launchagent.sh
hypAiAssist/scripts/postgres-sandbox/uninstall-neverinfamous-launchagent.sh
hypAiAssist/scripts/postgres-sandbox/verify-t7-postgres.sh
```

## Never Infamous HTTP Gateway

Install into the sandbox:

```bash
hypAiAssist/scripts/postgres-sandbox/install-neverinfamous.sh
```

Run local HTTP MCP gateway:

```bash
hypAiAssist/scripts/postgres-sandbox/run-neverinfamous-http.sh
```

Install as a user LaunchAgent so the gateway stays up outside a one-shot shell:

```bash
hypAiAssist/scripts/postgres-sandbox/install-neverinfamous-launchagent.sh
```

Remove the LaunchAgent:

```bash
hypAiAssist/scripts/postgres-sandbox/uninstall-neverinfamous-launchagent.sh
```

Defaults:

- MCP HTTP host: `127.0.0.1`
- MCP HTTP port: `3333`
- Postgres target: `127.0.0.1:5433/lattice_sandbox`
- Audit log: `/Volumes/SamsungT7/jacobs-lattice-sandbox/logs/neverinfamous-audit.jsonl`

For shared local access, keep it on loopback first. Add `MCP_AUTH_TOKEN` before exposing it beyond localhost.

## Communication Bridge Pattern

At first, agents should use the database as a structured message and history bridge:

1. Write messages into `lattice.agent_messages`.
2. Write durable observations into `lattice.agent_events`.
3. Promote important distilled context into `lattice.memory_atoms`.
4. Add graph links in AGE only when relationships matter.
5. Query recent messages/events for coordination.
6. Query semantic/graph memory for project context.

This avoids pretending the first version is a full distributed brain. The first version is a reliable shared notebook plus message bus. The lattice can become more autonomous after the schema proves itself.

## AGE Strategy

Never Infamous does not currently provide first-class Apache AGE tools. That is acceptable for the sandbox.

Phase 1 uses raw SQL:

```sql
LOAD 'age';
SET search_path = ag_catalog, "$user", public;

SELECT *
FROM cypher('jacobs_lattice', $$
  CREATE (:Agent {name: 'Vale', family: 'Codex'})
$$) AS (v agtype);
```

Later, a thin custom interface can expose safer tools:

- `lattice_age_create_vertex`
- `lattice_age_create_edge`
- `lattice_age_match`
- `lattice_age_link_memory`
- `lattice_age_context_sphere`

That custom layer should encode our domain rules instead of exposing arbitrary graph writes as the default agent behavior.

## Safety Rules

- Do not point production MCP clients at this sandbox until the schema and auth model are reviewed.
- Keep the HTTP gateway on `127.0.0.1` unless there is a deliberate auth boundary.
- Use `MCP_AUTH_TOKEN` before exposing the gateway beyond localhost.
- Treat `psql` as a diagnostic/admin path, not the normal AI-family coordination path.
- Prefer append-only events for history. Mutations should be explicit and audited.
- Do not let agents write arbitrary untyped "memory soup" without category, source, project, and metadata.
- Keep local Postgres on `5432` separate from the T7 sandbox on `5433`.

## Branch

Current sandbox branch:

```text
feature/t7-postgres-mcp-sandbox
```

This keeps infrastructure experiments separate from the main `hypaiassist/iceoryx2` working tree and avoids mixing with unrelated Plaza, audio, or cartridge changes.

## Assessment

This is the right foundation for the early Jacob's Lattice bridge:

- Postgres is the durable source of truth.
- pgvector handles semantic neighborhoods.
- Apache AGE handles graph structure.
- Redis can be added later as hot working memory.
- Never Infamous is the shared MCP gateway candidate.
- Custom AGE tools can be layered later after raw SQL proves the shape.

Be real assessment:

- This is feasible on Apple Silicon with 16GB RAM because early usage is small and mostly metadata/history.
- External-drive performance is acceptable for sandboxing and prototyping.
- Never Infamous is heavier than the minimal `mcp-postgres` server, but it is a better fit for a shared gateway because it offers HTTP/SSE transport, audit logging, tool filtering, pgvector tools, and a broad Postgres tool surface.
- Apache AGE support will require raw SQL at first and a custom interface later.
- Redis should collaborate with Postgres later, not replace it.

The main risk is not performance. The main risk is schema entropy. The early schema should stay small, opinionated, and provenance-first.
