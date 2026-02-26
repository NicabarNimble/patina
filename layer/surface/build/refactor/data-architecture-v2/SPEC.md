---
type: refactor
id: data-architecture-v2
status: draft
created: 2026-02-26
sessions:
  origin: 20260226-065302
beliefs:
  - measure-reads-tables-not-events
  - seq-order-is-not-timestamp-order
  - check-existing-emissions-before-adding
exit_criteria:
  - "events.db is append-only — no DELETE, no DROP, no rebuild touches it"
  - "project.db is fully rebuildable from git + events.db in under 60s"
  - "git post-commit hook triggers incremental code+git scrape automatically"
  - "measure.* events survive scrape --rebuild unchanged"
  - "LLM can query project state via MCP without knowing the DB split"
  - "patina measure --full returns comprehensive JSON snapshot for LLM consumption"
---
# refactor: Event-Sourced Data Architecture

> Split patina.db into sacred events.db (append-only history) and rebuildable project.db (derived cache), eliminate manual scrape friction with git hooks and SQLite triggers

## Problem

patina.db conflates two fundamentally different kinds of data:

1. **Irreplaceable history** — measure.* events, live session.ended, forge API cache.
   These cannot be regenerated from git. `scrape --rebuild` destroys them.

2. **Rebuildable cache** — code structure, git commits, patterns, sessions, beliefs.
   These are derived from git and can be regenerated anytime.

Mixing them in one database means:
- `--rebuild` is destructive (loses measure history, forge cache)
- No automatic updates (everything is manual `patina scrape`)
- The eventlog claims to be append-only but isn't (scrapers DELETE before INSERT)
- An LLM querying the system sees a single opaque blob instead of clean layers

Additionally, all data flow is batch-manual. Users must run `patina scrape` after
every change. Real systems use event-driven updates — a commit triggers indexing,
not a human remembering to run a command.

## Current State

### One database: `.patina/local/data/patina.db`

Contains everything:
- eventlog table (~95K events, 26 event types)
- 14+ materialized view tables (commits, beliefs, patterns, sessions, etc.)
- 5 FTS5 indices
- scrape_meta tracking table

### Manual batch pipeline

```
git (source) → manual `patina scrape` → patina.db → manual `patina oxidize` → embeddings
```

Every step requires human invocation. No hooks, no watchers, no triggers.

### Data that does NOT survive rebuild

| Data | In git? | Lost on rebuild? |
|------|---------|-----------------|
| measure.* events | No | Yes (except session.ended, fixed in this session) |
| Forge API cache | No | Re-fetched (slow, rate-limited) |
| Embeddings | No | Must re-run oxidize |
| scrape_meta state | No | Reset |

### Scrapers use DELETE+INSERT, not append

The layer scraper does:
```sql
DELETE FROM eventlog WHERE source_id = ?1 AND event_type LIKE 'pattern.%'
-- then re-inserts
```
This violates append-only semantics. The eventlog is really a mutable cache.

## Target State

### Three-layer data architecture

```
┌─────────────────────────────────────────────────┐
│  Layer 1: SOURCES (immutable, external)         │
│                                                 │
│  Git repo (.git/)     — commits, code, history  │
│  Layer files (layer/) — patterns, sessions,     │
│                         beliefs (on disk)        │
│  GitHub API           — issues, PRs             │
└─────────────────────────────────────────────────┘
                    ↓ (event-driven)
┌─────────────────────────────────────────────────┐
│  Layer 2: EVENTS.DB (append-only, sacred)       │
│                                                 │
│  measure.* events     — tool metrics over time  │
│  session.ended        — live session outcomes   │
│  belief.surface       — belief state snapshots  │
│  forge.*              — cached API responses    │
│  audit.*              — future audit trail      │
│                                                 │
│  RULES:                                         │
│  - Never DELETE rows                            │
│  - Never DROP tables                            │
│  - scrape --rebuild does NOT touch this         │
│  - Only grows, never shrinks                    │
│  - seq is monotonic, timestamp is wallclock     │
└─────────────────────────────────────────────────┘
                    ↓ (derived, rebuildable)
┌─────────────────────────────────────────────────┐
│  Layer 3: PROJECT.DB (cache, disposable)        │
│                                                 │
│  Materialized views:                            │
│    commits, commit_files, co_changes            │
│    patterns, sessions, observations, goals      │
│    beliefs (with computed metrics)              │
│    function_facts, type_vocabulary, call_graph  │
│    forge_issues, forge_prs                      │
│  FTS5 indices:                                  │
│    code_fts, commits_fts, pattern_fts,          │
│    belief_fts, eventlog_fts                     │
│  scrape_meta tracking                           │
│                                                 │
│  RULES:                                         │
│  - Fully rebuildable from git + events.db       │
│  - --rebuild only touches this                  │
│  - DELETE+INSERT is fine (it's a cache)         │
│  - ATTACHes events.db read-only for queries     │
└─────────────────────────────────────────────────┘
                    ↓ (derived)
┌─────────────────────────────────────────────────┐
│  Layer 4: EMBEDDINGS (derived, rebuildable)     │
│                                                 │
│  .patina/local/data/embeddings/                 │
│  usearch indices, projection matrices           │
│  Rebuilt by `patina oxidize`                    │
│                                                 │
└─────────────────────────────────────────────────┘
                    ↓ (federated)
┌─────────────────────────────────────────────────┐
│  Layer 5: MOTHER.DB (cross-project, separate)   │
│                                                 │
│  ~/.patina/mother/graph.db                      │
│  Belief federation, project registry            │
│  Synced by `patina mother graph sync`           │
│                                                 │
└─────────────────────────────────────────────────┘
```

### Automatic data flow (eliminate scrape friction)

**Phase A: Git hooks (immediate wins)**
```
.git/hooks/post-commit  → patina scrape git --incremental (fast, ~1s)
.git/hooks/post-merge   → patina scrape layer --incremental
.git/hooks/post-checkout → patina scrape code --incremental (if branch changed)
```

**Phase B: SQLite triggers (eliminate materialized view lag)**
```sql
-- Example: when git.commit event is inserted into eventlog,
-- auto-update the commits materialized view
CREATE TRIGGER IF NOT EXISTS trg_git_commit
AFTER INSERT ON eventlog WHEN NEW.event_type = 'git.commit'
BEGIN
  INSERT OR REPLACE INTO commits (...)
  VALUES (NEW.source_id, json_extract(NEW.data, '$.message'), ...);
END;
```

**Phase C: File watching (future, optional)**
- Watch layer/surface/epistemic/beliefs/*.md → auto-scrape beliefs
- Watch src/**/*.rs → auto-scrape code
- Only if Phase A/B don't cover enough

### LLM query surface

**`patina measure --full`** returns comprehensive JSON snapshot:
```json
{
  "beliefs": {
    "total": 163, "grounded": 43, "floating": 120,
    "recently_active": [...],
    "weakest": [...], "strongest": [...],
    "stale": 0
  },
  "sessions": {
    "total": 660, "recent_7d": 12,
    "commits_7d": 45, "beliefs_captured_7d": 5
  },
  "search_quality": {
    "latest_p5": 0.6, "history": [...]
  },
  "mother": {
    "connected_projects": 3, "shared_beliefs": 12
  },
  "code": {
    "files": 247, "functions": 2412
  }
}
```

MCP tool returns this — any LLM with tool calling can query it.
The model doesn't need to know about the DB split.

## Steps

### Phase 1: Database Split (events.db + project.db)

1. Create `events.db` schema — append-only eventlog with strict constraints
2. Migrate existing measure.*, session.ended, belief.surface, forge.* events
   from patina.db → events.db
3. Modify `scrape --rebuild` to only drop/recreate project.db, leaving
   events.db untouched
4. Modify project.db queries to ATTACH events.db read-only where needed
5. Update all scrapers: source-derived data → project.db,
   runtime metrics → events.db

### Phase 2: Git Hooks (eliminate scrape friction)

6. Create post-commit hook: incremental git + code scrape
7. Create post-merge hook: incremental layer scrape
8. Make hooks opt-in via `patina init` (respects user consent per
   safety-boundaries)
9. Hooks must be fast (<3s) — only incremental updates

### Phase 3: Rich Query Surface (LLM interface)

10. Implement `patina measure --full` with comprehensive JSON snapshot
11. Add verb drill-down to MCP measure tool (--verb parameter)
12. Add history/trend queries (--history N, --since Nd)
13. Wire as MCP tool — any model can query

### Phase 4: Future (not in this spec)

- SQLite triggers for auto-materialized views
- File watching for layer/ changes
- Cross-project measure federation via mother
- btop-style TUI dashboard (ratatui)

## Exit Criteria

- [ ] events.db is append-only — no DELETE, no DROP, no rebuild touches it
- [ ] project.db is fully rebuildable from git + events.db in under 60s
- [ ] git post-commit hook triggers incremental code+git scrape automatically
- [ ] measure.* events survive scrape --rebuild unchanged
- [ ] LLM can query project state via MCP without knowing the DB split
- [ ] patina measure --full returns comprehensive JSON snapshot for LLM consumption

## Design Decisions

### Why SQLite ATTACH, not a single DB?

SQLite supports `ATTACH DATABASE 'events.db' AS events` — project.db can
query events.db tables as `events.eventlog`. This means:
- events.db can be backed up independently
- project.db can be deleted without risk
- No code changes needed for queries that span both
- Read-only ATTACH prevents accidental writes to events.db

### Why git hooks, not file watching?

- Git hooks are built into git — no new dependencies
- They fire at exactly the right moment (after a commit, not on every save)
- They're opt-in and user-visible (in .git/hooks/)
- File watching requires a daemon process — heavier, more failure modes

### Why not rebuild events from git?

Some events can't be reconstructed:
- measure.* timestamps are the moment the tool ran, not derivable from git
- Forge API data requires re-fetching (rate-limited, slow)
- Live session.ended data has runtime metrics not in the archived file

The session.ended reconstruction from this session (Task 1) is a partial
fix — it recovers what's in archived files but loses live-only data.

### Model-swap requirement

The LLM query interface is MCP tool calling — model-agnostic by design.
Any model that supports tool calling (Claude, GPT, Llama, Mistral) can
call `patina measure --full` and reason over the JSON. No model-specific
code in the data layer.
