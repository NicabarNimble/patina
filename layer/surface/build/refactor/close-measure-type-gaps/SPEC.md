---
type: refactor
id: close-measure-type-gaps
status: active
created: 2026-03-02
sessions:
  origin: 20260301-194301
related:
- type-measure-domain
beliefs:
- parse-at-boundary-type-the-interior
- value-is-boundary-not-domain
- silent-default-hides-missing-data
exit_criteria:
- id: capture-modes-typed
  text: 4 new capture structs replace CaptureGenericMetrics — no BTreeMap<String, Value> in VerbMetrics
  checked: true
  verify: grep 'BTreeMap' src/commands/measure/internal.rs | grep -v test
- id: no-serialize-back-to-value
  text: render_system_view uses VerbMetrics::format_kv() instead of serde_json::to_value round-trip
  checked: true
  verify: grep -n 'serde_json::to_value.*latest_metrics' src/commands/measure/internal.rs
- id: verb-param-removed
  text: user_friendly_metrics takes only &SourceSummary, no unused _verb parameter
  checked: true
  verify: grep '_verb' src/commands/measure/internal.rs
- id: history-entry-typed
  text: HistoryEntry.metrics is VerbMetrics, parsed via from_db() at DB boundary
  checked: true
  verify: grep 'metrics.*serde_json::Value' src/commands/measure/internal.rs
- id: history-json-construction-gone
  text: get_believe_history and get_evolve_history construct typed structs, not json!({})
  checked: true
  verify: grep 'serde_json::json!' src/commands/measure/internal.rs
- id: format-metrics-inline-typed
  text: format_metrics_inline operates on VerbMetrics (not Value), or is replaced by VerbMetrics::format_kv()
  checked: true
- id: drilldown-json-preserved
  text: patina measure --verb believe/evolve --json drill-down output preserves history field names
  checked: true
  verify: diff drill-down baseline against current output with sorted keys
- id: json-shape-preserved
  text: patina measure --json output identical to v0.35.5 baseline (key names and values)
  checked: true
  verify: diff baseline against current output with sorted keys
- id: live-capture-modes-exercised
  text: patina measure --json exercises all typed capture modes (no Raw fallback in normal operation)
  checked: true
  verify: patina measure --json | python3 -c "import sys,json; d=json.load(sys.stdin); [print(s['mode']) for v in d['verbs'] if v['verb']=='capture' for s in v['sources']]"
- id: existing-tests-pass
  text: cargo test passes, pre-push checks pass
  checked: true
  verify: ./resources/git/pre-push-checks.sh
---
# refactor: Close the typed escape hatches in measure domain

> [[type-measure-domain]] (v0.35.5) typed 6 of 7 VerbMetrics variants but left escape
> hatches: a BTreeMap junk drawer, a serialize-back-to-Value render path, untyped history
> entries, and a vestigial string parameter. Close them.

## Current State

After [[type-measure-domain]], `src/commands/measure/internal.rs` has 4 remaining type gaps:

**1. CaptureGenericMetrics is a typed hole (lines 148-152)**
```rust
pub struct CaptureGenericMetrics {
    #[serde(flatten)]
    pub fields: BTreeMap<String, serde_json::Value>,
}
```
Accepts ANY JSON object. The `user_friendly_metrics` arm for this variant still does
`.as_i64().map(|n| n > 0).unwrap_or(false)` — the exact `.get().as_*().unwrap_or()`
pattern the parent spec eliminated everywhere else. 4 known capture modes use this:
- `beliefs` — 7 fields: attacks_edges, beliefs_processed, beliefs_skipped, beliefs_verified,
  duration_ms, supports_edges, values_processed
- `layer` — 3 fields: duration_ms, patterns_processed, sessions_processed
- `git` (scrape) — 5 fields: co_change_pairs, commits_processed, duration_ms, tags_indexed,
  tracked_files
- `health-check` — 5 fields: beliefs, layer_patterns, missing_tools, new_tools, sessions

**2. render_system_view serializes back to Value (lines 901-906)**
```rust
if let Ok(val) = serde_json::to_value(&src.latest_metrics) {
    if let Some(obj) = val.as_object() {
```
We worked to get out of Value, then round-trip back into it for generic key-value display.

**3. HistoryEntry.metrics remains serde_json::Value (line 72)**
Deferred in ADR-3 of [[type-measure-domain]]. Two construction sites (`get_believe_history`,
`get_evolve_history`) use `serde_json::json!({})` to build Value from typed SQL columns.
`get_recent_history` deserializes JSON blobs to Value. `format_metrics_inline` iterates
Value generically. The same `from_db()` parser can be reused.

**4. Vestigial `_verb` parameter (line 791)**
```rust
fn user_friendly_metrics(_verb: &str, src: &SourceSummary) -> String {
```
The leading underscore says "I used to need verb to dispatch, but the VerbMetrics variant
carries that information now." The parameter should be removed from the function and its
call site.

## Target State

- `CaptureGenericMetrics` replaced by 4 typed structs: `CaptureBeliefsMetrics`,
  `CaptureLayerMetrics`, `CaptureGitScrapeMetrics`, `CaptureHealthCheckMetrics`
- `from_db()` dispatch has explicit arms for all known capture modes; unknown modes fall
  to `Raw` with `tracing::warn!` (no more silent BTreeMap absorption)
- `VerbMetrics` has a `format_kv()` method that returns `Vec<(String, String)>` for generic
  key-value display, replacing the serialize-back-to-Value pattern in `render_system_view`
  and the Value iteration in `format_metrics_inline`
- `HistoryEntry.metrics` is `VerbMetrics`, parsed via `from_db()` at the DB boundary;
  `get_believe_history` and `get_evolve_history` construct typed structs directly
- `user_friendly_metrics` signature is `fn user_friendly_metrics(src: &SourceSummary) -> String`
- JSON output shape is identical to v0.35.5

## Steps

### Phase 1: Type capture modes + remove vestigial param (1 commit)

- Define 4 new structs: `CaptureBeliefsMetrics`, `CaptureLayerMetrics`,
  `CaptureGitScrapeMetrics`, `CaptureHealthCheckMetrics`
- Add 4 new VerbMetrics variants: `CaptureBeliefs`, `CaptureLayer`, `CaptureGitScrape`,
  `CaptureHealthCheck`
- Remove `CaptureGenericMetrics` struct and `CaptureGeneric` variant
- Remove `BTreeMap` import if no longer used
- Update `from_db()` dispatch: explicit arms for `("capture", "beliefs")`,
  `("capture", "layer")`, `("capture", "git")`, `("capture", "health-check")`;
  unknown capture modes fall to `Raw`. Note: `CaptureGitMetrics` (for `git.commit`
  direct construction) is never routed through `from_db()` — the `("capture", "git")`
  arm routes to `CaptureGitScrapeMetrics` for `measure.capture` events only
- Update `user_friendly_metrics`: replace `CaptureGeneric` arm with 4 typed arms using
  direct field access; remove `_verb` parameter from signature and call site (line 773
  in `render_user_view`)
- Update unit tests: replace `CaptureGeneric` assertions with new variant names; add
  test that each known capture mode deserializes to its typed variant (not Raw)
- Compile, test, verify JSON shape

### Phase 2: Add format_kv() + fix render paths (1 commit)

- Add `VerbMetrics::format_kv(&self) -> Vec<(String, String)>` method that matches on
  variants and returns key-value pairs for display. Standardize units in formatted
  values: `ms` suffix for durations, `%` for rates, integer formatting for counts
- Update `render_system_view`: replace `serde_json::to_value` round-trip with
  `format_kv()` iteration
- Update `format_metrics_inline` to accept `VerbMetrics` instead of `&serde_json::Value`
  (or replace with `format_kv()` + join)
- Compile, test

### Phase 3: Type HistoryEntry.metrics (1 commit)

- Capture drill-down baselines before changes:
  `patina measure --verb believe --json > /tmp/drilldown-believe-baseline.json`
  `patina measure --verb evolve --json > /tmp/drilldown-evolve-baseline.json`
- Change `HistoryEntry.metrics` from `serde_json::Value` to `VerbMetrics`
- Define `BelieveHistoryMetrics { beliefs, floating, avg_evidence }` and
  `EvolveHistoryMetrics { commits, files, beliefs, patterns }` as new VerbMetrics
  variants (Option A from ADR-3 — history shapes are genuinely different)
- Update `get_recent_history`: use `VerbMetrics::from_db()` at DB boundary
- Update `get_believe_history`: construct `VerbMetrics::BelieveHistory(...)` directly
- Update `get_evolve_history`: construct `VerbMetrics::EvolveHistory(...)` directly
- Update `format_metrics_inline` call sites to pass `&VerbMetrics` (or use format_kv())
- Verify: history-only variants use `#[serde(untagged)]` serialization correctly —
  no field overlap with summary variants (different field names guarantees this)
- Diff drill-down JSON against baselines to prove MCP drill-down shape preserved
- Compile, test

### Phase 4: Verify + clean up (1 commit)

- Verify zero `serde_json::Value` fields in domain structs (SourceSummary, HistoryEntry)
- Verify zero `serde_json::json!({})` calls in internal.rs
- Verify zero `serde_json::to_value` round-trips for display
- Diff `patina measure --json` against v0.35.5 baseline
- Run pre-push checks
- Check all exit criteria

## Open Questions

1. ~~**History metrics shape divergence.**~~ Resolved: Option A — separate
   `BelieveHistoryMetrics` / `EvolveHistoryMetrics` structs. See ADR-3 in DESIGN.md.

2. **SourceSummary string fields.** `source_type`, `tool`, `mode` are finite-valued
   strings that are partially redundant with the VerbMetrics variant. Typing these
   (e.g., `SourceType` enum) would be a further tightening but increases the scope.
   Decision: note as follow-up spec, not in scope here.

## Exit Criteria

See frontmatter `exit_criteria` list (8 criteria).
