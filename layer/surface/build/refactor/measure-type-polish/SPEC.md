---
type: refactor
id: measure-type-polish
status: draft
created: 2026-03-02
sessions:
  origin: 20260301-202410
related:
- close-measure-type-gaps
- type-measure-domain
beliefs:
- value-is-boundary-not-domain
- parse-at-boundary-type-the-interior
- enum-not-string-for-finite-states
exit_criteria:
- id: source-summary-strings-typed
  text: SourceSummary.source_type, tool, mode replaced with enums — no bare String for finite-valued fields in SourceSummary or HistoryEntry
  checked: false
- id: history-mode-no-clone
  text: HistoryEntry construction avoids redundant mode clone
  checked: false
- id: format-kv-unit-suffix-clean
  text: format_kv returns raw integers for all fields; format_kv_display adds unit suffixes. render_system_view calls display, format_metrics_inline calls raw.
  checked: false
  verify: patina measure --system 2>&1 | grep 'duration_ms'
- id: from-db-history-documented
  text: from_db() doc comment explains history variants are direct-construction only; regression test asserts history verb+mode pairs do not reach from_db in normal operation
  checked: false
- id: json-shape-preserved
  text: patina measure --json output identical to pre-change baseline (captured to /tmp/measure-baseline-polish.json before Phase 1)
  checked: false
  verify: diff <(python3 -c "import json,sys; json.dump(json.load(open('/tmp/measure-baseline-polish.json')),sys.stdout,sort_keys=True,indent=2)") <(patina measure --json | python3 -c "import json,sys; json.dump(json.load(sys.stdin),sys.stdout,sort_keys=True,indent=2)")
- id: drilldown-json-preserved
  text: patina measure --verb believe/evolve --json output identical to pre-change baselines
  checked: false
  verify: diff <(python3 -c "import json,sys; json.dump(json.load(open('/tmp/drilldown-believe-baseline-polish.json')),sys.stdout,sort_keys=True,indent=2)") <(patina measure --verb believe --json | python3 -c "import json,sys; json.dump(json.load(sys.stdin),sys.stdout,sort_keys=True,indent=2)")
- id: existing-tests-pass
  text: cargo test passes, pre-push checks pass
  checked: false
  verify: ./resources/git/pre-push-checks.sh
---
# refactor: Polish remaining measure type gaps

> 4 outliers from [[close-measure-type-gaps]]: SourceSummary string fields,
> HistoryEntry.mode clone, format_kv unit suffix in text output, and missing
> from_db documentation for history variants.

## Current State

After [[close-measure-type-gaps]] (v0.35.6), `src/commands/measure/internal.rs` has
zero `serde_json::Value` in domain structs but 4 minor loose ends remain:

**1. SourceSummary and HistoryEntry string fields (ADR-4 deferred)**
```rust
pub struct SourceSummary {
    pub source_type: String, // finite: "measure.*", "git.commit", "beliefs", "session.ended"
    pub tool: String,        // finite: "scrape", "session", "eval", "oxidize"
    pub mode: String,        // finite: "code", "beliefs", "layer", "git", "health-check", "lifecycle", ...
    ...
}
struct HistoryEntry {
    tool: String,            // same finite set as SourceSummary.tool
    mode: String,            // same finite set as SourceSummary.mode
    ...
}
```
These are finite-valued strings that [[enum-not-string-for-finite-states]] says should
be enums. The VerbMetrics variant now carries semantic dispatch, making these redundant
for control flow but still needed for display and JSON output. Both structs share the
same `tool` and `mode` value spaces — typing one and leaving the other creates an
inconsistency.

**2. HistoryEntry.mode clone (line ~1265)**
```rust
let mode: String = row.get(2)?;
Ok(HistoryEntry {
    ...
    mode: mode.clone(),
    metrics: VerbMetrics::from_db(verb, &mode, &metrics_str),
})
```
`mode` is read from DB, used for `from_db()` dispatch, then cloned into the struct.
Minor but avoidable — reorder to consume after dispatch.

**3. format_kv unit suffix inconsistency**
`format_kv()` appends `ms` suffix to duration fields (e.g., `duration_ms=1092ms`).
This affects 4 call sites:
- `render_system_view` (line 1101): direct `format_kv()` — wants display formatting
- `format_metrics_inline` (lines 1118, 1192, 1219): calls `format_kv()` internally
  — used in `--system` history tables and `--verb` drill-down tables, where the old
  code emitted `duration_ms=1092` (raw integer)

The JSON output is unaffected (serde path), but text output changed in v0.35.6.
No known MCP consumers parse text output, but the redundant `ms` suffix on a field
already named `_ms` is sloppy.

**4. No from_db() documentation for history variants**
`BelieveHistory` and `EvolveHistory` are constructed directly in `get_believe_history`
and `get_evolve_history` — never routed through `from_db()`. This is correct (they
come from `belief.surface`/`session.ended` events, not `measure.*` events), but
undocumented. A future contributor adding a new history source might not know whether
to route through `from_db()` or construct directly.

## Target State

- `SourceSummary` and `HistoryEntry` use `SourceType`, `ToolName`, `Mode` enums
  with `Display` impls for rendering and explicit `#[serde(rename = "...")]` for
  JSON output (not `rename_all` — strings contain dots and hyphens)
- `HistoryEntry` construction avoids the mode clone
- `format_kv()` returns raw values; `format_kv_display()` returns unit-annotated
  values. Callers choose which to use.
- `from_db()` has a doc comment explaining history variant routing, plus a regression
  test proving the invariant

## Steps

### Phase 1: Type SourceSummary and HistoryEntry string fields (1 commit)

Before any code changes, capture baselines:
```bash
cargo build --release && cargo install --path .
patina measure --json > /tmp/measure-baseline-polish.json
patina measure --verb believe --json > /tmp/drilldown-believe-baseline-polish.json
patina measure --verb evolve --json > /tmp/drilldown-evolve-baseline-polish.json
```

- Define `SourceType` enum with explicit `#[serde(rename = "...")]` per variant
  (not `rename_all` — strings like `"measure.capture"` and `"session.ended"` contain
  dots). Derive `Serialize` + `Display`.
- Define `ToolName` enum with `#[serde(rename_all = "lowercase")]` (all tool names
  are simple lowercase). Derive `Serialize` + `Display`.
- Define `Mode` enum (Option C — single flat enum) with explicit `#[serde(rename)]`
  for hyphenated modes (`"health-check"`), `rename_all = "lowercase"` for the rest.
  Add doc comments noting verb affinity per variant. Derive `Serialize` + `Display`.
- **Forward compatibility:** Do NOT add `#[serde(other)]` or `Unknown(String)`.
  These enums are construction-only (we build them in Rust, never deserialize from
  external input). If a new mode appears in the DB, it hits `from_db()`'s `Raw`
  fallback with a warning — the enum is only constructed for known-good paths.
- Update `SourceSummary`: `source_type: SourceType`, `tool: ToolName`, `mode: Mode`
- Update `HistoryEntry`: `tool: ToolName`, `mode: Mode`
- Update all construction sites in `build_*_summary`, `collect_measure_sources`,
  `get_recent_history`, `get_believe_history`, `get_evolve_history`
- Update display sites: `render_system_view` (line 1098), `render_user_view` (line
  939 `src.mode`), drill-down table headers (lines 1178, 1219) — use `Display` impl
- Diff JSON against baselines

### Phase 2: Clean up format_kv, mode clone, and from_db docs (1 commit)

- Split `format_kv()` into two methods:
  - `format_kv(&self) -> Vec<(String, String)>` — raw values, no unit suffixes
  - `format_kv_display(&self) -> Vec<(String, String)>` — human-readable with
    `ms` suffix for durations, `%` for rates
- Update call sites:
  - `render_system_view` (line 1101): call `format_kv_display()`
  - `format_metrics_inline` (line 1416): call `format_kv()` (raw)
- Fix `get_recent_history` HistoryEntry construction: compute `metrics` before
  struct literal, move `mode` instead of cloning
- Add doc comment to `from_db()` explaining history variant routing
- Add regression test: construct a `from_db("believe", "beliefs", ...)` call and
  assert it returns `VerbMetrics::Believe` (not `BelieveHistory`) — proves the
  summary/history dispatch paths are disjoint
- Diff JSON + text output against baselines

## Resolved Questions

1. ~~**Enum granularity for mode.**~~ Resolved: Option C — single flat `Mode` enum.
   VerbMetrics variant already enforces verb-mode coherence at the data level. Doc
   comments note verb affinity per variant.

2. ~~**Forward compatibility for new modes.**~~ Resolved: No `Unknown(String)` variant.
   These enums are never deserialized from external input — they're constructed in
   Rust code for known paths. New DB modes hit `from_db()`'s `Raw` fallback.

3. ~~**HistoryEntry scope.**~~ Resolved: `HistoryEntry.tool` and `HistoryEntry.mode`
   share the same value space as `SourceSummary` — they're typed with the same enums.
   Leaving them as `String` while typing `SourceSummary` would be inconsistent.

4. ~~**JSON parity verification.**~~ Resolved: capture golden baselines before Phase 1,
   diff with `python3 json.dump(sort_keys=True)` in exit criteria verify commands.
   No hand-wavy signoffs.

5. ~~**from_db history arm evidence.**~~ Resolved: regression test asserting that
   `from_db("believe", "beliefs", ...)` returns `Believe` (not `BelieveHistory`)
   proves the dispatch paths are disjoint. Doc comment explains the invariant.

## Exit Criteria

See frontmatter `exit_criteria` list (7 criteria).
