# Design: Polish remaining measure type gaps

## Approach

Continue the [[type-measure-domain]] → [[close-measure-type-gaps]] trajectory: replace
remaining stringly-typed fields with enums, clean up minor code smells introduced during
the typed migration. All changes in `src/commands/measure/internal.rs`.

### ADR-1: SourceSummary string fields → enums

**Decision:** Replace `source_type: String`, `tool: String`, `mode: String` with typed
enums. Use `#[serde(rename_all = "kebab-case")]` (or explicit renames) to preserve JSON
output shape.

**Rationale:** These are finite-valued strings with known variants:
- `source_type`: `"measure.capture"`, `"measure.index"`, `"measure.search"`,
  `"measure.believe"`, `"measure.evolve"`, `"git.commit"`, `"beliefs"`, `"session.ended"`
- `tool`: `"scrape"`, `"session"`, `"eval"`, `"oxidize"`
- `mode`: `"code"`, `"beliefs"`, `"layer"`, `"git"`, `"health-check"`, `"lifecycle"`,
  `"default"`, `"eval"`, `"audit"`

Per [[enum-not-string-for-finite-states]], if you can list the variants, it's an enum.

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
#[serde(rename_all = "lowercase")]
pub enum ToolName {
    Scrape,
    Session,
    Eval,
    Oxidize,
}
```

**Open question on Mode:** Mode values span multiple verbs — `"code"` and `"beliefs"` are
capture modes, `"eval"` is a search mode, `"lifecycle"` is an evolve mode. Options:

- **Option A: Single flat `Mode` enum.** Simple, all variants in one place, but mixes
  semantically unrelated modes. A `Mode::Lifecycle` on a capture source would typecheck
  but be nonsensical.
- **Option B: Per-verb mode enums.** `CaptureMode`, `SearchMode`, etc. Precise, but
  `SourceSummary.mode` can't hold them all without a wrapper enum. Adds complexity.
- **Option C: Single enum, document verb affinity.** Pragmatic middle ground — one enum
  with a comment noting which modes belong to which verb. The VerbMetrics variant already
  prevents nonsensical combinations at the semantic level.

**Recommendation:** Option C. The mode enum prevents typos and enables exhaustive matching
without the complexity of per-verb wrappers. VerbMetrics already enforces verb-mode
coherence at the data level.

**Display:** Implement `Display` for all three enums to replace `.to_string()` calls in
render functions. The `Display` output should match the current string values exactly.

### ADR-2: format_kv unit suffix strategy

**Decision:** `format_kv()` returns raw values without unit suffixes. Add a separate
`format_kv_display()` method (or let callers append units) for human-readable output.

**Rationale:** `format_kv()` is used by two consumers with different needs:
- `render_system_view`: wants human-readable display (`1092ms`, `80.0%`)
- `format_metrics_inline`: wants compact `key=value` pairs for history tables — the old
  code emitted `duration_ms=1092` (raw integer), now emits `duration_ms=1092ms` (redundant
  suffix on a field already named `_ms`)

**Implementation:**
```rust
/// Raw key-value pairs — values are plain numbers, no unit suffixes.
pub fn format_kv(&self) -> Vec<(String, String)> { ... }

/// Human-readable key-value pairs with unit suffixes (ms, %).
pub fn format_kv_display(&self) -> Vec<(String, String)> { ... }
```

`render_system_view` calls `format_kv_display()`. `format_metrics_inline` calls
`format_kv()`. This restores the pre-v0.35.6 text output for history tables.

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
    tool: row.get(1)?,
    mode,  // moved, not cloned
    metrics,
})
```

This is a one-line fix — compute `metrics` before constructing the struct, then move
`mode` into the struct instead of cloning.

### ADR-4: History variant from_db routing

**Decision:** Do NOT add `from_db()` arms for `BelieveHistory`/`EvolveHistory`. Instead,
add a doc comment on `from_db()` explaining that history variants are direct-construction
only, and why.

**Rationale:** `from_db()` dispatches on `(verb, mode)` pairs from `measure.*` events.
History entries from `get_believe_history` and `get_evolve_history` come from different
event types (`belief.surface`, `session.ended`) with different data shapes — they never
flow through `from_db()`. Adding arms would be dead code. A doc comment makes the
invariant explicit without adding unreachable match arms.

```rust
/// Parse metrics JSON at the DB boundary, dispatching to the correct typed
/// struct based on verb and mode context from the same DB row.
///
/// History-only variants (BelieveHistory, EvolveHistory) are constructed
/// directly in get_believe_history/get_evolve_history — they originate from
/// belief.surface/session.ended events, not measure.* events.
pub fn from_db(verb: &str, mode: &str, json_str: &str) -> Self { ... }
```

## Commits

1. `type SourceSummary string fields as enums` — SourceType, ToolName, Mode enums with
   serde renames, update all construction and display sites
2. `clean up format_kv, mode clone, and from_db docs` — split format_kv/format_kv_display,
   eliminate mode clone, document history variant routing

## Key Files

- `src/commands/measure/internal.rs` — sole target (same as parent specs)

## Not Touched

- `src/commands/measure/mod.rs` — unchanged
- DB schema — no changes
- JSON output shape — preserved exactly
