# Spec: Eventlog Architecture (LiveStore Pattern)

## Overview

Patina adopts LiveStore's event-sourcing architecture for deterministic knowledge accumulation. This spec explains why unified eventlog is foundational for multi-user workflows, ML projections, and temporal queries.

**Core Insight:** Git commits and session files ARE events. We don't create a separate event layer—we materialize existing event sources into a queryable unified log.

## The LiveStore Pattern

### What We Adopted

**LiveStore's elegance:**
```typescript
// 1. Define event with schema
const todoCreated = Events.synced({
  name: 'todoCreated',
  schema: Schema.Struct({ id: String, text: String })
});

// 2. Define materializer (event → state)
defineMaterializer(todoCreated, ({ id, text }) =>
  todos.insert({ id, text, completed: false })
);

// 3. System handles storage, validation, replay
```

**Why it's elegant:**
- **Pure functions:** Event → SQL transformations are deterministic
- **Schema-first:** Type safety at boundaries
- **Two-database separation:** `eventlog` immutable, `state` derived
- **Changeset tracking:** Enables rebasing for sync conflicts
- **Unknown event tolerance:** Forward compatibility

**Their structure:**
```
dbEventlog (immutable)           dbState (derived, rebuildable)
├── eventlog table               ├── todos (user tables)
│   ├── seqNum (ordering)        ├── users
│   ├── name                     └── __system_* (metadata)
│   ├── args (JSON)                  ├── schema_meta
│   ├── clientId/sessionId           └── session_changeset
│   └── schemaHash
└── sync_status
```

### What We Adapted

| LiveStore | Patina | Reason |
|-----------|--------|--------|
| **Event creation** | Events explicitly created | Events read from git/sessions | Different data sources |
| **Per-event materializers** | Real-time SQL transforms | Batch transformations | ML needs corpus context |
| **Schema hashing** | For rebasing conflicts | For recipe versioning | Detect stale builds |
| **Changesets** | Undo/redo during sync | Not needed | We rebuild from scratch |
| **Two databases** | `eventlog` + `state` | `eventlog` + materialized views | ✓ Keep this pattern |

### What We Added (Novel for ML)

**1. Multi-Projection Queries**

LiveStore materializes once (for UI). We need multiple projections from same events:

```sql
-- Semantic: Decision text + temporal context
SELECT
  e1.data->>'content' as text,
  (SELECT group_concat(e2.data->>'message', ' ')
   FROM eventlog e2
   WHERE e2.event_type = 'git.commit'
     AND e2.timestamp BETWEEN
       datetime(e1.timestamp, '-7 days') AND e1.timestamp
  ) as context
FROM eventlog e1
WHERE e1.event_type = 'session.decision';

-- Temporal: Co-occurrence matrix
SELECT
  f1.path as file_a,
  f2.path as file_b,
  COUNT(*) as cooccurrence
FROM eventlog e
CROSS JOIN json_each(e.data, '$.files') f1
CROSS JOIN json_each(e.data, '$.files') f2
WHERE e.event_type = 'git.commit'
GROUP BY file_a, file_b;

-- Social: Author collaboration network
SELECT
  e1.data->>'author' as author_a,
  e2.data->>'author' as author_b,
  COUNT(*) as shared_files
FROM eventlog e1
JOIN eventlog e2 ON
  e1.event_type = 'git.commit' AND
  e2.event_type = 'git.commit' AND
  EXISTS (
    SELECT 1 FROM json_each(e1.data, '$.files') f1
    JOIN json_each(e2.data, '$.files') f2
    WHERE f1.value->>'path' = f2.value->>'path'
  )
GROUP BY author_a, author_b;
```

**2. Temporal Slicing**

ML training needs temporal splits:

```rust
// Train on historical, validate on recent
let train_events = query_eventlog("timestamp < '2025-03-01'");
let val_events = query_eventlog("timestamp >= '2025-03-01'");

let embedding = train_projection(train_events);
let accuracy = validate(embedding, val_events);
```

**3. Causality Analysis**

```sql
-- Which decisions led to which commits?
SELECT
  d.data->>'content' as decision,
  c.data->>'message' as commit,
  (julianday(c.timestamp) - julianday(d.timestamp)) as days_between
FROM eventlog d
JOIN eventlog c ON c.event_type = 'git.commit'
WHERE d.event_type = 'session.decision'
  AND c.timestamp > d.timestamp
  AND c.timestamp < datetime(d.timestamp, '+14 days')
ORDER BY days_between;
```

## Patina's Database Structure

Following LiveStore pattern, adapted for ML:

```sql
-- Unified eventlog (source of truth)
CREATE TABLE eventlog (
    seq INTEGER PRIMARY KEY AUTOINCREMENT,  -- Global ordering
    event_type TEXT NOT NULL,                -- 'git.commit', 'session.decision', etc
    timestamp TEXT NOT NULL,                 -- ISO8601 when event occurred
    source_id TEXT NOT NULL,                 -- sha, session_id, function_name
    source_file TEXT,                        -- Original file path
    data JSON NOT NULL,                      -- Event-specific payload
    CHECK(json_valid(data))
);

-- Indexes for temporal and type-based queries
CREATE INDEX idx_eventlog_type ON eventlog(event_type);
CREATE INDEX idx_eventlog_timestamp ON eventlog(timestamp);
CREATE INDEX idx_eventlog_type_time ON eventlog(event_type, timestamp);
CREATE INDEX idx_eventlog_source ON eventlog(source_id);

-- Materialized views (derived from eventlog)
CREATE VIEW commits AS
  SELECT
    source_id as sha,
    data->>'message' as message,
    data->>'author' as author_name,
    data->>'email' as author_email,
    timestamp
  FROM eventlog
  WHERE event_type = 'git.commit';

CREATE VIEW sessions AS
  SELECT
    source_id as id,
    data->>'title' as title,
    data->>'branch' as branch,
    timestamp as started_at
  FROM eventlog
  WHERE event_type = 'session.started';

CREATE VIEW observations AS
  SELECT
    seq,
    source_id as session_id,
    data->>'content' as content,
    data->>'type' as observation_type,
    timestamp
  FROM eventlog
  WHERE event_type IN ('session.decision', 'session.observation');

-- ML-specific materialized tables (computed from eventlog)
CREATE TABLE co_changes (
  file_a TEXT,
  file_b TEXT,
  count INTEGER,
  last_updated_seq INTEGER,
  PRIMARY KEY (file_a, file_b)
);

CREATE TABLE author_expertise (
  author TEXT,
  file_path TEXT,
  commit_count INTEGER,
  last_commit_timestamp TEXT,
  last_updated_seq INTEGER,
  PRIMARY KEY (author, file_path)
);

-- Meta tables (recipe versioning, not rebasing)
CREATE TABLE oxidize_meta (
  key TEXT PRIMARY KEY,
  value JSON
);

CREATE TABLE scrape_meta (
  source TEXT PRIMARY KEY,  -- 'git', 'sessions', 'code'
  last_seq INTEGER,
  last_updated TEXT
);
```

## Multi-User Alignment

### How Unified Eventlog Supports Multi-User

**The pattern:**
```
USER A's Mac                              USER B's Mac
─────────────────────────────────────     ─────────────────────────────────────

project/.patina/ (SHARED via git)         project/.patina/ (SHARED via git)
└── oxidize.yaml      ← same recipe       └── oxidize.yaml      ← same recipe

.git/ (SHARED)                            .git/ (SHARED)
└── commits           ← event sources     └── commits           ← event sources

layer/sessions/ (SHARED)                  layer/sessions/ (SHARED)
└── *.md              ← event sources     └── *.md              ← event sources

project/.patina/data/ (LOCAL)             project/.patina/data/ (LOCAL)
└── patina.db                             └── patina.db
    ├── eventlog      ← rebuilt           ├── eventlog          ← rebuilt
    └── views         ← derived           └── views             ← derived
```

**Workflow:**
```bash
# User A adds knowledge
/session-note "TypeScript prefers Result types"
git commit -m "docs: capture TS error pattern" && git push

# User B pulls
git pull                # Gets session file
patina scrape           # Rebuilds eventlog from git+sessions
                        # Both users now have IDENTICAL eventlog
                        # (deterministic from same git SHA)

# Query works identically on both machines
patina scry "error handling"
  → [PROJECT] TypeScript prefers Result types
```

**Why deterministic:**
1. Events come from git (same SHA = same commits + sessions)
2. Scrape processes in `ORDER BY timestamp, source_id`
3. Global `seq` assigned during scrape (reproducible)
4. JSON payloads are canonical (no undefined ordering)

**Key property:** Two users on the same git commit will scrape to identical eventlog.

### Mothershi Integration

Mothership queries the unified eventlog:

```rust
// POST /scry endpoint
pub async fn scry(request: ScryRequest) -> Result<ScryResponse> {
    // 1. Query project eventlog
    let project_results = query_eventlog(
        &project.db_path,
        &request.query,
        event_types: vec!["session.decision", "session.observation"],
    )?;

    // Tag as [PROJECT]
    for r in project_results {
        results.push(TaggedResult {
            tag: "[PROJECT]",
            content: r.content,
            similarity: r.score,
        });
    }

    // 2. Query persona (separate DB, not in project)
    let persona_results = query_persona(&request.query)?;

    // Tag as [PERSONA] with penalty
    for r in persona_results {
        results.push(TaggedResult {
            tag: "[PERSONA]",
            content: r.content,
            similarity: r.score * 0.95,  // Slight penalty
        });
    }

    Ok(merge_and_sort(results))
}
```

**Benefits for scry:**
- Single source: Query eventlog for all project knowledge
- Temporal filtering: `WHERE timestamp BETWEEN ...` for time travel
- Cross-cutting: Correlate decisions with commits in one query
- Deterministic: Same eventlog = same query results

## Event Types

### Git Events

```sql
-- git.commit
INSERT INTO eventlog (event_type, timestamp, source_id, source_file, data) VALUES (
  'git.commit',
  '2025-11-21T11:29:32-05:00',
  '1e541ac8',
  NULL,
  json_object(
    'sha', '1e541ac87009536128438063656c79ddde288678',
    'message', 'docs: archive sessions',
    'author', 'NicabarNimble',
    'email', 'nicabar@gmail.com',
    'files', json_array(
      json_object('path', 'layer/sessions/20251121-042111.md', 'lines_added', 290, 'lines_removed', 0)
    )
  )
);
```

### Session Events

```sql
-- session.started
INSERT INTO eventlog (event_type, timestamp, source_id, source_file, data) VALUES (
  'session.started',
  '2025-11-21T11:58:12Z',
  '20251121-065812',
  'layer/sessions/20251121-065812.md',
  json_object(
    'id', '20251121-065812',
    'title', 'review build.md and spec files',
    'branch', 'neuro-symbolic-knowledge-system',
    'goals', json_array('review build.md and spec files')
  )
);

-- session.decision
INSERT INTO eventlog (event_type, timestamp, source_id, source_file, data) VALUES (
  'session.decision',
  '2025-11-21T10:30:00Z',
  '20251121-065812',
  'layer/sessions/20251121-065812.md',
  json_object(
    'session_id', '20251121-065812',
    'content', 'Scrape not emit - Read existing logs instead of instrumenting shell scripts'
  )
);

-- session.observation
INSERT INTO eventlog (event_type, timestamp, source_id, source_file, data) VALUES (
  'session.observation',
  '2025-11-21T10:30:00Z',
  '20251121-065812',
  'layer/sessions/20251121-065812.md',
  json_object(
    'session_id', '20251121-065812',
    'content', 'LiveStore event model good for sync, but we are git-native',
    'type', 'pattern'
  )
);
```

### Code Events ✅ IMPLEMENTED (2025-11-22)

```sql
-- code.function (example from actual implementation)
INSERT INTO eventlog (event_type, timestamp, source_id, source_file, data) VALUES (
  'code.function',
  '2025-11-22T12:27:37.884599+00:00',
  'src/commands/scrape/git/mod.rs::parse_git_log',
  'src/commands/scrape/git/mod.rs',
  json_object(
    'file', 'src/commands/scrape/git/mod.rs',
    'name', 'parse_git_log',
    'is_async', false,
    'is_public', true,
    'is_unsafe', false,
    'takes_mut_self', false,
    'returns_result', true,
    'parameters', json_array('full: bool'),
    'return_type', 'Result<Vec<GitCommit>>'
  )
);

-- code.struct
INSERT INTO eventlog (event_type, timestamp, source_id, source_file, data) VALUES (
  'code.struct',
  '2025-11-22T12:27:37.880000+00:00',
  'src/commands/scrape/code/database.rs::FunctionFact',
  'src/commands/scrape/code/database.rs',
  json_object(
    'file', 'src/commands/scrape/code/database.rs',
    'name', 'FunctionFact',
    'kind', 'struct',
    'visibility', 'public',
    'definition', 'pub struct FunctionFact { ... }'
  )
);

-- code.import
INSERT INTO eventlog (event_type, timestamp, source_id, source_file, data) VALUES (
  'code.import',
  '2025-11-22T12:27:37.870000+00:00',
  'src/main.rs::clap',
  'src/main.rs',
  json_object(
    'file', 'src/main.rs',
    'import_path', 'clap',
    'imported_names', json_array('Parser', 'Subcommand'),
    'import_kind', 'use',
    'line_number', 3
  )
);
```

**Current stats (patina codebase):**
- 13,146 code.* events across 10 types
- code.call (9,634 events) - function calls, 60% of all events
- code.symbol (1,423 events) - all code symbols with FTS
- code.function (790 events) - functions with rich metadata
- code.struct/enum/trait (129 events) - type definitions
- code.import (458 events) - dependency relationships
- code.constant (203 events) - macros, enums, statics
- code.member (477 events) - struct fields, methods

## Time Travel

**Query state at any point in history:**

```sql
-- What did we know on March 15, 2025?
SELECT * FROM eventlog
WHERE timestamp <= '2025-03-15T23:59:59'
ORDER BY seq;

-- What decisions were made in March?
SELECT data->>'content' as decision
FROM eventlog
WHERE event_type = 'session.decision'
  AND timestamp BETWEEN '2025-03-01' AND '2025-03-31'
ORDER BY timestamp;

-- What commits happened around a decision?
WITH decision AS (
  SELECT timestamp
  FROM eventlog
  WHERE source_id = '20251121-065812'
    AND event_type = 'session.decision'
)
SELECT
  e.data->>'message' as commit_message,
  (julianday(e.timestamp) - julianday(d.timestamp)) as days_offset
FROM eventlog e, decision d
WHERE e.event_type = 'git.commit'
  AND e.timestamp BETWEEN
    datetime(d.timestamp, '-7 days') AND
    datetime(d.timestamp, '+7 days')
ORDER BY days_offset;
```

**CLI support:**
```bash
# Scrape only events before date
patina scrape --until 2025-03-15

# Results: patina.db reflects state as of March 15
patina scry "architecture decisions"
  → Only shows decisions made before that date
```

## Materialization Strategies

### Incremental vs Batch

**Incremental (when possible):**
```rust
// Track last processed seq
let last_seq = get_last_seq("co_changes")?;

// Process only new events
let new_commits = query_eventlog(&format!(
    "SELECT * FROM eventlog
     WHERE event_type = 'git.commit'
       AND seq > {}
     ORDER BY seq",
    last_seq
))?;

// Update co_changes incrementally
for commit in new_commits {
    update_co_changes(&commit)?;
}

set_last_seq("co_changes", new_commits.last().seq)?;
```

**Batch (when corpus stats needed):**
```rust
// Some ML features need full corpus
// TF-IDF, LSA, clustering, etc.

// Just rebuild from scratch
fn materialize_term_frequencies() -> Result<()> {
    let all_decisions = query_eventlog(
        "SELECT data->>'content'
         FROM eventlog
         WHERE event_type = 'session.decision'"
    )?;

    // Compute corpus-wide statistics
    let vocab = build_vocabulary(&all_decisions);
    let idf = compute_idf(&all_decisions, &vocab);

    // Store for use in embeddings
    save_idf_weights(&idf)?;
}
```

**Metadata tracking:**
```sql
INSERT INTO scrape_meta (source, last_seq, last_updated) VALUES
  ('co_changes', 1234, '2025-11-21T12:00:00Z'),
  ('term_frequencies', 1234, '2025-11-21T12:05:00Z');

-- Detect staleness
SELECT source FROM scrape_meta
WHERE last_seq < (SELECT MAX(seq) FROM eventlog);
```

## Benefits Summary

### For Multi-User
- ✅ Deterministic: Same git SHA = identical eventlog
- ✅ Git-synced: Events shared via git pull/push
- ✅ Local materialization: Each user builds own views
- ✅ Recipe-driven: oxidize.yaml defines how to build embeddings

### For ML/Embeddings
- ✅ Multi-projection: Same events → multiple embedding spaces
- ✅ Temporal queries: Train on past, validate on recent
- ✅ Correlation analysis: Decision → commit causality
- ✅ Single source: All project knowledge in one table

### For Time Travel
- ✅ Historical queries: `WHERE timestamp < T`
- ✅ Temporal slicing: Train/val/test splits by time
- ✅ Replay: Rebuild state at any point in history
- ✅ Audit trail: Every event timestamped and ordered

### For Scry
- ✅ Unified query: Single eventlog for all project knowledge
- ✅ Cross-cutting: Correlate decisions, commits, code changes
- ✅ Tagged results: [PROJECT] vs [PERSONA]
- ✅ Deterministic: Same events = same results

## Comparison Table

| Property | LiveStore | Patina |
|----------|-----------|--------|
| **Event source** | User actions | Git commits + session files |
| **Event storage** | `eventlog` table | `eventlog` table ✓ |
| **Materialization** | Per-event, real-time | Batch + incremental |
| **Schema validation** | TypeScript schemas | Rust enums + JSON validation |
| **Sync mechanism** | Custom sync backend | Git ✓ |
| **Rebasing** | Changesets + rebasing | Not needed (rebuild) |
| **State derivation** | `state` DB from `eventlog` | Views + ML tables from `eventlog` ✓ |
| **Time travel** | Replay from seq N | Filter by timestamp ✓ |
| **Multi-user** | Real-time sync | Git-based (eventual) |
| **Primary use case** | Collaborative apps | ML knowledge extraction |

## Implementation Status

**Phase 1: Unified Eventlog** ✅ COMPLETE (2025-11-22)
1. [x] Implement unified eventlog table (2025-11-21)
2. [x] Update git scraper to populate eventlog (2025-11-21)
3. [x] Update sessions scraper to populate eventlog (2025-11-21)
4. [x] Update code scraper to populate eventlog (2025-11-22)
5. [x] Add materialized views (commits, sessions, observations, code tables) (2025-11-22)
6. [x] Validate cross-cutting queries (2025-11-22)

**Stats (patina codebase):**
- 16,027 total events across 17 event types
- 41MB unified patina.db
- All tests passing (80+)
- Zero functionality lost

**Phase 2: Integration & Enhancements** (Next)
- [ ] Integrate with scry (spec-mothership-service.md)
- [ ] Add time-travel CLI (`--until` flag)
- [ ] Document recipe versioning (oxidize_meta table)
- [ ] Implement oxidize embeddings pipeline (spec-oxidize.md)

## References

- LiveStore: event-sourcing framework (layer/dust/repos/livestore)
- build.md: Phase 1 complete, unified DB architecture
- spec-scrape-pipeline.md: Implementation details
- spec-mothership-service.md: Scry integration
- spec-cross-project.md: Multi-user workflows
