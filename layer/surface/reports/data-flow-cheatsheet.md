# Patina Data Flow Cheatsheet

> Quick reference for tracing data through the Patina system.
> Updated: 2026-02-08 | Session: 20260208-144855 (post semantic-structural split)

## Big Picture

- **Source of truth:** filesystem + `.git/` + `layer/` + `~/.patina/personas/.../events/*.jsonl`
- **Derived stores:** `.patina/local/data/patina.db` + `.patina/local/data/embeddings/.../*.usearch` + `~/.patina/cache/.../persona.db`
- **Readers:**
  - `scry` = semantic vectors only (knowledge domain: beliefs + patterns + commits)
  - `assay` = DB only (FTS5 keyword search, structural queries, co-change, belief grounding)
  - `context` = **layer files direct-read** + (topic: assay factual + scry semantic + beliefs)
  - **Mother** = federation over *outputs/artifacts*, not indexing
- **Eventlog** = append-only audit trail inside `patina.db` (LiveStore pattern)

---

## Commands → Writes → Used by

### `patina scrape code`
**Writes:** `.patina/local/data/patina.db`

**Eventlog:** `code.*` events (`code.function`, `code.call`, `code.symbol`, `code.member`, `code.import`, …)

**Materialized tables:** `function_facts`, `import_facts`, `call_graph`, `code_fts`

**Read by:** `assay` (all structural + FTS5 queries)

### `patina scrape git`
**Writes:** `.patina/local/data/patina.db`

**Eventlog:** `git.commit`, `git.tag`

**Materialized tables:** `commits`, `co_changes(file_a,file_b,count)`, `commits_fts`

**Read by:** `assay search` (FTS5 commit text), `assay cochange` (co_changes), `assay derive` (activity signals), `scry` (commit messages embedded in knowledge domain)

### `patina scrape layer`
**Writes:** `.patina/local/data/patina.db`

**Eventlog:** `pattern.*`, `belief.*` (e.g. `pattern.surface`, `belief.surface`)

**Materialized tables/indices:** `patterns`, `beliefs`, `belief_fts`, `pattern_fts`

**Read by:** `assay search` (FTS5), `assay belief` (grounding), `scry` (beliefs + patterns embedded in knowledge domain), `context` beliefs (no-topic aggregate)

### `patina scrape sessions`
**Writes:** `.patina/local/data/patina.db`

**Eventlog:** `session.*` (e.g. `session.goal`, `session.decision`, `session.work`, `session.pattern`, `session.context`)

**Tables:** `sessions(id, title, started_at, ended_at, branch, classification, ...)`, `goals`, `observations`

**Read by:** `assay search` (FTS5 session text). Sessions are NOT embedded in the knowledge domain (deferred to Phase 5 pending validation).

### `patina scrape forge`
**Writes:** `.patina/local/data/patina.db`

**Eventlog:** `forge.*` (issues, PRs from GitHub)

**Read by:** `assay search` (FTS5 when `--include-issues`). Forge events are NOT embedded in the knowledge domain.

### `patina persona note`
**Writes:** `~/.patina/personas/default/events/*.jsonl` (source stream)

**Read by:** `patina persona materialize`

### `patina persona materialize`
**Writes:** `~/.patina/cache/personas/default/persona.db`

**Tables:** `knowledge(id, content, domains, timestamp)`

**Read by:** `scry` (legacy persona bolting, deprecated)

### `patina oxidize`
**Reads:** `patina.db` (beliefs, patterns, commits from eventlog)

**Writes:** `.patina/local/data/embeddings/e5-base-v2/projections/knowledge.usearch` (+ knowledge.safetensors projection)

**Read by:** `scry` semantic oracle (QueryEngine), `context` topic queries (via scry)

### `patina assay derive`
**Reads:** `git.commit` events and/or materialized git tables

**Writes:** `.patina/local/data/patina.db`

**Materialized table:** `module_signals(path, is_used, activity_level, centrality_score, ...)`

**Read by:** `assay` queries, `scry orient`

### `patina session start/update/end`
**Writes:** `.patina/local/data/patina.db` (eventlog only, action-time)

**Eventlog:** `session.started`, `session.update`, `session.ended`

**Note:** These write at action-time, not via scrape. Dual-write with markdown files.

### `patina scry`
**Reads:** Semantic oracle only (knowledge.usearch via QueryEngine)

**Writes:** `.patina/local/data/patina.db` (eventlog)

**Eventlog:** `scry.query` (every search), `scry.use` (open/copy), `scry.feedback` (ratings)

---

## Post-Split: Scry vs Assay

| Tool | Storage | Needs | Covers |
|------|---------|-------|--------|
| **scry** (semantic) | `knowledge.usearch` | `oxidize` | beliefs, patterns, commits (knowledge domain) |
| **assay search** (factual) | `patina.db` FTS5 | `scrape *` | `code_fts`, `commits_fts`, `pattern_fts`, `belief_fts` |
| **assay cochange** | `patina.db` | `scrape git` | `co_changes(file_a, file_b, count)` |
| **assay belief** | `patina.db` | `scrape layer` | `beliefs` table (evidence grounding) |
| **assay derive** | `patina.db` | `scrape git` | `module_signals` (activity, centrality) |

---

## Vector ID Ranges (knowledge.usearch)

A single USearch index holds knowledge domain content by encoding **type in the ID offset**.

| Range | Content |
|-------|---------|
| `0 - ~N` | Commit messages |
| `2B - 3B` | Pattern content |
| `4B - 5B` | Belief content |

**Not embedded** (deferred to Phase 5+): code symbols, sessions, forge events.

---

## Context: Consumer-Level Fusion (Phase 3)

### Direct filesystem reads (no scrape)
- `layer/core/*.md`
- `layer/surface/*.md`

### With topic (fusion)
- **Factual matches:** `assay_search()` (FTS5 keyword hits across code, commits, patterns)
- **Semantic matches:** `QueryEngine::query()` (vector similarity in knowledge domain)
- **Beliefs:** FTS5 belief search, ranked by relevance
- Merge: facts first, then meaning for gaps (HashSet dedup by source_id)

### Without topic
- SQL aggregate over `patina.db:beliefs` (all active beliefs)
- No factual/semantic search (would return random results without focus)

### Why direct reads for patterns?
1. Patterns are meant to be human-readable briefings
2. No need to embed/index them — just render summaries
3. Keeps patterns as git-tracked source of truth (not derived)

---

## Mother: Federation, Not Indexing

Mother does **not** participate in scrape/materialize/oxidize.

She operates on **outputs/artifacts** (beliefs, values, rules) across projects and knows where they live.

She's the **federation layer**, not the **indexing layer**.

---

## Eventlog (Append-Only Audit Trail)

**Location:** `.patina/local/data/patina.db` → `eventlog` table

**Pattern:** LiveStore — eventlog is immutable source of truth, other tables are materialized views.

### Schema
```sql
CREATE TABLE eventlog (
    seq INTEGER PRIMARY KEY AUTOINCREMENT,
    event_type TEXT NOT NULL,      -- e.g. 'git.commit', 'session.decision'
    timestamp TEXT NOT NULL,       -- ISO8601 when event occurred
    source_id TEXT NOT NULL,       -- sha, session_id, function_name, etc
    source_file TEXT,              -- original file path (optional)
    data TEXT NOT NULL,            -- event-specific JSON payload
    CHECK(json_valid(data))
);
```

### Writers
| Command | Event Types |
|---------|-------------|
| `scrape code` | `code.*` (function, call, symbol, member, import) |
| `scrape git` | `git.commit`, `git.tag` |
| `scrape sessions` | `session.*` (goal, decision, work, pattern, context) |
| `scrape layer` | `pattern.*`, `belief.*` |
| `scrape forge` | `forge.*` |
| `session start` | `session.started` (action-time) |
| `session update` | `session.update` (action-time) |
| `session end` | `session.ended` (action-time) |
| `scry` | `scry.query` (every search) |
| `scry open/copy` | `scry.use` |
| `scry feedback` | `scry.feedback` |

### Readers
| Command | Uses |
|---------|------|
| `eval --feedback` | Correlates queries with commits via `feedback_*` views |
| `assay derive` | Activity signals from `git.commit` events |
| `oxidize` | Embedding ID mapping/stability |

### Feedback Views (Precision Measurement)
```sql
feedback_session_queries  -- queries per session
feedback_commit_files     -- files committed per session
feedback_query_hits       -- did retrieval match committed files?
feedback_usage            -- explicit open/copy usage
feedback_ratings          -- good/bad ratings
```

**Key insight:** `feedback_query_hits` answers: *"When I searched during session X, did I end up committing any of those files?"* — implicit precision measurement, no user action needed.

---

## Shape (Truth → Derived → Readers)

```
SOURCES (truth)              DERIVED (indexed)             READERS
─────────────────            ─────────────────             ───────
src/**/*
.git/                        ──► patina.db ◄──────────────► assay (facts: FTS5, structure, co-change)
layer/**/*.md                        │
                                     ▼
                             knowledge.usearch ◄──────────► scry (meaning: vector similarity)

layer/core/*.md               ────────────────────────────► context (direct read)
layer/surface/*.md            ────────────────────────────► context (direct read)
                                                           context (topic: assay + scry fusion)

Mother (federation)           ────────────────────────────► operates on outputs/artifacts
```

---

## Tracing Data Flows

### "Where does X come from?"

1. **Find the reader** — which command/oracle uses this data?
2. **Find the table** — `sqlite3 .patina/local/data/patina.db ".schema <table>"`
3. **Find the writer** — grep for table name in `src/commands/scrape/`
4. **Find the source** — what files/git data does the scraper read?

### "Where does X go?"

1. **Find the writer** — which scrape command produces this?
2. **Check eventlog** — `SELECT DISTINCT event_type FROM eventlog WHERE event_type LIKE 'x.%'`
3. **Check materialized tables** — what tables does this event populate?
4. **Find the readers** — grep for table name in `src/commands/scry/`, `src/commands/assay/`

### Useful Queries

```bash
# Event type counts
sqlite3 .patina/local/data/patina.db \
  "SELECT event_type, COUNT(*) FROM eventlog GROUP BY event_type ORDER BY COUNT(*) DESC"

# Recent events
sqlite3 .patina/local/data/patina.db \
  "SELECT event_type, source_id, timestamp FROM eventlog ORDER BY seq DESC LIMIT 20"

# Schema for a table
sqlite3 .patina/local/data/patina.db ".schema beliefs"

# FTS tables
sqlite3 .patina/local/data/patina.db ".tables" | tr ' ' '\n' | grep fts
```

---

## File Locations

### Project: `.patina/local/data/`
- `patina.db` — everything structured (eventlog + materialized tables)
- `embeddings/.../projections/knowledge.usearch` — vectors (knowledge domain)

### User: `~/.patina/`
- `personas/default/events/*.jsonl` — persona source stream
- `cache/personas/default/persona.db` — persona materialized
- `mother/graph.db` — cross-project routing

### Layer: `layer/` (git-tracked)
- `core/*.md`, `surface/*.md` — context direct reads
- `surface/epistemic/beliefs/*.md` — scraped → DB → embedded
- `sessions/*.md` — scraped → DB (not embedded; deferred to Phase 5)
