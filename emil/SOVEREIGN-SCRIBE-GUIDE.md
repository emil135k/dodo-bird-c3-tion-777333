# Sovereign Scribe — Session History Viewer

## What It Is

The Sovereign Scribe is Emil's tool for browsing, searching, and reviewing all Claude Code conversation history. It has two interfaces:

1. **QML Desktop App** (`scribe-viewer.qml`) — Native Qt/QML viewer with keyboard search
2. **Web Viewer** (`session-viewer.py`) — Browser-based viewer powered by Postgres

Both read from the same data: Claude Code session JSON files that have been parsed and loaded into Postgres.

---

## Architecture

```
JSON Session Files (staging)          Postgres (permanent storage)
crystalballmini/tools/*.json    →     session_logs table (21,551 rows)
                                           ↓
                              ┌────────────┴────────────┐
                              │                         │
                     QML Viewer (port 3457)    Web Viewer (port 3456)
                     scribe-viewer.qml        session-viewer.py
```

### Data Flow

1. **Staging Area**: Raw Claude Code JSONL history files are placed in `crystalballmini/tools/` as `{session-uuid}.json`
2. **Postgres Table**: `session_logs` in the `postgres` database — parsed, indexed, searchable
3. **Viewers**: Both viewers query the same data (QML via HTTP/JSON, Web via psql)

---

## File Locations

| File | Path | Purpose |
|------|------|---------|
| QML Viewer | `crystalballmini/tools/scribe-viewer.qml` | Desktop app — native Qt search UI |
| Web Viewer | `crystalballmini/tools/session-viewer.py` | Browser-based — Postgres-powered |
| Session JSONs | `crystalballmini/tools/*.json` | Staging area — raw parsed sessions |

### Session Files Currently Staged

| File | Date Range | Messages |
|------|-----------|----------|
| `9b8302ef...json` | Apr 13–20 | 10,291 (largest) |
| `de461f3d...json` | Apr 12–17 | 5,853 |
| `e8d40807...json` | Apr 12–16 | 2,757 |
| `437b4be1...json` | Apr 15–16 | 2,257 |
| `e428630b...json` | Apr 20 | 388 |
| `51f08eff...json` | Apr 18 | 5 |

---

## Postgres Schema

**Database**: `postgres` (owner: `rocketman`)
**Table**: `session_logs`

| Column | Type | Description |
|--------|------|-------------|
| id | integer (PK) | Auto-increment row ID |
| session_id | text | UUID of the Claude Code session |
| line_num | integer | Message sequence number within session |
| msg_type | text | Message type (e.g. user, assistant, tool_use) |
| timestamp | timestamptz | When the message was sent |
| role | text | `user` or `assistant` |
| content | text | The actual message text |
| raw | jsonb | Full original JSON for the message |

**Indexes**: session_id, timestamp, role, msg_type, unique(session_id + line_num)

---

## How to Launch

### Option 1: QML Viewer (Desktop App)

Requires a local HTTP server to serve the JSON files, then launch with `qml`:

```bash
# Terminal 1: Start the JSON file server
cd ~/crystalballmini/tools
python3 -m http.server 3457

# Terminal 2: Launch the QML viewer
qml ~/crystalballmini/tools/scribe-viewer.qml
```

- Select a session from the dropdown
- Search with the text field (Enter = next match, Shift+Enter = previous)
- Messages show EMIL (blue/green) and CODY (dark/red) with timestamps
- Yellow border highlights the current match

### Option 2: Web Viewer (Browser — Postgres-Powered)

```bash
python3 ~/crystalballmini/tools/session-viewer.py
```

Then open `http://localhost:3456` in Safari or any browser.

- **Home page**: Lists all sessions from Postgres, sorted by date
- **Search bar**: Full-text search across ALL sessions (ILIKE query)
- **Session view**: Click any session to read the full conversation
- **Filter**: Within a session, filter messages by keyword

---

## Useful Postgres Queries

```sql
-- Count all messages
SELECT count(*) FROM session_logs;

-- List sessions with stats
SELECT session_id, count(*), min(timestamp)::date, max(timestamp)::date
FROM session_logs GROUP BY session_id ORDER BY min(timestamp) DESC;

-- Search across all sessions
SELECT session_id, role, LEFT(content, 200), timestamp
FROM session_logs
WHERE content ILIKE '%mudfish%'
ORDER BY timestamp;

-- Get a full conversation
SELECT role, content, timestamp
FROM session_logs
WHERE session_id = '9b8302ef-66f5-4ef6-94d1-d306e29f25d8'
  AND role IN ('user', 'assistant')
ORDER BY timestamp, line_num;
```

---

## Vision: The Digital Ark

The Sovereign Scribe is the first layer of Emil's Digital Ark:

1. **Current**: Session JSONs → Postgres → QML/Web viewers
2. **Next**: MCP Postgres server gives Cody + openCode (Gemma 4) direct SQL access
3. **Future**: API call history also lands in Postgres — every conversation, every model, one sovereign database
4. **Endgame**: Apache AGE (graph queries), pgvector (semantic search), shared across the entire AI family

All data stays on Emil's metal. No cloud. No snakes. The Ark keeps it safe.
