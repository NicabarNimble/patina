# Design: Measure as LLM Query Surface — Structured Health for AI Consumers

## Approach

Build `FullMeasureReport` as a typed layer on top of the existing `MeasureReport`
infrastructure from v0.35.7. The existing `build_report()`, typed `VerbMetrics`,
and domain enums (`SourceType`, `ToolName`, `Mode`) remain untouched — new code
extends, not replaces.

Two principles:

1. **Measure is a dashboard — read-only.** Tools emit into events.db, scrape
   materializes into patina.db tables, measure reads both. Zero writes to any
   database. Per [[measure-reads-tables-not-events]] and [[events-are-autobiography-not-telemetry]].

2. **Every field is a typed Rust field.** No `serde_json::Value` in new types.
   Diagnostics and health summary are methods on typed structs, not ad-hoc
   string assembly. Per [[parse-at-boundary-type-the-interior]].

### Pre-Implementation Concerns (Resolved)

All 7 from session [[20260227-075037]]:

1. **VerbStatus enum** — Add `Degraded` as 4th variant. Verb-level: freshness is
   stale AND existing status has issues. Health-level: worst-verb-wins across all 5.

2. **Field schema** — See field table below. Required vs nullable is explicit.

3. **execute_feedback() ownership** — Eval owns the computation
   (`src/commands/eval/mod.rs:965`). Measure reads the P@5 result via the search
   verb's health status. Interface is a value, not a shared connection.

4. **ATTACH helper** — Extract `fn attach_events(conn: &Connection) -> Result<()>`.
   Promote to newtype only when a second consumer appears (Gjengset principle).

5. **health.status derivation** — Worst-verb-wins:
   `Degraded > NeedsAttention > Good > NoData`.

6. **Temporal scope** — v1 is point-in-time (catalog questions 1–7). Questions 8–9
   (trends) are stretch. Don't block completion on temporal.

7. **MCP/CLI unification** — Single function
   `build_full_report(conn) -> Result<FullMeasureReport>`. Both CLI `--full --json`
   and `mcp_measure()` call it and serialize. Same binary, same library function.

### Field Schema

**`FullMeasureReport`** (`#[derive(Debug, Serialize)]`):

| Field | Type | Nullable | Notes |
|-------|------|----------|-------|
| health | HealthSummary | no | aggregate across all verbs |
| verbs | BTreeMap<String, FullVerbSummary> | no | keyed by verb name for stable paths |
| event_counts | EventCounts | no | |

**`HealthSummary`**:

| Field | Type | Nullable | Notes |
|-------|------|----------|-------|
| status | VerbStatus | no | worst-verb-wins |
| summary | String | no | one-sentence, LLM-quotable |
| assessed_at | String | no | RFC3339 |

**`FullVerbSummary`**:

| Field | Type | Nullable | Notes |
|-------|------|----------|-------|
| status | VerbStatus | no | |
| latest_timestamp | Option\<String\> | yes | None when no data |
| age_hours | Option\<f64\> | yes | None when no timestamp |
| freshness | Option\<Freshness\> | yes | None when no timestamp |
| sources | Vec\<SourceSummary\> | no | may be empty |
| diagnostics | Vec\<Diagnostic\> | no | may be empty |

**`Diagnostic`**:

| Field | Type | Nullable | Notes |
|-------|------|----------|-------|
| severity | Severity | no | Warning or Error |
| message | String | no | actionable, specific |

**`EventCounts`**:

| Field | Type | Nullable | Notes |
|-------|------|----------|-------|
| total_runtime_events | i64 | no | events.db total |
| by_type | BTreeMap\<String, i64\> | no | event_type → count |

**New enums:**

- `Freshness` — `Fresh`, `Aging`, `Stale` (serialized lowercase)
- `Severity` — `Warning`, `Error` (serialized lowercase)
- `VerbStatus::Degraded` — added to existing enum

### Freshness Thresholds (Constants)

From SPEC.md, hardcoded — not configurable:

| Verb | Fresh | Aging | Stale |
|------|-------|-------|-------|
| capture | < 24h | 24–72h | > 72h |
| index | < 48h | 48h–7d | > 7d |
| search | < 7d | 7–30d | > 30d |
| believe | < 7d | 7–30d | > 30d |
| evolve | < 7d | 7–30d | > 30d |

### Verbs as Map, Not Array

Existing `MeasureReport` uses `verbs: Vec<VerbSummary>`. `FullMeasureReport` uses
`BTreeMap<String, FullVerbSummary>` so output is `{"capture": {...}, "believe": {...}}`.
BTreeMap gives alphabetical key ordering. Stable JSON path:
`output.verbs.believe.status`.

### BelieveMetrics Extension

Add `contested_count` to existing `BelieveMetrics`. Contested = beliefs with
unresolved attacks (from `belief_attacks` table in patina.db). This changes the
serialized shape of `--json` output — acceptable because it's additive (new field,
no existing fields removed).

### execute_feedback() ATTACH Pattern

Current state: `execute_feedback()` in `src/commands/eval/mod.rs:965` queries
`patina.db` eventlog for both `scry.query` and `git.commit` events. Post-db-split,
`scry.query` events live in `events.db` (confirmed: 21 events), `git.commit` events
stay in `patina.db` (2892 events).

Rewrite: open `events.db`, ATTACH `patina.db`, join `events.eventlog` (scry.query)
with `patina.eventlog` (git.commit). Parse results into typed `FeedbackRow` struct
at the query boundary. The P@5 metric feeds into search verb health via a
`measure.search` event.

## Commits

1. `feat(measure): define FullMeasureReport type hierarchy` — VerbStatus::Degraded,
   Freshness/Severity enums, Diagnostic, FullVerbSummary, HealthSummary,
   FullMeasureReport, EventCounts structs. Freshness threshold constants.
   All `#[derive(Serialize)]`, zero `serde_json::Value`.

2. `feat(measure): implement freshness, diagnostics, and health derivation` —
   `compute_age_hours()` from timestamps, `Freshness::for_verb()` with per-verb
   thresholds, `diagnostics()` methods on BelieveMetrics/SearchMetrics/capture
   variants (computed from typed fields), `health_summary()` on FullMeasureReport
   (worst-verb-wins). Tests for each derivation.

3. `feat(measure): wire build_full_report and --full CLI flag` — Add `full: bool`
   to MeasureOptions, `build_full_report(conn) -> Result<FullMeasureReport>` that
   reuses existing verb builders then wraps with freshness/diagnostics/health.
   Extract `attach_events()` helper. CLI: `--full --json` serializes FullMeasureReport,
   `--full` without `--json` renders enriched terminal view.

4. `feat(measure): unify MCP and CLI measure paths` — Change `mcp_measure()` to
   call `build_full_report()` and return typed `FullMeasureReport` serialized.
   Delete `serde_json::Value` return type and `json!({})` fallback paths.
   MCP and CLI `--full --json` produce identical output.

5. `feat(measure): add believe grounding breakdown` — Add `contested_count` to
   BelieveMetrics, query `belief_attacks` table for beliefs with unresolved attacks.
   Add diagnostic: "N beliefs have active attacks without resolution".

6. `feat(eval): rewrite execute_feedback with ATTACH cross-db query` — Open
   events.db, ATTACH patina.db. Join scry.query events with git.commit data via
   typed `FeedbackRow` struct at query boundary. Eval emits `measure.search`
   event with P@5 result (eval is the tool, measure is the reader). Remove JSON
   blob parsing from old path.

7. `feat(measure): verify catalog questions and check exit criteria` — Run
   `--full --json`, confirm all 9 catalog questions answerable from output. Check
   all 12 exit criteria in SPEC.md. Update DESIGN.md with final state.

## Key Files

- `src/commands/measure/internal.rs` — all new types, `build_full_report()`, freshness, diagnostics
- `src/commands/measure/mod.rs` — `MeasureOptions` (add `full: bool`), `execute()` routing
- `src/mcp/server/mod.rs` — `handle_measure()` calls `build_full_report()` (commit 4)
- `src/commands/eval/mod.rs` — `execute_feedback()` ATTACH rewrite (commit 6)
- `src/eventlog.rs` — `attach_events()` helper if promoted to shared (commit 3)

## Open Questions

1. ~~**Contested count source**~~ — Resolved. `belief_attacks` exists in `patina.db`
   (same database measure already opens). Query directly:
   `SELECT COUNT(DISTINCT to_belief) FROM belief_attacks WHERE defeated = 0`.
   `beliefs.contested_by` column also available as a pre-materialized field.

2. **--full terminal view**: Spec focuses on JSON contract. The `--full` without
   `--json` terminal rendering is unspecified. Proposal: reuse `render_user_view()`
   structure but add freshness indicators and inline diagnostics. Keep it minimal —
   the LLM path (`--json`) is the primary consumer.

## Measurement Protocol — How Probes Talk to the Dashboard

The emission protocol was designed in sessions [[20260225-143514]], [[20260225-182257]],
[[20260225-191304]], and [[20260225-194001]]. Two entry points, one contract:

**WIT interface** (plugins): `wit/deps/patina-host/host.wit:147`
```wit
interface measure {
    record-measurement: func(
        verb: string,         // capture, index, search, believe, evolve
        tool: string,         // doctor, scrape, eval, oxidize, etc.
        mode: string,         // health-check, code, beliefs, git, etc.
        metrics-json: string, // flat JSON object, numeric values only
    ) -> result<_, string>;
}
```

**Core function** (compiled-in tools): `src/measure.rs`
```rust
pub fn emit(verb: &str, tool: &str, mode: &str, metrics: &serde_json::Value) -> Result<()>
```

Both paths produce identical events in events.db:
```
event_type: "measure.{verb}"
source_id:  "{tool}:{mode}"
data:       { verb, tool, mode, metrics: {key: number, ...}, source: "core"|"plugin:<name>" }
```

The host overrides `source` for plugins — plugins can't impersonate core tools.

### How measure reads probes it wasn't compiled against

The protocol envelope `(verb, tool, mode, metrics_json)` is the standard contract.
Any probe — known or unknown — can emit through it. Measure's `collect_measure_sources()`
queries `WHERE event_type = 'measure.{verb}'` and groups by `tool:mode`.

For **known probes**, measure has typed metric structs (`CaptureHealthCheckMetrics`,
`CaptureCodeMetrics`, `BelieveMetrics`, etc.) dispatched via `VerbMetrics::from_db()`.
These enable rich diagnostics — "23 beliefs floating (14%)" computed from typed fields.

For **unknown probes**, `from_db()` falls back to `VerbMetrics::Raw(serde_json::Value)`.
The probe still appears in the dashboard with flat key-value rendering via `format_kv()`.
It works, it's visible, but it can't generate diagnostics.

### Graduation path (future)

How unknown probes graduate from `Raw` to understood — three options identified:

1. **Manual** (today): add a typed struct to measure, recompile. Works for small
   known probe sets. Doesn't scale to arbitrary plugins.

2. **Schema-declared**: probes register their metrics shape via the fact schema
   system (`patina schemas`). Measure reads the schema at runtime. The
   infrastructure exists but isn't wired to measure yet.

3. **Diagnostic rules**: probes declare threshold rules alongside their schema
   ("if floating_count / total_beliefs > 0.1 → warning"). Measure evaluates
   rules against flat metrics without needing compiled-in knowledge.

For this spec, option 1 (manual typed structs) is sufficient — the probe set is
known and small. Options 2–3 belong in plugin system evolution.

## Doctor / Measure Separation

Doctor is a **probe** (emitter). Measure is a **dashboard** (reader). Data flows
one way: `doctor → events.db → measure`.

Doctor today: runs checks (tools installed, config valid, layer files exist),
emits `measure.capture` with mode `health-check`, AND displays its own summary.
That dual role (probe + display) muddies the separation with measure.

Design direction: doctor is probe-first, display-secondary. Its primary job is
emitting findings into the event stream. Its terminal output is a convenience
for interactive use ("I ran, here's what I found"), not the canonical health view.
Measure is always the dashboard.

Future: doctor runs continuously via plugin tick/schedule lifecycle — Mother
manages when to tick, doctor does the check, events.db stores it, measure reads
it. This is the fetch/scrape/schedule separation from [[persona-is-a-patina-instance]].
More probes emitting = richer dashboard, without measure changing. Freshness
thresholds (this spec) will surface "doctor hasn't run in 72h" automatically.

See: `plugins/doctor/src/lib.rs` (WASM command plugin, first extracted command).

## Data Layout Reference

Measure queries span 2 databases (3 for feedback eval):

```
patina.db (58MB, rebuildable)          events.db (332KB, irreplaceable)
├── beliefs (178 rows)                 ├── eventlog
│   ├── grounding_score                │   ├── measure.* (32 events)
│   ├── health_score                   │   ├── scry.query (21 events)
│   ├── contested_by                   │   ├── session.* (60 events)
│   └── floating = grounding_score=0   │   ├── forge.* (92 events)
├── belief_attacks (defeated=0→active) │   └── assay/context.query
├── eventlog                           └── scrape_meta
│   ├── git.commit (2892)
│   ├── session.ended (675)
│   └── belief.surface (178)
├── commits / commit_files
└── code_search (FTS5)

ATTACH pattern:
  measure opens patina.db, ATTACHes events.db
  feedback eval opens events.db, ATTACHes patina.db (scry.query JOIN git.commit)
```
