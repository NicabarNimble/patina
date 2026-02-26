---
type: feat
id: data-emission-completeness
status: draft
created: 2026-02-26
blocked_by:
- data-db-split
- data-db-split-fixes
sessions:
  origin: 20260226-124149
related:
- data-architecture-v2
beliefs:
- events-are-autobiography-not-telemetry
- check-existing-emissions-before-adding
exit_criteria:
- id: measure-capture-event-emitted-for-every-scraper-git-layer-beliefs-forge
  text: 'measure.capture event emitted for every scraper: git, layer, beliefs, forge'
  checked: false
- id: scry-query-events-fire-regardless-of-session-state-no-early-return-on-missing-session-id
  text: scry.query events fire regardless of session state — no early return on missing session_id
  checked: false
- id: context-and-assay-commands-emit-usage-events-to-events-db
  text: context and assay commands emit usage events to events.db
  checked: false
- id: patina-doctor-reports-emission-coverage-which-commands-emit-which-don-t
  text: '`patina doctor` reports emission coverage — which commands emit, which don''t'
  checked: false
- id: session-lifecycle-events-target-events-db-not-patina-db-after-db-split
  text: session lifecycle events target events.db (not patina.db) after db-split
  checked: false
---
# feat: Emission Completeness — No Silent Operations

> Fill the emission gaps so every tool operation produces an event in events.db.
> Area 2 of [[data-architecture-v2]]. Blocked by Area 1 (data-db-split).

## Problem

Patina's autobiography is incomplete. Several commands run silently — no event
in the eventlog records that they happened, when, or how long they took.

The project can't answer "how has scrape performance trended?" because 3 of 4
scrapers (git, layer, beliefs) don't emit `measure.capture` events. Context
and assay queries leave no trace at all. Scry drops events when no session is
active — losing search behavior data outside tracked sessions.

**The 7 real gaps (corrected from original 8):**

| # | Gap | File | What Exists | What's Missing |
|---|-----|------|-------------|----------------|
| 1 | scrape git | `src/commands/scrape/git/mod.rs` | Emits `git.commit`, `git.tag` (source-derived) | No `measure.capture` for scrape timing/counts |
| 2 | scrape layer | `src/commands/scrape/layer/mod.rs` | Emits `pattern.*` (source-derived) | No `measure.capture` for scrape timing/counts |
| 3 | scrape beliefs | `src/commands/scrape/beliefs/mod.rs` | Emits `belief.surface` (source-derived) | No `measure.capture` for scrape timing/counts |
| 4 | scrape forge | `src/commands/scrape/forge/mod.rs` | Emits `forge.issue`, `forge.pr` (runtime cache) | No `measure.capture` for scrape timing/counts |
| 5 | context command | `src/commands/context.rs` | Nothing | No event at all — pure read, no trace |
| 6 | assay command | `src/commands/assay/mod.rs` | Nothing | No event at all — pure read, no trace |
| 7 | scry without session | `src/commands/scry/internal/logging.rs:89` | Emits `scry.query` when session active | Early return `?` on `get_active_session_id()` drops event when no session |

**Note:** Session lifecycle events (session.started, session.ended, etc.)
already exist in `src/commands/session/internal.rs`. The original gap list
was wrong — these emitters are already wired. After db-split, they need to
target events.db instead of patina.db, but that's Area 1's job.

## Solution

### 1. Wire measure.capture into scrapers (gaps 1-4)

Each scraper's `run()` function already computes timing and item counts.
Add a `measure::emit()` call at the end of each run, following the pattern
established by `scrape code` (the one scraper that already emits).

**Pattern** (already exists in code scraper):
```rust
measure::emit(&conn, "capture", "scrape", "git", &serde_json::json!({
    "commits_processed": stats.items_processed,
    "duration_ms": stats.time_elapsed.as_millis(),
}))?;
```

Apply to: `git/mod.rs`, `layer/mod.rs`, `beliefs/mod.rs`, `forge/mod.rs`.

**After db-split:** The `conn` parameter targets events.db, not patina.db.
The scrapers write source-derived data to patina.db and their performance
event to events.db — two different connections in one run.

### 2. Fix scry session-id early return (gap 7)

In `src/commands/scry/internal/logging.rs:89`:
```rust
// BEFORE: early return drops event
let session_id = get_active_session_id()?;

// AFTER: session_id is optional context
let session_id = get_active_session_id().unwrap_or(None);
```

Include `session_id` in the event JSON as an optional field. When null,
the event still fires — consistent with forward-compatible JSON conventions.

Affects both `log_scry_query()` and `log_scry_query_with_routing()`.

### 3. Add context and assay usage events (gaps 5-6)

Emit a lightweight usage event when context or assay is invoked:

- Event type: `context.query` or `assay.query`
- Data: `{ "topic": <topic>, "duration_ms": <ms> }` for context;
  `{ "query_type": <type>, "pattern": <pattern>, "duration_ms": <ms> }` for assay
- These are runtime events → events.db after db-split

**Note:** These event types need to be added to the event registry in
data-architecture-v2 SPEC.md before implementation.

### 4. Doctor emission coverage audit

Add an emission coverage check to `patina doctor`:
- For every command with side effects, verify a corresponding emit exists
- Report: which commands emit, which don't, which event types have zero
  events in events.db
- Surfaces as a doctor warning, not a build failure

## Non-Goals

- **New event types beyond the registry.** This spec wires existing gaps, not
  new event domains. decision.made, discovery.*, etc. remain Planned.
- **Event schema standardization.** Forward-compatible JSON conventions are
  already established. This spec follows them, doesn't change them.
- **Scrape performance optimization.** Adding one INSERT per scrape run is
  negligible overhead. Perf work belongs in Area 5.
- **Session event rewiring.** Session events already exist. Retargeting them
  to events.db is Area 1's job (db-split), not Area 2's.
