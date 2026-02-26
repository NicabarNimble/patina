# Design: Database Split — events.db + patina.db Separation

## Approach

Single implementation commit covering all 7 spec steps. The change touches
14 files across 3 categories: infrastructure (eventlog.rs, measure.rs),
runtime writers (scry logging, MCP scry, forge), and readers (measure).

Key design decisions:
- `measure::emit()` no longer takes a connection — opens events.db internally.
  This simplifies all 7 callers and makes the function self-contained.
- Forge `insert_issues`/`insert_prs` take dual connections (patina + events)
  since they write to both databases (eventlog → events.db, materialized
  views → patina.db). The dedup check also moved to events.db.
- Measure uses ATTACH pattern: open patina.db, ATTACH events.db AS events,
  then prefix runtime event queries with `events.eventlog`. Graceful fallback
  if events.db doesn't exist yet.
- `ensure_events_db()` is called from both writers (via `open_events_db()`)
  and readers (measure, explicitly) to guarantee migration runs on first use.
- Migration uses ATTACH internally: events.db ATTACHes patina.db to copy
  runtime events in a single SQL statement. Idempotent — checks existence first.

## Commits
1. `fc02bcaa` — feat: split events.db from patina.db — all 7 spec steps
2. `8fbfceb1` — style: apply cargo fmt
3. `d2374958` — fix: remove unused re-exports from database.rs

## Key Files
- `src/eventlog.rs` — EVENTS_DB constant, ensure_events_db(), open_events_db()
- `src/measure.rs` — emit() opens events.db internally (no conn param)
- `src/commands/scrape/mod.rs` — execute_rebuild() guards events.db
- `src/commands/scrape/forge/mod.rs` — dual-connection insert pattern
- `src/commands/scry/internal/logging.rs` — all logging writes to events.db
- `src/mcp/server/scry.rs` — MCP logging + handle_detail cross-db reads
- `src/commands/measure/internal.rs` — ATTACH pattern for cross-system queries

## Open Questions
None — all resolved during implementation.
