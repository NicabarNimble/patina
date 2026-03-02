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
  text: SourceSummary.source_type, tool, mode replaced with enums — no bare String for finite-valued fields
  checked: false
- id: history-mode-no-clone
  text: HistoryEntry construction avoids redundant mode clone
  checked: false
- id: format-kv-unit-suffix-clean
  text: format_kv duration fields emit raw integers (no ms suffix) — unit annotation handled by callers or separate method
  checked: false
- id: from-db-history-arms
  text: from_db() has explicit arms for believe/evolve history shapes, or history construction is provably unreachable via from_db
  checked: false
- id: json-shape-preserved
  text: patina measure --json output identical to pre-change baseline
  checked: false
- id: drilldown-json-preserved
  text: patina measure --verb believe/evolve --json drill-down output identical to pre-change baseline
  checked: false
- id: existing-tests-pass
  text: cargo test passes, pre-push checks pass
  checked: false
---
# refactor: Polish remaining measure type gaps

> 4 outliers from [[close-measure-type-gaps]]: SourceSummary string fields,
> HistoryEntry.mode clone, format_kv unit suffix in text output, and missing
> from_db arms for history variants.

## Current State

After [[close-measure-type-gaps]] (v0.35.6), `src/commands/measure/internal.rs` has
zero `serde_json::Value` in domain structs but 4 minor loose ends remain:

**1. SourceSummary string fields (ADR-4 deferred)**
```rust
pub struct SourceSummary {
    pub source_type: String, // finite: "measure.*", "git.commit", "beliefs", "session.ended"
    pub tool: String,        // finite: "scrape", "session", "eval", "oxidize"
    pub mode: String,        // finite: "code", "beliefs", "layer", "git", "health-check", "lifecycle", ...
    ...
}
```
These are finite-valued strings that [[enum-not-string-for-finite-states]] says should
be enums. The VerbMetrics variant now carries semantic dispatch, making these redundant
for control flow but still needed for display and JSON output.

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
This is fine for `render_system_view` but `format_metrics_inline` (used in history
tables) also flows through `format_kv()` now, producing `duration_ms=1092ms` where
the old code produced `duration_ms=1092`. The JSON output is unaffected (serde path),
but text output changed. If any MCP consumer parses text output, they'd notice.

**4. No from_db() arms for history variants**
`BelieveHistory` and `EvolveHistory` are constructed directly in `get_believe_history`
and `get_evolve_history` — never routed through `from_db()`. If a new history source
goes through the DB boundary, it would silently fall to `Raw`. The compiler won't
catch this since `from_db()` has a catch-all.

## Target State

- `SourceSummary` uses `SourceType`, `ToolName`, `ModeName` enums (or similar)
  with `Display` impls for rendering and `Serialize` for JSON output
- `HistoryEntry` construction avoids the mode clone
- `format_kv()` returns raw values; unit formatting is caller responsibility
  or a separate `format_kv_display()` method
- History variant construction is either routed through `from_db()` or documented
  as intentionally direct-only with a comment explaining why

## Steps

### Phase 1: Type SourceSummary string fields

- Define enums for `source_type`, `tool`, `mode` with serde rename for JSON compat
- Update all construction sites in `build_*_summary` functions
- Update display sites (render_user_view, render_system_view, drill-down)
- Verify JSON output unchanged

### Phase 2: Clean up minor issues

- Fix HistoryEntry.mode clone by reordering construction
- Split format_kv into raw values and display-formatted variants
- Document history variant from_db routing (or add arms)
- Verify JSON + text output

## Open Questions

1. **Enum granularity for mode.** `mode` has ~8 known values across verbs. Should this
   be a single `Mode` enum or per-verb mode enums? Single is simpler but mixes
   capture modes with search modes.

## Exit Criteria

See frontmatter `exit_criteria` list (7 criteria).
