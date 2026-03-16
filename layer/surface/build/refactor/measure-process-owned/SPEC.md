---
type: refactor
id: measure-process-owned
status: draft
created: 2026-03-10
sessions:
  origin: 20260310-074810
related:
- ducklake
- http-proxy-extraction
- pipe-architecture
beliefs:
- telemetry-is-process-owned
- measure-first
- measure-reads-tables-not-events
- children-have-agency-toys-are-capabilities
exit_criteria:
- id: shared-vocabulary
  text: "patina-pipe exports the measure vocabulary: VALID_VERBS, MeasureEvent struct (verb, tool, mode, metrics, timestamp, source, schema_version). Same 5 verbs, same shape, available to any binary in the workspace."
  checked: false
- id: child-local-emit
  text: "patina-pipe provides a MeasureEvent type that children use to validate and emit to their own storage. DuckLake child writes to its DuckLake catalog. Vocabulary is shared, storage is local."
  checked: false
- id: mother-queries-children
  text: "Mother can query child measure state by reading the child's store directly (e.g., opening lake.ducklake read-only to read _measure table). Contract: same table schema, same event envelope."
  checked: false
- id: core-unchanged
  text: "Existing core measure path (measure::emit → events.db) is unchanged. Core commands (scrape, oxidize, eval, etc.) continue to write to project events.db."
  checked: false
- id: proxy-no-telemetry
  text: "Shared HTTP proxy in patina-pipe does not emit measure events. Callers wrap the proxy with their own telemetry."
  checked: false
---
# refactor: Actor-Owned Telemetry — Extend Measure for Autonomous Children

> The measure vocabulary (verb/tool/mode) is universal. The storage
> is local. Each actor with agency — Mother and children — captures
> its own telemetry using the same language. Toys are capabilities
> that do not emit telemetry; the actor using a toy measures it.
> Mother queries, she doesn't centralize. Per
> [[children-have-agency-toys-are-capabilities]]: children own
> their workflow, including observability of their toy usage.

## Problem

The measure system assumes everything runs inside the main patina
binary and writes to one project's events.db:

```rust
// src/measure.rs
pub fn emit(verb: &str, tool: &str, mode: &str, metrics: &Value) -> Result<()>
```

This works for core commands (scrape, oxidize, eval). It breaks
for autonomous children:

- **DuckLake child** is a separate binary — can't call
  `crate::measure::emit`, has no project events.db
- **Shared HTTP proxy** in patina-pipe can't import
  `crate::measure` — it's in the main binary
- **Future children** (transforms, other lake types) will have
  the same problem

The vocabulary is right. The storage model is wrong for a
multi-process architecture.

## What Stays the Same

The measure vocabulary is immutable:

```
5 verbs:  capture, index, search, believe, evolve
Tools:    scrape, session, eval, oxidize, doctor, belief, pipe, hook, bench
Modes:    tool-specific (code, git, beliefs, http, build, train, ...)
Call:     emit(verb, tool, mode, metrics)
```

Core commands in the main binary continue to use
`measure::emit_or_warn` → events.db. Nothing changes for them.

Plugins continue to use the WIT `record_measurement` interface
with host-side validation. Nothing changes for them.

## What Changes

### Canonical event envelope

The event shape moves to `patina-pipe` with a minimal canonical
envelope. This is the contract — every measure event everywhere
must have these fields:

```rust
// crates/patina-pipe/src/measure.rs

pub const VALID_VERBS: &[&str] = &["capture", "index", "search", "believe", "evolve"];

pub struct MeasureEvent {
    pub schema_version: u32,  // 1 — for forward compat
    pub verb: String,         // must be in VALID_VERBS
    pub tool: String,         // WHAT measured: pipe, lake, scrape, oxidize, ...
    pub mode: String,         // sub-category within tool
    pub metrics: Value,       // JSON object with numeric values
    pub timestamp: String,    // RFC3339
    pub source: String,       // WHO emitted: "core", "plugin:name", "child:ducklake"
}

pub fn validate(event: &MeasureEvent) -> Result<(), String>;
```

Core `src/measure.rs` imports `VALID_VERBS` from patina-pipe
instead of defining its own. Same function, shared constant.

### tool vs source — clear boundary

**`tool`** is WHAT measured. It names the subsystem that produced
the data. Tools are a controlled vocabulary:

```
Existing:  scrape, session, eval, oxidize, doctor, belief, pipe, hook, bench
New:       lake
```

**`source`** is WHO emitted. It identifies the actor:

```
core              — main patina binary (Mother)
plugin:<name>     — WASM plugin (e.g., plugin:github-connector)
child:<name>      — autonomous child (e.g., child:ducklake)
```

Same tool from different sources = different rows, same
vocabulary. Example: both Mother's broker and the DuckLake child
emit `(capture, pipe, http)` — the tool is `pipe`, the mode is
`http`. They are distinguished by source: `core` vs
`child:ducklake`. The child measures its own use of approved
toys (connector, HTTP proxy). Mother measures her own
orchestration. Neither centralizes the other's telemetry.

**Children cannot define new tools** without updating the shared
vocabulary. Modes are tool-scoped and documented per tool (see
below). This keeps the vocabulary tight.

### Registered modes per tool

Modes are documented per tool. Each tool has a fixed set:

| Tool | Modes | Owner |
|------|-------|-------|
| scrape | code, git, beliefs, layer, structure, health-check | core |
| oxidize | build, train | core |
| eval | dimension, feedback, nl | core |
| belief | audit | core |
| session | lifecycle | core |
| pipe | http | core + children |
| hook | post-commit, post-merge | core |
| bench | (query set names) | core |
| doctor | (tbd) | core |
| **lake** | **ingest, cursor, error** | **children** |

New modes require updating the vocabulary. This prevents sprawl.

### Each process captures locally

**Core (main binary)** — unchanged:
```rust
measure::emit_or_warn("capture", "scrape", "code", &json!({...}));
// → events.db, source = "core"
```

**DuckLake child** — writes to its DuckLake catalog:
```rust
self.emit_measure("capture", "lake", "ingest", &json!({
    "records_written": 847,
    "tables_created": 2,
    "duration_ms": 1234,
}));
// → lake._measure table, source = "child:ducklake"
```

**Broker HTTP wrapper** — unchanged:
```rust
measure::emit_or_warn("capture", "pipe", "http", &json!({...}));
// → events.db, source = "core"
```

**DuckLake child HTTP wrapper** — child measures its own toy usage:
```rust
self.emit_measure("capture", "pipe", "http", &json!({...}));
// → lake._measure table, source = "child:ducklake"
```

Same verb/tool/mode. Different storage. Different source.
The child owns telemetry for its approved toys. The proxy toy
itself emits nothing — the child wraps it with measurement.

### Mother retrieval path

Mother queries measure data from each process's store. The
contract: every store that captures measure events uses the
same table schema matching the canonical envelope.

**Project metrics** — `patina measure` reads events.db (existing):
```sql
SELECT * FROM eventlog
WHERE event_type LIKE 'measure.%'
ORDER BY timestamp DESC;
```

**Lake metrics** — Mother opens lake.ducklake read-only:
```sql
-- DuckLake child writes to this table
SELECT * FROM _measure ORDER BY timestamp DESC LIMIT 20;

-- Mother reads it for status reporting
SELECT tool, mode, MAX(timestamp) as last_run,
       SUM(json_extract(metrics, '$.records_written')) as total_records
FROM _measure
WHERE verb = 'capture'
GROUP BY tool, mode;
```

`patina mother status` aggregates across stores: scans project
events.db + opens each lake's `_measure` table read-only. The
shared `MeasureEvent` schema makes cross-store queries consistent.

**Why not pipe/status?** Reading the store directly is simpler
and works even when the child isn't running. pipe/status is
optional for live children but the store is always queryable.

## Dependency Ordering

```
http-proxy-extraction    measure-process-owned
(no built-in telemetry)  (shared vocabulary)
         │                       │
         └───────┬───────────────┘
                 │
              ducklake
         (uses both: proxy for HTTP,
          vocabulary for local measure)
```

`http-proxy-extraction` and `measure-process-owned` are parallel.
The proxy lands with "no built-in telemetry" as a design choice.
The measure vocabulary can land independently. DuckLake needs both.

## Steps

1. Add `MeasureEvent`, `VALID_VERBS`, `validate()` to
   `patina-pipe/src/measure.rs`
2. Core `src/measure.rs` imports verbs from patina-pipe
3. Document registered tools and modes in patina-pipe
4. DuckLake child uses patina-pipe vocabulary, writes to
   `_measure` table in its DuckLake catalog
5. `patina mother status` queries lake `_measure` for
   aggregated view

## Key Files

**New:**
- `crates/patina-pipe/src/measure.rs` — shared vocabulary

**Refactor:**
- `src/measure.rs` — imports VALID_VERBS from patina-pipe

**Consumers:**
- `children/ducklake/src/main.rs` — child-local emit
- `src/commands/mother.rs` — aggregated status

**Unchanged:**
- `src/commands/measure/` — CLI reads events.db
- `src/eventlog.rs` — project-local storage
- All existing `emit_or_warn` call sites
- Plugin WIT `record_measurement` interface

## Non-Goals

- Changing the 5 protocol verbs
- Changing how core commands emit
- Changing the plugin WIT measure interface
- Centralized telemetry aggregation service
- Real-time streaming of child metrics to Mother
- Standardizing child storage format beyond the shared envelope
- Telemetry hooks or callbacks in the HTTP proxy
- Open-ended mode registration (modes are documented per tool)
