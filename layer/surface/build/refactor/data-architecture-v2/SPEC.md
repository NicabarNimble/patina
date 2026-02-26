---
type: refactor
id: data-architecture-v2
status: draft
created: 2026-02-26
sessions:
  origin: 20260226-065302
  audit: 20260226-094014
beliefs:
  - measure-reads-tables-not-events
  - seq-order-is-not-timestamp-order
  - check-existing-emissions-before-adding
exit_criteria:
  - "measure.* and forge.* events survive scrape --rebuild unchanged"
  - "scrape --rebuild restores runtime events after database recreation"
  - "patina scrape git --fast completes in <2s (no co-change rebuild)"
  - "git post-commit hook triggers fast incremental git scrape"
  - "hooks are opt-in (patina hooks install) and removable (patina hooks remove)"
  - "patina measure --full returns comprehensive JSON snapshot for LLM consumption"
---
# refactor: Event-Sourced Data Architecture

> Protect runtime events from rebuild destruction, eliminate manual scrape friction
> with git hooks, and expose rich LLM query surface — via three independent phases
> that can ship separately.

## Problem

patina.db conflates two fundamentally different kinds of data:

1. **Runtime-only events** (96 events, 0.1%) — measure.*, forge.* API cache.
   These cannot be regenerated from source. `scrape --rebuild` destroys them.

2. **Source-derived cache** (95,571 events, 99.9%) — code.*, git.*, pattern.*,
   session.*, belief.surface. All derived from git repo + layer files on disk.
   Can be regenerated anytime by re-running scrapers.

Today's `execute_rebuild()` does `std::fs::remove_file(patina.db)` — this
destroys both categories indiscriminately.

Additionally, all data flow is batch-manual: users must run `patina scrape`
after every commit. Incremental git scrape takes 15.7s due to co-change
rebuild (O(n²) on 20K+ commit_files), making git hooks impractical.

## Audit Findings (session 20260226-094014)

### Event Classification Reality

Audited all 26 event types against source code:

| Classification | Event Types | Count |
|---|---|---:|
| **Source-derived** | code.*, git.*, pattern.*, session.*, belief.surface | 95,571 |
| **Runtime-only** | measure.*, forge.* | 96 |

Key corrections from original draft:
- **session.ended** — classified as runtime in original spec, but is actually
  **source-derived**. After Phase 2 hardening (commit cb1f9ffd), all 661
  session.ended events are reconstructed from archived session files during
  layer scrape using DELETE+INSERT idempotent pattern.
- **belief.surface** — also source-derived. Emitted by belief scraper from
  markdown files on disk, not from runtime activity.

### The Eventlog Is Not Broken

The original spec said "DELETE+INSERT violates append-only." This is correct
as a description but wrong as a diagnosis. The eventlog serves two roles:

1. **Staging area** for source-derived cache — DELETE+INSERT is correct here
   (it's cache invalidation, not data loss).
2. **Permanent store** for runtime events — these should never be deleted.

The problem isn't "append-only is violated" — it's that both roles share one
table, and `--rebuild` deletes the file containing both.

### ATTACH Complexity Underestimated

Original spec claimed "no code changes needed for queries that span both" DBs.
Audit found ~15 query callsites in measure/internal.rs alone that reference
the eventlog table. SQLite ATTACH requires prefixing: `events.eventlog` vs
`project.beliefs`. A full DB split would touch ~20 source files.

### Incremental Git Scrape Is Too Slow

Timed `patina scrape git` (incremental, 5 new commits): **15.7 seconds**.
Bottleneck: co-change rebuild processes all 20,771 commit_files rows every
time (O(n²)). Post-commit hooks need <2s.

## Revised Architecture: Three Independent Specs

The original spec tried to solve 3 problems at once. These should be
independent specs that can ship separately:

### Spec A: Protected Rebuild (this spec)

**Solve:** measure.* and forge.* events lost on rebuild.
**Approach:** Export/restore runtime events around rebuild, not a full DB split.
**Size:** 1-2 sessions. ~3 files changed.

### Spec B: Fast Incremental Scrape + Git Hooks

**Solve:** Manual scrape friction, slow incremental updates.
**Approach:** New `--fast` mode that skips co-change/FTS rebuild. Git hooks.
**Blocked by:** Nothing (independent).
**Size:** 2-3 sessions. ~5 files changed.

### Spec C: Rich LLM Query Surface

**Solve:** LLM needs comprehensive project snapshot.
**Approach:** `patina measure --full` with domain-organized JSON.
**Blocked by:** Nothing (independent).
**Size:** 1-2 sessions. ~2 files changed.

## Phase 1: Protected Rebuild (this spec's scope)

### Design

Instead of splitting into two databases (20+ files changed), protect runtime
events during rebuild:

```
execute_rebuild() now does:
1. Export runtime events to temp table or file
   SELECT * FROM eventlog WHERE event_type LIKE 'measure.%'
      OR event_type LIKE 'forge.%'
2. Delete patina.db
3. Run all scrapers (recreates DB)
4. Restore runtime events from backup
   INSERT INTO eventlog (...) VALUES (...)
```

This is ~30 lines of code in `src/commands/scrape/mod.rs`.

### Why Not a Full DB Split?

A full split (events.db + project.db) was the original proposal. Audit found:
- **20+ files** need modification (every command opens patina.db)
- Every eventlog query needs ATTACH table prefix rewriting
- The "sacred" events.db would contain only 96 events (0.1% of data)
- The complexity cost far exceeds the protection benefit

Protected rebuild achieves the same safety guarantee with 100x less code change.

If the project grows to have thousands of runtime events, the split can be
revisited. For now, 96 events fit in a temp file trivially.

### Steps

1. In `execute_rebuild()`, before `remove_file(db_path)`:
   - Query `SELECT * FROM eventlog WHERE event_type LIKE 'measure.%' OR event_type LIKE 'forge.%'`
   - Store results in memory (Vec of tuples) or temp JSON file
2. After all scrapers complete:
   - Re-insert preserved events with original timestamps and seq values
3. Add test: rebuild with measure events → verify they survive

### Files Changed

- `src/commands/scrape/mod.rs` — export/restore logic in execute_rebuild()
- `src/eventlog.rs` — optional: add `export_runtime_events()` / `restore_runtime_events()` helpers

### Exit Criteria (Phase 1)

- [ ] measure.* and forge.* events survive scrape --rebuild unchanged
- [ ] scrape --rebuild restores runtime events after database recreation
- [ ] Event count and content identical before and after rebuild

## Phase 2: Fast Incremental Scrape + Git Hooks (separate spec)

### Problem

`patina scrape git` takes 15.7s even for 5 new commits because:
- Co-change rebuild: DELETE all 25K rows, recompute O(n²) — **~12s**
- Git tags: DELETE+rebuild 1,682 tags — **~1s**
- FTS rebuild: DELETE+rebuild 2,787 entries — **~1s**
- Actual commit insertion: 5 commits — **<0.1s**

### Design

New `patina scrape git --fast` mode:
- Insert new commits only (no co-change rebuild)
- Skip tag rebuild (tags don't change on commit)
- Skip FTS rebuild (defer to next full scrape)
- Target: <2s total

Git hooks:
```bash
# .git/hooks/post-commit (fire-and-forget, non-blocking)
#!/bin/sh
patina scrape git --fast 2>/dev/null &
```

Hook lifecycle:
- `patina hooks install` — creates hooks (asks for confirmation)
- `patina hooks remove` — removes hooks
- `patina hooks status` — shows what's installed
- Hooks check `command -v patina` before running
- Hooks run in background (`&`) — never block git operations
- Stderr goes to `/dev/null` — silent failures

### Open Questions

- Should post-merge trigger layer scrape? (layer files change on merge)
- Should post-checkout trigger code scrape? (code changes on branch switch)
- How to handle DB lock contention (hook + manual scrape simultaneously)?

## Phase 3: Rich LLM Query Surface (separate spec)

### Design

`patina measure --full` returns domain-organized JSON:

```json
{
  "generated": "2026-02-26T14:40:00Z",
  "beliefs": {
    "total": 163,
    "grounded": 43,
    "floating": 120,
    "recently_active": ["belief-id-1", "belief-id-2"],
    "weakest": [{"id": "...", "health": 0.2}],
    "strongest": [{"id": "...", "health": 1.0}],
    "stale": 0
  },
  "sessions": {
    "total": 661,
    "recent_7d": 12,
    "commits_7d": 45,
    "beliefs_captured_7d": 5
  },
  "code": {
    "files": 247,
    "functions": 2412,
    "types": 977
  },
  "search_quality": {
    "latest_p5": 0.6,
    "latest_mrr": 0.45
  },
  "actions": ["Run oxidize to update embeddings", ...]
}

```

### Query Mapping

| Field | Source | Exists? |
|---|---|---|
| beliefs.total | `SELECT COUNT(*) FROM beliefs` | Yes |
| beliefs.grounded | `WHERE grounding_score > 0` | Yes |
| beliefs.recently_active | `ORDER BY last_activity DESC LIMIT 5` | New query |
| beliefs.weakest | `ORDER BY health_score ASC LIMIT 5` | New query |
| beliefs.stale | From audit stale_days logic | Yes (in audit) |
| sessions.total | `COUNT(*) FROM eventlog WHERE event_type = 'session.ended'` | Yes |
| sessions.recent_7d | `WHERE timestamp > date('-7d')` | New query |
| code.files | `COUNT(*) FROM index_state` | Yes |
| code.functions | `COUNT(*) FROM function_facts` | Yes |
| search_quality | From measure.search events | Yes |

~60% reuses existing queries. ~40% needs new queries (all straightforward).

MCP integration: `mcp_measure()` already exists — add `mcp_measure_full()`
that calls a new `build_full_snapshot()` function.

## Design Decisions

### Why protected rebuild, not DB split?

- **96 runtime events** vs 95K source-derived. The "sacred" data fits in memory.
- DB split touches 20+ files. Protected rebuild touches 2.
- ATTACH requires rewriting every eventlog query with table prefixes.
- If runtime events grow significantly (>10K), revisit the split decision.

### Why separate specs, not one monolith?

Each phase delivers independent value:
- Phase 1 alone prevents data loss (the critical problem)
- Phase 2 alone reduces friction (the usability problem)
- Phase 3 alone improves LLM ergonomics (the interface problem)

Shipping them separately means faster feedback and smaller blast radius.

### Why git hooks, not file watching?

- Git hooks are built into git — no new dependencies
- They fire at exactly the right moment (after commit, not on every save)
- They're opt-in and user-visible (in .git/hooks/)
- File watching requires a daemon process — heavier, more failure modes

### Model-swap requirement

The LLM query interface is MCP tool calling — model-agnostic by design.
Any model that supports tool calling can call `patina measure --full` and
reason over the JSON. No model-specific code in the data layer.

## References

- Session 20260225-221415 — Measure edge-finding, 4 bugs fixed
- Session 20260226-065302 — Rebuild resilience, 3 fixes, spec drafted
- Session 20260226-094014 — Deep audit, spec revised
- Belief: [[measure-reads-tables-not-events]]
- Belief: [[seq-order-is-not-timestamp-order]]
- Belief: [[check-existing-emissions-before-adding]]
