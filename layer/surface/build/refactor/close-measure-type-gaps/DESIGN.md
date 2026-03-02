# Design: Close the typed escape hatches in measure domain

## Approach

Continue the [[type-measure-domain]] pattern: replace remaining untyped data with typed
structs at the boundary, make the compiler enforce field access. Each phase closes one
escape hatch without disrupting working code.

### ADR-1: Explicit capture mode structs over BTreeMap catch-all

**Decision:** Replace `CaptureGenericMetrics { fields: BTreeMap<String, Value> }` with 4
typed structs. Unknown capture modes fall to `VerbMetrics::Raw` with `tracing::warn!`.

**Rationale:** CaptureGenericMetrics silently absorbs any JSON — the unit test proved this
when `{"unknown_key": 42}` deserialized as CaptureGeneric instead of Raw. The BTreeMap
is exactly the `Value`-shaped escape hatch that [[value-is-boundary-not-domain]] warns
against. With 4 known capture modes and stable field shapes from the eventlog, there's no
reason to defer typing.

**New structs:**
```rust
pub struct CaptureBeliefsMetrics {
    pub beliefs_processed: i64,
    pub beliefs_verified: i64,
    pub beliefs_skipped: i64,
    pub supports_edges: i64,
    pub attacks_edges: i64,
    pub values_processed: i64,
    pub duration_ms: i64,
}

pub struct CaptureLayerMetrics {
    pub patterns_processed: i64,
    pub sessions_processed: i64,
    pub duration_ms: i64,
}

pub struct CaptureGitScrapeMetrics {
    pub commits_processed: i64,
    pub tracked_files: i64,
    pub tags_indexed: i64,
    pub co_change_pairs: i64,
    pub duration_ms: i64,
}

pub struct CaptureHealthCheckMetrics {
    pub beliefs: i64,
    pub sessions: i64,
    pub layer_patterns: i64,
    pub missing_tools: i64,
    pub new_tools: i64,
}
```

**Updated dispatch:**
```rust
("capture", "code") => CaptureCodeMetrics,
("capture", "beliefs") => CaptureBeliefsMetrics,
("capture", "layer") => CaptureLayerMetrics,
("capture", "git") => CaptureGitScrapeMetrics,
("capture", "health-check") => CaptureHealthCheckMetrics,
("capture", _) => Raw  // unknown modes now warn instead of silently absorbing
```

**Naming:** `CaptureGitScrapeMetrics` (not `CaptureGitMetrics`) because `CaptureGitMetrics`
already exists for the `git.commit` construction path in `build_capture_summary`.

### ADR-2: format_kv() method over serialize-back-to-Value

**Decision:** Add `VerbMetrics::format_kv(&self) -> Vec<(String, String)>` that matches on
variants and yields human-readable key-value pairs. Use this in `render_system_view` and
as a replacement for `format_metrics_inline`.

**Rationale:** `serde_json::to_value(&src.latest_metrics)` followed by `.as_object()` is
a round-trip through the untyped world — exactly the pattern we eliminated from the domain.
The method gives renderers access to field names and formatted values without losing type
information. For Raw fallback, format_kv() serializes the inner Value (acceptable since
Raw data is genuinely untyped).

**Alternative considered:** `impl Display for VerbMetrics`. Rejected because the system view
needs key-value pairs for columnar display, not a single formatted string.

### ADR-3: History entry typing strategy

**Decision:** To be resolved in Phase 3. Two viable approaches:

**Option A — Separate history structs:**
Add `BelieveHistoryMetrics { beliefs, floating, avg_evidence }` and
`EvolveHistoryMetrics { commits, files, beliefs, patterns }` as new VerbMetrics variants.
Pro: preserves current JSON field names in drill-down output. Con: adds 2 more variants.

**Option B — Normalize to match summary structs:**
Change `get_believe_history` to use `BelieveMetrics` field names (total_beliefs instead
of beliefs, floating_count instead of floating). Pro: reuses existing structs, no new
variants. Con: changes drill-down JSON output (breaking for MCP consumers).

**Recommendation:** Option A — history shapes are genuinely different from summaries (fewer
fields, different names), so separate structs are semantically honest. The variant count
increase is acceptable given they're each small (3-4 fields).

### ADR-4: SourceSummary string fields — deferred

**Decision:** Do not type `source_type`, `tool`, `mode` strings in this spec.

**Rationale:** These strings serve display and JSON output. The VerbMetrics variant now
carries the semantic dispatch information, making the strings redundant for control flow
but still needed for rendering. Typing them (e.g., `SourceType` enum) would be correct but
would touch the build_* functions, JSON output shape, and potentially MCP consumers. Better
as a follow-up spec after this one proves the approach.

## Commits

1. `type capture modes and remove vestigial verb parameter` — 4 new structs, remove
   CaptureGenericMetrics, update from_db dispatch, rewrite user_friendly_metrics arms,
   drop _verb param.

2. `add format_kv and eliminate serialize-back-to-Value` — VerbMetrics::format_kv(),
   update render_system_view, update/replace format_metrics_inline.

3. `type HistoryEntry.metrics via from_db at boundary` — HistoryEntry.metrics becomes
   VerbMetrics, history builders construct typed structs, format_metrics_inline uses
   format_kv().

4. `verify zero Value escape hatches and check exit criteria` — verify, diff, clean up,
   pre-push.

## Key Files

- `src/commands/measure/internal.rs` — sole target (same as parent spec)

## Not Touched

- `src/commands/measure/mod.rs` — unchanged
- DB schema — no changes
- `SourceSummary.source_type/tool/mode` — deferred per ADR-4
