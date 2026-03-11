# Design: Process-owned telemetry — extend measure for autonomous children

## Approach

The measure system today is a single function in the main binary
(`src/measure.rs:27 emit`) that writes to one project's events.db.
The vocabulary is right — 5 verbs, tool/mode/source. The storage
model is wrong — it assumes one binary, one database.

This refactor extracts the vocabulary and event envelope to
`patina-pipe` so any binary in the workspace can validate and
emit measure events. The main binary's `src/measure.rs` imports
`VALID_VERBS` from patina-pipe instead of defining its own. The
storage stays local to each actor — core writes events.db, the
DuckLake child writes to its `_measure` table.

### What exists today

`src/measure.rs` (69 lines):
- `VALID_VERBS: &[&str]` at line 12 — `capture, index, search, believe, evolve`
- `emit()` at line 27 — validates verb, builds JSON, writes to events.db via `crate::eventlog`
- `emit_or_warn()` at line 64 — try-emit, log warning on failure

15 call sites across the main binary, all using `emit_or_warn`:

| Verb | Tool | Mode | Caller |
|------|------|------|--------|
| capture | scrape | code | `commands/scrape/code/mod.rs:122` |
| capture | scrape | structure | `commands/scrape/code/mod.rs:154` |
| capture | scrape | git | `commands/scrape/git/mod.rs:891` |
| capture | scrape | beliefs | `commands/scrape/beliefs/mod.rs:1773` |
| capture | scrape | layer | `commands/scrape/layer/mod.rs:515` |
| capture | pipe | http | `broker/http.rs:125` |
| capture | hook | (hook_name) | `commands/hook/internal.rs:73` |
| index | oxidize | build | `commands/oxidize/mod.rs:128` |
| index | oxidize | train | `commands/oxidize/mod.rs:367` |
| search | eval | dimension | `commands/eval/mod.rs:301` |
| search | eval | feedback | `commands/eval/mod.rs:1195` |
| search | eval | nl | `commands/eval/mod.rs:1565` |
| search | bench | (query_set) | `commands/bench/internal.rs:503` |
| believe | belief | audit | `commands/belief/mod.rs:747` |

All 15 call sites use `source = "core"` implicitly (hardcoded in
`emit()` at line 44). None of these change. The refactor is purely
additive — shared vocabulary in patina-pipe, core imports from there.

### The shared layer

`patina-pipe/src/measure.rs` provides:

1. **`VALID_VERBS`** — the canonical constant, single source of truth
2. **`MeasureEvent`** — the canonical envelope struct with schema_version
3. **`validate()`** — validates verb against VALID_VERBS, checks required fields

The core `src/measure.rs` imports `VALID_VERBS` from patina-pipe
and drops its local definition. The `emit()` function still writes
to events.db via `crate::eventlog` — that path is unchanged.

Children import `MeasureEvent` and `validate()` to build validated
events, then write them to their own storage using their own
connection. The contract: same envelope shape, same vocabulary.

### tool vs source in the envelope

Currently `emit()` sets `source = "core"` unconditionally (line 44).
The `MeasureEvent` struct makes source explicit:

```
source = "core"              — main patina binary (Mother)
source = "plugin:<name>"     — WASM plugin
source = "child:<name>"      — autonomous child (e.g., child:ducklake)
```

Core's `emit()` continues to set `source = "core"`. Children set
their own source. Same tool/mode from different sources are
distinguishable in queries.

### Mother reads, she doesn't centralize

Mother queries child telemetry by reading the child's store:

- Project metrics: `SELECT * FROM eventlog WHERE event_type LIKE 'measure.%'`
- Lake metrics: open `lake.ducklake` read-only, `SELECT * FROM _measure`

The `MeasureEvent` schema makes cross-store queries consistent.
No pipe protocol needed for telemetry retrieval — the store is
always queryable, even when the child isn't running.

## Commits

1. **`pipe: add shared measure vocabulary — MeasureEvent, VALID_VERBS, validate`**
   Create `crates/patina-pipe/src/measure.rs` with:
   - `pub const VALID_VERBS: &[&str]`
   - `pub struct MeasureEvent { schema_version, verb, tool, mode, metrics, timestamp, source }`
   - `pub fn validate(event: &MeasureEvent) -> Result<(), String>`
   Add `pub mod measure` to `crates/patina-pipe/src/lib.rs`.
   Tests: valid verbs accepted, invalid rejected, all required fields
   checked, source format validated.

   **Why:** The vocabulary must be in patina-pipe before any child
   can use it. This is the foundation commit — types only, no
   behavior changes in the main binary.

2. **`measure: import VALID_VERBS from patina-pipe`**
   `src/measure.rs` changes `VALID_VERBS` from a local const to a
   re-export from `patina_pipe::measure::VALID_VERBS`. Delete the
   local definition. The `emit()` function is otherwise unchanged —
   still constructs JSON, still writes to events.db.

   **Why:** Single source of truth for the vocabulary. If a verb is
   added or renamed, it happens in one place. All existing call
   sites unchanged.

3. **`measure: add source field to emit — explicit "core" origin`**
   Update `emit()` to include `"source": "core"` in the event JSON
   (already present at line 44, but now uses the `MeasureEvent`
   struct for validation before writing). The `source_id` field
   in eventlog remains `tool:mode` — `source` is in the JSON data.
   Add `schema_version: 1` to the event data.

   **Why:** Core events need the same envelope shape as child events
   for cross-store query consistency. Adding schema_version now
   enables forward-compatible reads.

4. **`pipe: document registered tools and modes`**
   Add a `REGISTERED_TOOLS` doc constant or module-level doc comment
   to `patina-pipe/src/measure.rs` documenting the current tool/mode
   vocabulary from the table above. This is documentation, not
   runtime enforcement — modes are tool-scoped and documented.

   **Why:** The spec says "new modes require updating the vocabulary."
   Having the canonical list in the code (not just the spec) makes
   it visible to child authors who will `use patina_pipe::measure`.

## Key Files

### New
- `crates/patina-pipe/src/measure.rs` — shared vocabulary: `VALID_VERBS`, `MeasureEvent`, `validate()`

### Modified
- `crates/patina-pipe/src/lib.rs` — add `pub mod measure`
- `src/measure.rs` — import `VALID_VERBS` from patina-pipe, add source + schema_version to event data

### Consumers (future, not this spec)
- `children/ducklake/src/main.rs` — imports `MeasureEvent`, writes to `_measure` table
- `src/commands/mother.rs` — queries `_measure` for aggregated status

### Unchanged
- All 14 `emit_or_warn` call sites in commands/ — no signature change
- `broker/http.rs:125` — still calls `measure::emit_or_warn("capture", "pipe", "http", ...)`
- `src/eventlog.rs` — project-local storage unchanged
- Plugin WIT `record_measurement` interface

## Open Questions

1. **Mode enforcement.** ~~Resolved:~~ `validate()` enforces verb and
   tool at runtime (`VALID_VERBS` + `REGISTERED_TOOLS`). Mode remains
   documentation-only for now — modes are tool-scoped and the
   combinatorial table is harder to maintain as a runtime check.
   Tighten if sprawl actually happens.

2. **schema_version migration.** Adding `schema_version: 1` to event
   data means existing events in events.db don't have it. Queries that
   filter on schema_version need `COALESCE(json_extract(data, '$.schema_version'), 0)`.
   Should we backfill? Recommendation: no backfill — events.db is
   append-only. Queries handle missing version as "pre-schema" (v0).

3. **patina-pipe-types vs patina-pipe.** Should `MeasureEvent` go in
   patina-pipe-types (pure types, no logic) or patina-pipe (has
   `validate()` logic)? The VALID_VERBS constant + validate function
   need to be co-located. Recommendation: patina-pipe, since
   patina-pipe-types is protocol types only and measure is a higher-level
   concern. Children already depend on patina-pipe for the Child trait.
