# Design: Polish remaining measure type gaps

## Approach

Continue the [[type-measure-domain]] → [[close-measure-type-gaps]] trajectory: replace
remaining stringly-typed fields with enums, clean up minor code smells introduced during
the typed migration. All changes in `src/commands/measure/internal.rs`.

### ADR-1: SourceSummary + HistoryEntry string fields → enums

**Decision:** Replace `source_type: String`, `tool: String`, `mode: String` in both
`SourceSummary` and `HistoryEntry` with typed enums. Use explicit `#[serde(rename)]`
per variant — NOT `rename_all` — because strings contain dots (`"measure.capture"`)
and hyphens (`"health-check"`) that no single rename strategy handles.

**Rationale:** These are finite-valued strings with known variants:
- `source_type`: `"measure.capture"`, `"measure.index"`, `"measure.search"`,
  `"measure.believe"`, `"measure.evolve"`, `"git.commit"`, `"beliefs"`, `"session.ended"`
- `tool`: `"scrape"`, `"session"`, `"eval"`, `"oxidize"`
- `mode`: `"code"`, `"beliefs"`, `"layer"`, `"git"`, `"health-check"`, `"lifecycle"`,
  `"default"`, `"eval"`, `"audit"`

Per [[enum-not-string-for-finite-states]], if you can list the variants, it's an enum.
Both `SourceSummary` and `HistoryEntry` share the same `tool`/`mode` value spaces —
typing one and leaving the other as `String` would be inconsistent.

**Enum design:**
```rust
#[derive(Debug, Clone, Serialize)]
pub enum SourceType {
    #[serde(rename = "measure.capture")]
    MeasureCapture,
    #[serde(rename = "measure.index")]
    MeasureIndex,
    #[serde(rename = "measure.search")]
    MeasureSearch,
    #[serde(rename = "measure.believe")]
    MeasureBelieve,
    #[serde(rename = "measure.evolve")]
    MeasureEvolve,
    #[serde(rename = "git.commit")]
    GitCommit,
    #[serde(rename = "beliefs")]
    Beliefs,
    #[serde(rename = "session.ended")]
    SessionEnded,
}

#[derive(Debug, Clone, Serialize)]
pub enum ToolName {
    #[serde(rename = "scrape")]
    Scrape,
    #[serde(rename = "session")]
    Session,
    #[serde(rename = "eval")]
    Eval,
    #[serde(rename = "oxidize")]
    Oxidize,
}

/// Single flat mode enum (Option C). VerbMetrics variant already enforces
/// verb-mode coherence — this enum prevents typos and enables exhaustive matching.
#[derive(Debug, Clone, Serialize)]
pub enum Mode {
    // Capture modes
    #[serde(rename = "code")]
    Code,
    #[serde(rename = "beliefs")]
    Beliefs,
    #[serde(rename = "layer")]
    Layer,
    #[serde(rename = "git")]
    Git,
    #[serde(rename = "health-check")]
    HealthCheck,
    // Search modes
    #[serde(rename = "eval")]
    Eval,
    #[serde(rename = "audit")]
    Audit,
    // Evolve modes
    #[serde(rename = "lifecycle")]
    Lifecycle,
    // Generic
    #[serde(rename = "default")]
    Default,
}
```

**Forward compatibility:** These enums are construction-only — we build them in Rust,
never deserialize from external input. No `#[serde(other)]` or `Unknown(String)` needed.
New DB modes that appear via `collect_measure_sources` would need to be added to `Mode`
(compiler error at the construction site), which is the correct forcing function. The
`from_db()` `Raw` fallback handles unknown metric shapes; the enum handles unknown mode
names at compile time.

**Display:** Implement `Display` for all three enums. Output matches current string
values exactly (e.g., `Mode::HealthCheck` displays as `"health-check"`).

### ADR-2: format_kv unit suffix strategy

**Decision:** Split `format_kv()` into raw and display variants.

**Rationale:** `format_kv()` has 4 consumers with two different needs:

| Call site | Line | Context | Needs |
|-----------|------|---------|-------|
| `render_system_view` | 1101 | System view metric table | Human-readable (`1092ms`, `80.0%`) |
| `format_metrics_inline` (via `render_system_view`) | 1118 | System view history rows | Raw (`duration_ms=1092`) |
| `format_metrics_inline` (via `run_verb_drilldown`) | 1192 | Drill-down measure events | Raw (`duration_ms=1092`) |
| `format_metrics_inline` (via `run_verb_drilldown`) | 1219 | Drill-down existing events | Raw (`duration_ms=1092`) |

**Implementation:**
```rust
/// Raw key-value pairs — values are plain numbers, no unit suffixes.
/// Used by format_metrics_inline for compact history table output.
pub fn format_kv(&self) -> Vec<(String, String)> { ... }

/// Human-readable key-value pairs with unit suffixes (ms, %).
/// Used by render_system_view for the current-state metric display.
pub fn format_kv_display(&self) -> Vec<(String, String)> { ... }
```

`render_system_view` (line 1101) calls `format_kv_display()`. `format_metrics_inline`
(line 1416) calls `format_kv()`. This restores the pre-v0.35.6 text output for history
tables while keeping the system view human-readable.

**Implementation note:** `format_kv_display()` can be implemented as a thin wrapper that
calls `format_kv()` and applies unit suffixes based on key name conventions (`*_ms` →
append `ms`, known rate fields → append `%`). This avoids duplicating the per-variant
match logic.

### ADR-3: HistoryEntry.mode clone elimination

**Decision:** Reorder struct construction so `mode` is consumed by the struct field after
`from_db()` borrows it.

**Implementation:**
```rust
let mode: String = row.get(2)?;
let metrics_str: String = row.get(3)?;
let metrics = VerbMetrics::from_db(verb, &mode, &metrics_str);
Ok(HistoryEntry {
    timestamp: row.get(0)?,
    tool: ToolName::from_str(&row.get::<_, String>(1)?),
    mode: Mode::from_str(&mode),
    metrics,
})
```

Compute `metrics` before constructing the struct, then move `mode` into the enum
conversion. With the Mode enum from ADR-1, the string is consumed by `Mode::from_str()`
— no clone needed.

### ADR-4: History variant from_db routing — doc + regression test

**Decision:** Do NOT add `from_db()` arms for `BelieveHistory`/`EvolveHistory`. Add a
doc comment explaining the invariant AND a regression test proving it.

**Rationale:** `from_db()` dispatches on `(verb, mode)` pairs from `measure.*` events.
History entries from `get_believe_history` and `get_evolve_history` come from different
event types (`belief.surface`, `session.ended`) with different data shapes — they never
flow through `from_db()`. Adding arms would be dead code.

**Doc comment:**
```rust
/// Parse metrics JSON at the DB boundary, dispatching to the correct typed
/// struct based on verb and mode context from the same DB row.
///
/// History-only variants (BelieveHistory, EvolveHistory) are constructed
/// directly in get_believe_history/get_evolve_history — they originate from
/// belief.surface/session.ended events, not measure.* events. The dispatch
/// paths are disjoint: from_db("believe", "beliefs", ...) → Believe (summary),
/// while get_believe_history constructs BelieveHistory directly from SQL columns.
pub fn from_db(verb: &str, mode: &str, json_str: &str) -> Self { ... }
```

**Regression test:**
```rust
#[test]
fn from_db_believe_returns_summary_not_history() {
    // Proves dispatch paths are disjoint: from_db returns Believe (summary),
    // not BelieveHistory (which is direct-construction only).
    let json = r#"{"total_beliefs": 178, "floating_count": 5, "grounded_count": 173, "avg_evidence": 1.72, "avg_health": 0.88}"#;
    let result = VerbMetrics::from_db("believe", "beliefs", json);
    assert!(matches!(result, VerbMetrics::Believe(_)));
}
```

This test will fail if someone accidentally routes believe through a history variant,
catching the bug at CI time rather than in production.

## Commits

1. `type SourceSummary and HistoryEntry string fields as enums` — SourceType, ToolName,
   Mode enums with serde renames + Display impls, update all construction and display
   sites in both structs
2. `clean up format_kv, mode clone, and from_db docs` — split format_kv/format_kv_display,
   eliminate mode clone, add from_db doc comment + regression test

## Key Files

- `src/commands/measure/internal.rs` — sole target (same as parent specs)

## Not Touched

- `src/commands/measure/mod.rs` — unchanged
- DB schema — no changes
- JSON output shape — preserved exactly (verified via golden baseline diffs)
