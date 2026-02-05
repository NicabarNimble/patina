# Patina Data Flow Cheatsheet

> Quick reference for tracing data through the Patina system.
> Updated: 2026-02-05 | Session: 20260205-064001

## Big Picture

- **Source of truth:** filesystem + `.git/` + `layer/` + `~/.patina/personas/.../events/*.jsonl`
- **Derived stores:** `.patina/local/data/patina.db` + `.patina/local/data/embeddings/.../*.usearch` + `~/.patina/cache/.../persona.db`
- **Readers:**
  - `scry` = DB + vectors only
  - `assay` = DB only
  - `context` = **layer files direct-read** + (beliefs via SQL or BeliefOracle)
  - **Mother** = federation over *outputs/artifacts*, not indexing
- **Eventlog** = append-only audit trail inside `patina.db` (LiveStore pattern)

---

## Commands → Writes → Used by

### `patina scrape code`
**Writes:** `.patina/local/data/patina.db`

**Eventlog:** `code.*` events (`code.function`, `code.call`, `code.symbol`, `code.member`, `code.import`, …)

**Materialized tables:** `function_facts`, `import_facts`, `call_graph`, `code_fts`

**Read by:** `assay` (all), `scry` (semantic+lexical via DB content)

### `patina scrape git`
**Writes:** `.patina/local/data/patina.db`

**Eventlog:** `git.commit`, `git.tag`

**Materialized tables:** `commits`, `co_changes(file_a,file_b,count)`, `commits_fts`

**Read by:** `scry` lexical (commit text), `scry` temporal (co_changes), `assay derive` (activity signals)

### `patina scrape layer`
**Writes:** `.patina/local/data/patina.db`

**Eventlog:** `pattern.*`, `belief.*` (e.g. `pattern.surface`, `belief.surface`)

**Materialized tables/indices:** `patterns`, `beliefs`, `belief_fts`, `pattern_fts`

**Read by:** `scry` belief (SQL/FTS), `context` beliefs (no-topic aggregate)

### `patina scrape sessions`
**Writes:** `.patina/local/data/patina.db`

**Eventlog:** `session.*` (e.g. `session.goal`, `session.decision`, `session.work`, `session.pattern`, `session.context`)

**Tables:** `sessions(id, title, started_at, ended_at, branch, classification, ...)`, `goals`, `observations`

**Read by:** `scry` semantic (session content embedded in `semantic.usearch`)

### `patina scrape forge`
**Writes:** `.patina/local/data/patina.db`

**Eventlog:** `forge.*` (issues, PRs from GitHub)

**Read by:** `scry` (when `--include-issues`), embedded in semantic.usearch (5B-6B range)

### `patina persona note`
**Writes:** `~/.patina/personas/default/events/*.jsonl` (source stream)

**Read by:** `patina persona materialize`

### `patina persona materialize`
**Writes:** `~/.patina/cache/personas/default/persona.db`

**Tables:** `knowledge(id, content, domains, timestamp)`

**Read by:** `scry` persona oracle

### `patina oxidize`
**Reads:** `patina.db` + `persona.db` (+ eventlog for stable ID mapping)

**Writes:** `.patina/local/data/embeddings/e5-base-v2/projections/semantic.usearch`

**Read by:** `scry` semantic oracle, `BeliefOracle::query()` (topic beliefs)

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
**Reads:** All oracles (semantic, lexical, temporal, persona, belief)

**Writes:** `.patina/local/data/patina.db` (eventlog)

**Eventlog:** `scry.query` (every search), `scry.use` (open/copy), `scry.feedback` (ratings)

---

## Scry: Oracles → Storage

| Oracle | Storage | Needs | Covers |
|--------|---------|-------|--------|
| **Semantic** | `semantic.usearch` | `oxidize` | code, sessions, patterns, beliefs, forge (by ID range) |
| **Lexical** | `patina.db` FTS5 | `scrape *` | `code_fts`, `commits_fts`, `belief_fts` |
| **Temporal** | `patina.db` | `scrape git` | `co_changes(file_a, file_b, count)` |
| **Persona** | `persona.db` | `persona materialize` | `knowledge` table |
| **Belief** | `patina.db` + `semantic.usearch` | `scrape layer` + `oxidize` | `beliefs` table + vector range 4B-5B |

---

## Vector ID Ranges (semantic.usearch)

A single USearch index holds multiple content types by encoding **type in the ID offset**.

| Range | Content |
|-------|---------|
| `0 - ~N` | Code symbols (functions, types, etc.) |
| `1B - 2B` | Session content |
| `2B - 3B` | Pattern content |
| `4B - 5B` | Belief content |
| `5B - 6B` | Forge content (issues/PRs) |

---

## Context: The "Odd One Out"

### Direct filesystem reads (no scrape)
- `layer/core/*.md`
- `layer/surface/*.md`

### Beliefs
- **No topic:** SQL aggregate over `patina.db:beliefs`
- **With topic:** `BeliefOracle::query()` (requires `oxidize`)

### Why direct reads?
1. Patterns are meant to be human-readable briefings
2. No need to embed/index them — just render summaries
3. Keeps patterns as git-tracked source of truth (not derived)

Beliefs are different — they need semantic ranking for topic queries.

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
.git/                        ──► patina.db ◄──────────────► assay (structure)
layer/**/*.md                        │
                                     ▼
                             semantic.usearch ◄───────────► scry (search)
                                     ▲
~/.patina/personas/            ──► persona.db ◄────────────┘
  events/*.jsonl

layer/core/*.md               ────────────────────────────► context (direct)
layer/surface/*.md            ────────────────────────────► context (direct)

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
- `embeddings/.../projections/semantic.usearch` — vectors

### User: `~/.patina/`
- `personas/default/events/*.jsonl` — persona source stream
- `cache/personas/default/persona.db` — persona materialized
- `mother/graph.db` — cross-project routing

### Layer: `layer/` (git-tracked)
- `core/*.md`, `surface/*.md` — context direct reads
- `surface/epistemic/beliefs/*.md` — scraped → DB → embedded
- `sessions/*.md` — scraped → DB → embedded
