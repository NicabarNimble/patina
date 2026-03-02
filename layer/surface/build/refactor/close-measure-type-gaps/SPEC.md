---
type: refactor
id: close-measure-type-gaps
status: draft
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
  checked: false
  verify: grep 'BTreeMap' src/commands/measure/internal.rs | grep -v test
- id: no-serialize-back-to-value
  text: render_system_view uses VerbMetrics::format_kv() instead of serde_json::to_value round-trip
  checked: false
  verify: grep -n 'serde_json::to_value.*latest_metrics' src/commands/measure/internal.rs
- id: verb-param-removed
  text: user_friendly_metrics takes only &SourceSummary, no unused _verb parameter
  checked: false
  verify: grep '_verb' src/commands/measure/internal.rs
- id: history-entry-typed
  text: HistoryEntry.metrics is VerbMetrics, parsed via from_db() at DB boundary
  checked: false
  verify: grep 'metrics.*serde_json::Value' src/commands/measure/internal.rs
- id: history-json-construction-gone
  text: get_believe_history and get_evolve_history construct typed structs, not json!({})
  checked: false
  verify: grep 'serde_json::json!' src/commands/measure/internal.rs
- id: format-metrics-inline-typed
  text: format_metrics_inline operates on VerbMetrics (not Value), or is replaced by VerbMetrics::format_kv()
  checked: false
- id: json-shape-preserved
  text: patina measure --json output identical to v0.35.5 baseline (key names and values)
  checked: false
  verify: diff baseline against current output with sorted keys
- id: existing-tests-pass
  text: cargo test passes, pre-push checks pass
  checked: false
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
- Update `from_db()` dispatch: explicit arms for `("capture", "beliefs")`,
  `("capture", "layer")`, `("capture", "git")`, `("capture", "health-check")`;
  unknown capture modes fall to `Raw`
- Update `user_friendly_metrics`: replace `CaptureGeneric` arm with 4 typed arms using
  direct field access; remove `_verb` parameter from signature and call site
- Compile, test, verify JSON shape

### Phase 2: Add format_kv() + fix render paths (1 commit)

- Add `VerbMetrics::format_kv(&self) -> Vec<(String, String)>` method that matches on
  variants and returns key-value pairs for display
- Update `render_system_view`: replace `serde_json::to_value` round-trip with
  `format_kv()` iteration
- Update `format_metrics_inline` to accept `VerbMetrics` instead of `&serde_json::Value`
  (or replace with `format_kv()`)
- Compile, test

### Phase 3: Type HistoryEntry.metrics (1 commit)

- Change `HistoryEntry.metrics` from `serde_json::Value` to `VerbMetrics`
- Update `get_recent_history`: use `VerbMetrics::from_db()` at DB boundary
- Update `get_believe_history`: construct `VerbMetrics::Believe(BelieveMetrics { ... })`
  — note: history shape differs from summary (fewer fields), so define
  `BelieveHistoryMetrics` or use `Raw` for the reduced shape
- Update `get_evolve_history`: construct `VerbMetrics::Evolve(EvolveMetrics { ... })`
  — same concern: history has `commits/files/beliefs/patterns` not the full
  `total_*` field names. Define `EvolveHistoryMetrics` or rename fields to match
- Update `format_metrics_inline` call sites to pass `&VerbMetrics`
- Compile, test, verify drill-down JSON shape

### Phase 4: Verify + clean up (1 commit)

- Verify zero `serde_json::Value` fields in domain structs (SourceSummary, HistoryEntry)
- Verify zero `serde_json::json!({})` calls in internal.rs
- Verify zero `serde_json::to_value` round-trips for display
- Diff `patina measure --json` against v0.35.5 baseline
- Run pre-push checks
- Check all exit criteria

## Open Questions

1. **History metrics shape divergence.** `get_believe_history` constructs
   `{ beliefs, floating, avg_evidence }` which has different field names than
   `BelieveMetrics { total_beliefs, floating_count, ... }`. Options:
   a. Define separate `BelieveHistoryMetrics` / `EvolveHistoryMetrics` structs
      (adds 2 variants to the enum)
   b. Normalize history field names to match summary (breaking change in drill-down JSON)
   c. Use `Raw` for history entries (defers the problem again)
   Decision: resolve during Phase 3 design review.

2. **SourceSummary string fields.** `source_type`, `tool`, `mode` are finite-valued
   strings that are partially redundant with the VerbMetrics variant. Typing these
   (e.g., `SourceType` enum) would be a further tightening but increases the scope.
   Decision: note as follow-up spec, not in scope here.

## Exit Criteria

See frontmatter `exit_criteria` list (8 criteria).
