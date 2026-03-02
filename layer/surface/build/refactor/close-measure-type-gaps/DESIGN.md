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

**Mode routing safety:** `CaptureGitMetrics` is only used for direct construction in
`build_capture_summary` (the `git.commit` event source) — it never enters `from_db()`.
The `("capture", "git")` arm in `from_db()` routes exclusively to `CaptureGitScrapeMetrics`
for `measure.capture` events. These two paths cannot cross-wire because `git.commit` sources
are built inline with known field values, while `measure.capture` events flow through
`collect_measure_sources` → `from_db()`. A unit test should exercise both paths to make
this invariant explicit.

### ADR-2: format_kv() method over serialize-back-to-Value

**Decision:** Add `VerbMetrics::format_kv(&self) -> Vec<(String, String)>` that matches on
variants and yields human-readable key-value pairs. Use this in `render_system_view` and
as a replacement for `format_metrics_inline`.

**Rationale:** `serde_json::to_value(&src.latest_metrics)` followed by `.as_object()` is
a round-trip through the untyped world — exactly the pattern we eliminated from the domain.
The method gives renderers access to field names and formatted values without losing type
information. For Raw fallback, format_kv() serializes the inner Value (acceptable since
Raw data is genuinely untyped).

**Unit standardization:** `format_kv()` should apply consistent formatting to values:
durations get `ms` suffix, rates get `%`, counts are plain integers. This keeps system
view and history output visually consistent now that both flow through the same helper.

**Alternative considered:** `impl Display for VerbMetrics`. Rejected because the system view
needs key-value pairs for columnar display, not a single formatted string.

### ADR-3: History entry typing — separate structs (Option A)

**Decision:** Add `BelieveHistoryMetrics { beliefs, floating, avg_evidence }` and
`EvolveHistoryMetrics { commits, files, beliefs, patterns }` as new VerbMetrics variants.

**Rationale:** History shapes are genuinely different from summary shapes — fewer fields,
different names (`beliefs` vs `total_beliefs`, `floating` vs `floating_count`). Reusing
the summary structs would require either renaming fields (breaking drill-down JSON for
MCP consumers) or making all fields Optional (losing the "this shape is complete" signal).
Separate small structs are semantically honest.

**Confirmed baseline shapes (from live data):**
- believe drill-down: `{ beliefs: i64, floating: i64, avg_evidence: f64 }`
- evolve drill-down: `{ commits: i64, files: i64, beliefs: i64, patterns: i64 }`

**Serde note:** History-only variants participate in `#[serde(untagged)]` serialization
on VerbMetrics. Since field names are disjoint from summary variants (`beliefs` vs
`total_beliefs`, `commits` vs `total_commits`), there is no serialization overlap.
Deserialization remains manual via `from_db()`, so no ambiguity. The history structs
derive both `Serialize` and `Deserialize` like all other metric structs.

**Alternative rejected:** Option B (normalize field names) — would break `patina measure
--verb believe --json` output consumed by MCP clients.

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
