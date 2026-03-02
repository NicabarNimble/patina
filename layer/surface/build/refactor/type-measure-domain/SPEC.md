---
type: refactor
id: type-measure-domain
status: ready
created: 2026-03-01
sessions:
  origin: 20260301-165723
related:
- enum-status-types
beliefs:
- parse-at-boundary-type-the-interior
- silent-default-hides-missing-data
- enum-not-string-for-finite-states
exit_criteria:
- id: verb-metrics-enum
  text: VerbMetrics enum defined with 7 variants + Raw fallback
  checked: true
  verify: grep -c 'VerbMetrics' src/commands/measure/internal.rs
- id: zero-value-domain-fields
  text: No serde_json::Value fields in SourceSummary (latest_metrics is VerbMetrics)
  checked: true
  verify: grep 'serde_json::Value' src/commands/measure/internal.rs | grep -v 'Raw\|HistoryEntry\|from_db\|format_metrics'
- id: zero-get-as-chains
  text: No .get().as_*().unwrap_or() chains on metrics in renderers or status decisions
  checked: true
  verify: grep -n '\.get(.*\.as_i64\|\.get(.*\.as_f64' src/commands/measure/internal.rs
- id: parse-at-boundary
  text: collect_measure_sources parses into typed VerbMetrics at DB boundary via from_db()
  checked: true
- id: option-not-default
  text: SearchMetrics uses Option<f64> for p_at_5/mrr/recall_at_5; renderers show "n/a" for None
  checked: true
- id: raw-fallback-logged
  text: VerbMetrics::Raw fallback emits tracing::warn! so new metric shapes are discoverable
  checked: true
- id: raw-fallback-graceful
  text: Synthetic unknown payload triggers Raw fallback with warn! and renders without panic
  checked: true
  verify: Unit test passes a JSON blob with unrecognized keys to from_db() for each verb, asserts Raw variant returned and rendering produces valid output
- id: mcp-payload-diff
  text: MCP payload before/after migration has identical field names and structure (no key renames or field drops)
  checked: true
  verify: Diff patina measure --json output before and after migration — capture baseline before Phase 2
- id: json-shape-preserved
  text: MCP output (mcp_measure) and --json output preserve flat JSON shape via serde(untagged)
  checked: true
  verify: cargo build --release && cargo install --path . && patina measure --json | python3 -c "import sys,json; d=json.load(sys.stdin); [print(v['verb'], list(s['latest_metrics'].keys())) for v in d['verbs'] for s in v['sources']]"
- id: existing-tests-pass
  text: cargo test passes, pre-push checks pass
  checked: true
  verify: ./resources/git/pre-push-checks.sh
---
# refactor: Type the measure domain model

> measure/internal.rs uses serde_json::Value as domain state causing 80+ .get().as_*().unwrap_or() chains — replace with typed structs

## Current State

`src/commands/measure/internal.rs` (1182 lines) has two `serde_json::Value` fields that
propagate untyped data through the entire rendering pipeline:

- `SourceSummary.latest_metrics: serde_json::Value` (line 51) — main domain state
- `HistoryEntry.metrics: serde_json::Value` (line 71) — drill-down history

**Construction sites** — 5 locations build `serde_json::json!({...})` from already-typed SQL
query results, erasing types into Value:
- `build_capture_summary` (line 246) — `{ files_tracked, total_commits }`
- `build_believe_summary` (lines 385-390) — `{ total_beliefs, floating_count, grounded_count, avg_evidence, avg_health }`
- `build_evolve_summary` (lines 483-489) — `{ total_sessions, total_commits, total_files_changed, total_beliefs_captured, total_patterns_modified }`
- `get_believe_history` (lines 1006-1009) — `{ beliefs, floating, avg_evidence }`
- `get_evolve_history` (lines 1039-1043) — `{ commits, files, beliefs, patterns }`

**DB boundary parse sites** — 2 locations deserialize JSON blobs from SQLite, stopping at Value
instead of parsing into typed structs:
- `collect_measure_sources` (lines 562-563) — `serde_json::from_str(&metrics_str).unwrap_or(Value::Null)`
- `get_recent_history` (lines 970-971) — same pattern

**Consumption sites** — ~22 `.get().as_*().unwrap_or()` chains extract data back from Value
in `user_friendly_metrics` (lines 678-777), `build_search_summary` (lines 296-309),
`build_believe_summary` (lines 406-418), `render_system_view` (line 811), and
`format_metrics_inline` (lines 1124-1143).

**String dispatch** — `user_friendly_metrics` dispatches on `(verb, src.source_type.as_str())`
with 7 match arms over string literals. This would be replaced by matching on typed enum variants.

**Silent defaults** — 13 `.unwrap_or(0)` / `.unwrap_or(0.0)` sites on Value chains make
"missing" indistinguishable from "zero" per [[silent-default-hides-missing-data]].

**Already typed (no changes needed):** VerbStatus enum (3 variants), VerbSummary struct,
MeasureReport struct, MeasureOptions struct.

## Target State

`SourceSummary.latest_metrics` is typed as `VerbMetrics` enum with verb-specific struct variants.
All `.get().as_*().unwrap_or()` chains are replaced by direct field access on typed structs.
Parsing happens at the DB boundary via `VerbMetrics::from_db(verb, mode, json_str)` with
`Raw(Value)` fallback that logs a warning. MCP/JSON output shape is preserved via
`#[serde(untagged)]` serialization.

`HistoryEntry.metrics` remains `serde_json::Value` (TODO for follow-up — only rendered
generically via `format_metrics_inline`, no typed access needed).

## Steps

### Phase 0: Capture baseline (no commit)
- Save `patina measure --json` output as baseline for `mcp-payload-diff` exit criterion
- Note current search metric rendering for comparison after Option<f64> migration

### Phase 1: Define types (1 commit)
- Add `VerbMetrics` enum with 7 typed variants + `Raw(Value)` fallback
- Add 7 metric structs: `CaptureCodeMetrics`, `CaptureGitMetrics`, `CaptureGenericMetrics`,
  `IndexMetrics`, `SearchMetrics`, `BelieveMetrics`, `EvolveMetrics`
- Add `VerbMetrics::from_db(verb, mode, json_str)` constructor with manual dispatch
  (avoids `#[serde(untagged)]` deserialization ambiguity — see ADR-1 in DESIGN.md)
- Enum derives `Serialize` (untagged) but NOT `Deserialize` — individual structs derive both
- Raw fallback emits `tracing::warn!`
- Pure additions — no consumers changed, existing code unaffected

### Phase 2: Migrate SourceSummary + all consumers (1 commit)
- Change `SourceSummary.latest_metrics` from `serde_json::Value` to `VerbMetrics`
- Update 5 construction sites: replace `serde_json::json!({...})` with typed struct construction
- Update `collect_measure_sources`: call `VerbMetrics::from_db()` at DB boundary
- Rewrite `user_friendly_metrics`: match on `VerbMetrics` variants instead of `(verb, source_type)` strings
- Update `build_search_summary` status decision: match on `VerbMetrics::Search` variant,
  use `Option<f64>` for p_at_5 (behavior change: None renders as "n/a", not "0%")
- Update `build_believe_summary` floating_pct calculation: match on `VerbMetrics::Believe`
- Update `render_system_view`: use `VerbMetrics` display trait or match for raw dump
- All in `internal.rs` — single-file atomic change

### Phase 3: Clean up + verify (1 commit)
- Verify zero `.get().as_*().unwrap_or()` chains remain on metrics
- Verify JSON output shape matches pre-migration (MCP smoke test)
- Verify `format_metrics_inline` still works for `HistoryEntry` (unchanged)
- Diff `patina measure --json` against Phase 0 baseline to confirm no field drops/renames
- Note: search metrics rendering changes from silent "0%" to explicit "n/a" when absent —
  this is intentional per [[silent-default-hides-missing-data]] and ADR-2
- Run pre-push checks
- Check all exit criteria

## Exit Criteria

See frontmatter `exit_criteria` list (8 criteria).
