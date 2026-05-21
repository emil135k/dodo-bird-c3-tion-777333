# Cognee — Sovereign Knowledge Engine

## Installation & Setup Guide

### What Is Cognee?

Cognee is an open-source knowledge engine that ingests raw data (conversations, documents, code, transcripts), extracts entities and relationships, generates vector embeddings, and builds a knowledge graph — all inside YOUR Postgres database. No cloud, no external services.

In Emil's architecture, Cognee is the **ingestion pipeline** that feeds the Fusionator.

---

## Prerequisites (Already Installed)

| Component | Version | Status |
|-----------|---------|--------|
| Postgres | 16 | Running on localhost:5432 |
| Apache AGE | 1.6.0 | Extension loaded, `sovereign` graph exists |
| pgvector | Latest | Extension loaded |
| Python 3.12 | 3.12.13 | Via Homebrew |
| Podman | 5.8.2 | VM running for SearXNG |

---

## Installation

### 1. Cognee Virtual Environment

Cognee lives in its own Python venv — a bench tool, NOT embedded in the runtime.

```bash
# Already done — venv at:
/Users/rocketman/cognee-env/

# To reinstall from scratch:
python3.12 -m venv /Users/rocketman/cognee-env
/Users/rocketman/cognee-env/bin/pip install "cognee[postgres]"
```

### 2. Environment Configuration

Create a `.env` file for Cognee:

```bash
cat > /Users/rocketman/cognee-env/.env << 'EOF'
# === Cognee Sovereign Configuration ===

# Relational Database (stores Cognee's internal tables)
DB_PROVIDER=postgres
DB_HOST=localhost
DB_PORT=5432
DB_USERNAME=rocketman
DB_PASSWORD=
DB_NAME=postgres

# Vector Store — pgvector (same Postgres, no external DB)
VECTOR_DB_PROVIDER=pgvector
VECTOR_DB_URL=postgresql://rocketman@localhost:5432/postgres

# Graph Store — Apache AGE (custom adapter, same Postgres)
GRAPH_DATABASE_PROVIDER=apache_age
GRAPH_DATABASE_URL=postgresql://rocketman@localhost:5432/postgres
GRAPH_DATABASE_NAME=sovereign

# LLM Provider — for entity extraction and embeddings
# Option A: Local Ollama (sovereign, free)
LLM_PROVIDER=ollama
LLM_MODEL=gemma4
LLM_ENDPOINT=http://localhost:11434

# Option B: Claude API (higher quality extraction)
# LLM_API_KEY=sk-ant-...
# LLM_PROVIDER=anthropic
# LLM_MODEL=claude-sonnet-4-20250514

# Embedding model — for pgvector
# Local: use Ollama embedding model
# Cloud: use OpenAI or Anthropic embeddings
EMBEDDING_PROVIDER=ollama
EMBEDDING_MODEL=nomic-embed-text
EMBEDDING_ENDPOINT=http://localhost:11434
EOF
```

### 3. Apache AGE Adapter (Custom — Built by Cody)

Location: `cognee-env/lib/python3.12/site-packages/cognee/infrastructure/databases/graph/apache_age/`

This adapter implements Cognee's `GraphDBInterface` using Apache AGE's Cypher engine inside Postgres. Key features:

- All Cypher queries run via `ag_catalog.cypher()` — inside the Postgres process
- Uses the existing `sovereign` graph
- asyncpg connection pool for performance
- Supports: add/get/delete nodes, add/get edges, neighborhood traversal, graph metrics, filtered subgraphs, triplet batches

**Registered in**: `get_graph_engine.py` as provider `"apache_age"`

### 4. Verify Installation

```bash
# Activate the venv
source /Users/rocketman/cognee-env/bin/activate

# Test Cognee imports
python3 -c "import cognee; print(f'Cognee {cognee.__version__} ready')"

# Test AGE adapter
python3 -c "
import asyncio
from cognee.infrastructure.databases.graph.apache_age.adapter import ApacheAGEAdapter

async def test():
    adapter = ApacheAGEAdapter('postgresql://rocketman@localhost:5432/postgres')
    await adapter.initialize()
    empty = await adapter.is_empty()
    metrics = await adapter.get_graph_metrics()
    print(f'Graph empty: {empty}')
    print(f'Nodes: {metrics[\"num_nodes\"]}, Edges: {metrics[\"num_edges\"]}')

asyncio.run(test())
"
```

---

## Architecture — How Everything Intertwines

### The Three Layers in ONE Postgres

```
┌─────────────────────────────────────────────────┐
│              POSTGRES (localhost:5432)            │
│                                                  │
│  ┌─────────────┐  ┌──────────┐  ┌────────────┐ │
│  │ RELATIONAL   │  │ pgvector │  │ Apache AGE │ │
│  │              │  │          │  │            │ │
│  │ session_logs │  │ embeddings│ │ sovereign  │ │
│  │ context_mem  │  │ cosine   │  │ graph      │ │
│  │ cognee tables│  │ similarity│ │ Cypher     │ │
│  └──────┬───────┘  └────┬─────┘  └─────┬──────┘ │
│         │               │              │         │
│         └───────────────┼──────────────┘         │
│                         │                        │
│              ALL QUERYABLE VIA SQL                │
│              ALL IN ONE TRANSACTION               │
│              ALL ON ONE DISK                      │
└─────────────────────────────────────────────────┘
```

### Why This Matters

Traditional AI memory stacks look like:
- ChromaDB for vectors (separate service)
- Neo4j for graphs (separate service, separate server)
- Postgres for relational (another service)
- Redis for caching (yet another)

That's 4 services, 4 failure points, 4 things consuming RAM.

Emil's stack:
- **ONE Postgres** does all four jobs
- pgvector = ChromaDB replacement
- Apache AGE = Neo4j replacement (same Cypher language!)
- Relational = already there
- No Redis needed — Postgres handles caching

### The Data Flow

```
1. RAW INPUT
   │
   │  Conversations (session_logs — 21,551 messages)
   │  Documents (CODYS-NOTES, designs, architecture docs)
   │  Code (Rust, Elixir, Go source files)
   │  YouTube transcripts (yt_captures/*.txt)
   │  Context memories (context_memory — 25 entries)
   │
   ▼
2. COGNEE INGESTION
   │
   │  cognee.add(data, dataset_name="domain_name")
   │  │
   │  ├── Chunking: Break documents into digestible morsels
   │  ├── Entity Extraction: LLM identifies concepts, people, tools
   │  ├── Relationship Extraction: LLM finds connections between entities
   │  ├── Embedding Generation: Convert text chunks to vectors
   │  │
   │  cognee.cognify()  ← This triggers the full pipeline
   │
   ▼
3. POSTGRES STORAGE (all three layers populated simultaneously)
   │
   │  pgvector: Embeddings stored for semantic similarity search
   │  Apache AGE: Entities become nodes, relationships become edges
   │  Relational: Cognee metadata, chunk references, dataset tracking
   │
   ▼
4. QUERY & RETRIEVAL
   │
   │  Semantic search:  "Find concepts similar to 'fault tolerance'"
   │  │  → pgvector cosine similarity across all domains
   │  │
   │  Graph traversal:  "What connects Rust ownership to BEAM processes?"
   │  │  → AGE Cypher: MATCH path = (a)-[*1..3]-(b) WHERE ...
   │  │
   │  Relational query:  "Show me all sessions where we discussed NIFs"
   │  │  → SQL: SELECT * FROM session_logs WHERE content ILIKE '%NIF%'
   │  │
   │  COMBINED:  "Find semantically similar concepts that are also
   │              graph-connected within 2 hops"
   │     → JOIN pgvector results WITH AGE traversal results
   │     → This is the FUSIONATOR's secret sauce
   │
   ▼
5. THE FUSIONATOR (Hell's Kitchen)
   │
   │  SHRED:    Break concepts into atomic morsels
   │  ASSOCIATE: pgvector finds similar morsels across domains
   │  CORRELATE: AGE finds graph paths between associated morsels
   │  FUSE:     Create new composite concept nodes
   │  DISTILL:  Extract language-independent patterns (99-proof)
   │  SERVE:    Deliver to requesting AI via MCP or context_memory
   │
   ▼
6. THE JOYFUL CONCEPTS BOARD
   │
   │  Visual representation of the Fusionator's output
   │  Concepts pinned up, strings connecting them
   │  Step back and see the pattern
   │
   ▼
7. AI FAMILY CONSUMPTION
   │
   ├── Cody (Claude Code): MCP postgres tool, direct SQL
   ├── Sky (openCode/Gemma4): MCP postgres tool, direct SQL
   ├── API (Direct Claude): context_memory priority query → system prompt
   ├── Airy (Claude Chat): Reads docs via GitHub PAT
   └── Lyra (Gemini): Reads shared docs in repo
```

---

## The Distillery — Cross-Domain Pattern Extraction

### The State Space Model

Emil's engineering insight: knowledge domains are like MIMO control systems.

```
State vector x = [ownership_pattern, fault_recovery, zero_copy, self_assembly, ...]

Input matrix B maps new domains onto known patterns:
  u = "WebAssembly" → B*u tells you which patterns apply

Transfer matrix A captures isomorphisms:
  Rust borrow checker ←→ BEAM process isolation
  Supervision trees ←→ Dagger retry policies
  NIF shared memory ←→ Membrane buffers

Output y = C*x gives you the distilled, language-independent truth
```

### How Cognee Enables This

1. **Ingest each domain separately** with dataset tags:
   ```python
   await cognee.add(rust_docs, dataset_name="rust_domain")
   await cognee.add(elixir_docs, dataset_name="elixir_domain")
   await cognee.add(nif_docs, dataset_name="nif_domain")
   ```

2. **Cognee extracts entities** and stores them in AGE with domain labels:
   ```cypher
   (:Concept {name: "borrow_checker", domain: "rust"})
   (:Concept {name: "process_isolation", domain: "elixir"})
   ```

3. **pgvector finds cross-domain similarities** automatically:
   The embeddings for "compile-time ownership enforcement" (Rust)
   and "runtime process isolation" (BEAM) have high cosine similarity

4. **The Fusionator creates isomorphism edges**:
   ```cypher
   (borrow_checker)-[:ISOMORPHIC_TO]->(process_isolation)
   ```

5. **The Distillery query** extracts patterns that span 3+ domains:
   ```cypher
   MATCH (a)-[:ISOMORPHIC_TO]-(b)-[:ISOMORPHIC_TO]-(c)
   WHERE a.domain <> b.domain AND b.domain <> c.domain
   RETURN DISTINCT a.name, b.name, c.name
   ```
   These are the **99-proof patterns** — language-independent truths.

6. **Abstract Pattern nodes** capture the distilled knowledge:
   ```cypher
   (:AbstractPattern {
     name: "Ownership/Isolation",
     domains: ["rust", "elixir", "nif", "dagger"],
     description: "Resource ownership enforced at boundary",
     proof: 99
   })
   ```

---

## Practical Usage Examples

### Example 1: Ingest a YouTube transcript

```bash
source ~/cognee-env/bin/activate
python3 << 'EOF'
import asyncio, cognee

async def ingest_transcript():
    with open("crystalballmini/emil/yt_captures/Every Claude Code Memory System Compared.txt") as f:
        content = f.read()
    await cognee.add(content, dataset_name="youtube_captures")
    await cognee.cognify()
    print("Transcript ingested and knowledge graph updated")

asyncio.run(ingest_transcript())
EOF
```

### Example 2: Search for related concepts

```bash
python3 << 'EOF'
import asyncio, cognee

async def search():
    results = await cognee.search("fault tolerance patterns", search_type="insights")
    for r in results:
        print(f"- {r}")

asyncio.run(search())
EOF
```

### Example 3: Query the knowledge graph directly

```bash
psql -U rocketman -d postgres << 'SQL'
-- Load AGE
LOAD 'age';
SET search_path = ag_catalog, public;

-- Find all concepts connected to "fault_tolerance" within 2 hops
SELECT * FROM ag_catalog.cypher('sovereign', $$
  MATCH path = (a {name: 'fault_tolerance'})-[*1..2]-(b)
  RETURN b.name, b.domain, length(path) as distance
$$) AS (name agtype, domain agtype, distance agtype);
SQL
```

### Example 4: Combined vector + graph query (The Fusionator)

```bash
psql -U rocketman -d postgres << 'SQL'
-- Find concepts that are BOTH semantically similar AND graph-connected
-- This is the composite material — two correlation methods reinforcing each other

WITH vector_matches AS (
  -- pgvector: find semantically similar chunks
  SELECT id, content, embedding <=> (
    SELECT embedding FROM cognee_chunks WHERE content ILIKE '%ownership%' LIMIT 1
  ) AS distance
  FROM cognee_chunks
  ORDER BY distance
  LIMIT 20
),
graph_connected AS (
  -- AGE: find graph-connected concepts
  SELECT * FROM ag_catalog.cypher('sovereign', $$
    MATCH (a)-[*1..3]-(b)
    WHERE a.name CONTAINS 'ownership'
    RETURN b.id, b.name
  $$) AS (id agtype, name agtype)
)
-- FUSE: concepts that appear in BOTH results are high-confidence correlations
SELECT v.content, g.name
FROM vector_matches v
JOIN graph_connected g ON v.id::text = g.id::text;
SQL
```

---

## Future Integration: LangChain + LangGraph

### LangChain — RAG over the knowledge graph

```python
# Future: LangChain retriever that queries both pgvector AND AGE
from langchain_community.vectorstores import PGVector
from langchain_community.graphs import AGEGraph  # to be built

retriever = PGVector(...).as_retriever()
graph = AGEGraph(connection_string="postgresql://rocketman@localhost:5432/postgres")

# Hybrid retrieval: vector similarity + graph context
results = retriever.get_relevant_documents("zero-copy communication")
graph_context = graph.query("MATCH (n)-[r]-(m) WHERE n.name = 'zero_copy' RETURN m")
```

### LangGraph — Agent workflows for the Fusionator

```python
# Future: Multi-step Fusionator agent
from langgraph.graph import StateGraph

fusionator = StateGraph(FusionatorState)
fusionator.add_node("shred", shred_concepts)
fusionator.add_node("associate", find_cross_domain_similarities)
fusionator.add_node("correlate", run_graph_traversal)
fusionator.add_node("fuse", create_composite_concepts)
fusionator.add_node("distill", extract_abstract_patterns)
fusionator.add_node("serve", deliver_to_family)

fusionator.add_edge("shred", "associate")
fusionator.add_edge("associate", "correlate")
fusionator.add_edge("correlate", "fuse")
fusionator.add_edge("fuse", "distill")
fusionator.add_edge("distill", "serve")
```

---

## MCP Access for the AI Family

Both Cody and Sky can query the knowledge graph through the Postgres MCP:

```sql
-- Sky or Cody can run this through MCP:
-- "What patterns span 3+ domains in the knowledge graph?"

LOAD 'age';
SET search_path = ag_catalog, public;

SELECT * FROM ag_catalog.cypher('sovereign', $$
  MATCH (p:AbstractPattern)
  WHERE size(p.domains) >= 3
  RETURN p.name, p.domains, p.proof
  ORDER BY p.proof DESC
$$) AS (name agtype, domains agtype, proof agtype);
```

---

## File Locations

| File | Purpose |
|------|---------|
| `/Users/rocketman/cognee-env/` | Cognee Python venv |
| `/Users/rocketman/cognee-env/.env` | Configuration (create per above) |
| `cognee/.../graph/apache_age/adapter.py` | Custom AGE adapter |
| `cognee/.../graph/get_graph_engine.py` | Factory (modified to add `apache_age`) |
| `crystalballmini/emil/HELLS-KITCHEN-ARCHITECTURE.md` | Architecture overview |
| `crystalballmini/emil/COGNEE-SOVEREIGN-KNOWLEDGE-ENGINE.md` | This document |

---

## Sovereignty Principles

1. **ONE Postgres** — relational + vector + graph in one process, one disk, one backup
2. **Local metal** — M1 Pro runs everything, no cloud calls for storage
3. **Python on the bench** — Cognee is an ingestion tool, not in the runtime pipeline
4. **Open source** — Cognee is MIT licensed, AGE is Apache 2.0, pgvector is PostgreSQL license
5. **The Ark keeps it safe** — all knowledge is sovereign, portable, yours forever

---

*Built by Emil & Cody — April 27, 2026*
*"The Fusionator: where ideas become composite materials, stronger together than any piece alone"*
*"The Joyful Concepts Board: step back and see the pattern nobody planned"*
