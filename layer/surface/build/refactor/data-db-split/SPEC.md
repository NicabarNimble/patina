---
type: refactor
id: data-db-split
status: active
created: 2026-02-26
sessions:
  origin: 20260226-124149
related:
- data-architecture-v2
beliefs:
- if-its-patina-its-git
- events-are-autobiography-not-telemetry
- measure-reads-tables-not-events
exit_criteria:
- id: events-db-exists-at-patina-local-data-events-db
  text: events.db exists at .patina/local/data/events.db
  checked: false
- id: runtime-events-measure-scry-forge-write-to-events-db
  text: runtime events (measure.*, scry.*, forge.*) write to events.db
  checked: false
- id: source-derived-events-code-git-session-pattern-belief-surface-write-to-patina-db
  text: source-derived events (code.*, git.*, session.*, pattern.*, belief.surface) write to patina.db
  checked: false
- id: scrape-rebuild-deletes-patina-db-but-leaves-events-db-untouched
  text: '`scrape --rebuild` deletes patina.db but leaves events.db untouched'
  checked: false
- id: runtime-events-survive-a-rebuild-count-before-count-after
  text: 'runtime events survive a rebuild: count before == count after'
  checked: false
- id: measure-reads-both-databases-via-attach-for-cross-system-queries
  text: measure reads both databases via ATTACH for cross-system queries
  checked: false
- id: one-time-migration-copies-existing-runtime-events-from-patina-db-to-events-db
  text: one-time migration copies existing runtime events from patina.db to events.db
  checked: false
- id: pragma-user-version-set-to-1-in-events-db
  text: PRAGMA user_version set to 1 in events.db
  checked: false
---
# refactor: Database Split — events.db + patina.db Separation

> Separate runtime events (the project's autobiography) from source-derived
> projections (the rebuildable cache). Area 1 of [[data-architecture-v2]].

## Current State

One database: `patina.db` holds everything — runtime events (measure.*,
scry.*, forge.*) mixed with source-derived events (code.*, git.*, session.*,
pattern.*, belief.surface) in a single `eventlog` table plus 60+ projection
tables.

`execute_rebuild()` deletes the entire file and recreates from scratch.
This destroys runtime event history — the project loses its operational
memory on every rebuild.

**Current volume:** 97,338 events total. 96 runtime events (154KB), 97,242
source-derived events (19.6MB). Runtime events are a tiny fraction of the
database.

## Target State

Two databases with different lifecycle rules:

```
.patina/local/data/events.db    — runtime events (IRREPLACEABLE)
  Append-only. No DELETE. No UPDATE. Survives rebuilds.
  PRAGMA user_version = 1

.patina/local/data/patina.db    — projections (REBUILDABLE)
  DELETE + INSERT on scrape. Full wipe on rebuild.
  Source-derived eventlog rows + structured tables + FTS5.
```

**Runtime event writers** (~7 files) target events.db:
- `src/measure.rs` — measure::emit()
- `src/commands/scrape/forge/` — forge.issue, forge.pr
- `src/commands/scry/internal/logging.rs` — scry.query, scry.use, scry.feedback
- `src/mcp/server/scry.rs` — MCP scry event path

**Cross-system readers** (~3 files) use ATTACH:
- `src/commands/measure/internal.rs` — health queries spanning both DBs
- `src/commands/eval/mod.rs` — feedback loop (scry.query × commit files)
- Any future cross-system query path

**Rebuild** deletes only patina.db. events.db is never opened by scrape.

## Steps

1. **Create events.db initialization** — new function in eventlog.rs that
   creates events.db with the same eventlog schema shape. Set
   `PRAGMA user_version = 1`. Add `EVENTS_DB` path constant alongside
   existing `PATINA_DB`.

2. **One-time migration** — on first command invocation, check if events.db
   exists. If not, create it and copy runtime events from patina.db:
   ```sql
   INSERT INTO events.eventlog (event_type, timestamp, source_id, source_file, data)
   SELECT event_type, timestamp, source_id, source_file, data
   FROM patina.eventlog
   WHERE event_type LIKE 'measure.%'
      OR event_type LIKE 'scry.%'
      OR event_type LIKE 'forge.%'
   ORDER BY timestamp ASC
   ```
   Verify: row counts match. Migration is a COPY — patina.db retains
   originals until next rebuild.

3. **Rewire runtime writers** — change measure::emit(), forge insert, and
   scry logging to open events.db instead of patina.db. Forge dedup check
   also moves to events.db.

4. **Update rebuild** — `execute_rebuild()` deletes patina.db only. Add
   explicit guard: never open/touch events.db during rebuild.

5. **ATTACH for cross-system reads** — helper function that opens patina.db
   and ATTACHes events.db READONLY. Used by measure and eval --feedback.
   Per-request pattern (open → ATTACH → query → close).

6. **Delete dead code** — remove `create_feedback_views()` from eventlog.rs
   (dead code, no callers).

7. **Verify** — run the gate: `rm patina.db && patina scrape`, then confirm
   runtime events in events.db are unchanged.

## Exit Criteria

See frontmatter. The gate test:
```bash
# Count runtime events before
sqlite3 .patina/local/data/events.db "SELECT COUNT(*) FROM eventlog"
# Rebuild
rm .patina/local/data/patina.db && patina scrape
# Count runtime events after — must be identical
sqlite3 .patina/local/data/events.db "SELECT COUNT(*) FROM eventlog"
```
