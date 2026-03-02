# Design: Type the measure domain model

## Approach

Follow the [[enum-status-types]] pattern: define typed enums/structs, then migrate construction
and consumption sites in minimal commits. The key difference from enum-status-types is that this
spec replaces a `serde_json::Value` domain field (not a `String` field), which requires a
deserialization strategy at the DB boundary.

### ADR-1: Manual dispatch for deserialization, untagged for serialization

**Decision:** `VerbMetrics` derives `Serialize` with `#[serde(untagged)]` but does NOT derive
`Deserialize`. DB parsing uses `VerbMetrics::from_db(verb, mode, json_str)` which manually
dispatches to the correct struct's `Deserialize` impl based on verb/mode context.

**Rationale:** `#[serde(untagged)]` deserialization tries variants in declaration order, which
is fragile:
- `CaptureGenericMetrics` with `#[serde(flatten)]` on a `BTreeMap` matches ANY JSON object
- Structs sharing common field names (e.g., `total_commits` in both CaptureGit and Evolve)
  could match the wrong variant
- Variant ordering becomes a silent correctness dependency

Manual dispatch avoids these issues because we always know the verb and mode at the parse site
(they come from the same DB row as the metrics JSON). The dispatch is explicit:

```rust
pub fn from_db(verb: &str, mode: &str, json_str: &str) -> Self {
    let result = match (verb, mode) {
        ("capture", "code") => serde_json::from_str::<CaptureCodeMetrics>(json_str)
            .map(VerbMetrics::CaptureCode),
        ("capture", "git") => serde_json::from_str::<CaptureGitMetrics>(json_str)
            .map(VerbMetrics::CaptureGit),
        ("index", _) => serde_json::from_str::<IndexMetrics>(json_str)
            .map(VerbMetrics::Index),
        ("search", _) => serde_json::from_str::<SearchMetrics>(json_str)
            .map(VerbMetrics::Search),
        ("believe", _) => serde_json::from_str::<BelieveMetrics>(json_str)
            .map(VerbMetrics::Believe),
        ("evolve", _) => serde_json::from_str::<EvolveMetrics>(json_str)
            .map(VerbMetrics::Evolve),
        _ => Err(serde_json::Error::custom("unknown verb/mode")),
    };

    result.unwrap_or_else(|e| {
        tracing::warn!(verb, mode, error = %e, "Falling back to raw metrics");
        let value = serde_json::from_str(json_str)
            .unwrap_or(serde_json::Value::Null);
        VerbMetrics::Raw(value)
    })
}
```

For the "capture" verb with unknown modes, fall through to `CaptureGenericMetrics`:
```rust
        ("capture", _) => serde_json::from_str::<CaptureGenericMetrics>(json_str)
            .map(VerbMetrics::CaptureGeneric),
```

### ADR-2: Option<f64> for search metrics — intentional behavior change

**Decision:** `SearchMetrics.p_at_5`, `.mrr`, `.recall_at_5` are `Option<f64>` instead of
defaulting to 0.0.

**Rationale:** Per [[silent-default-hides-missing-data]], "missing" must not equal "zero."
A search evaluation that hasn't computed MRR yet is not the same as MRR=0.0.

**User-visible change:** When a search metric is `None`, renderers show "n/a" instead of
silently omitting or defaulting. The status decision in `build_search_summary` already handles
`None` correctly (only flags `NeedsAttention` when p_at_5 IS present and < 0.4).

### ADR-3: HistoryEntry.metrics stays as Value (deferred)

**Decision:** `HistoryEntry.metrics: serde_json::Value` is not migrated in this spec.

**Rationale:**
- History entries are only rendered via `format_metrics_inline` (generic key-value iteration)
- History shapes differ from summary shapes (e.g., `{ beliefs, floating, avg_evidence }` vs
  full `BelieveMetrics`) — typing them requires additional structs for minimal benefit
- The history builders (`get_believe_history`, `get_evolve_history`) construct Value from SQL
  but these are only consumed generically

**Follow-up:** Reuse `VerbMetrics::from_db` in history queries once the summary path is proven.
This prevents divergence between the two parse paths.

### ADR-4: Raw fallback monitoring + synthetic test

**Decision:** `VerbMetrics::Raw` fallback emits `tracing::warn!` with verb, mode, and error
context. A unit test validates the fallback path with synthetic unknown payloads.

**Rationale:** Without logging, new metric shapes from future tools would silently pass through
as untyped Value, re-introducing the exact problem this spec solves. The warning makes new
shapes discoverable during development so they can be added as typed variants.

**Test:** Phase 1 includes a unit test that passes unrecognized JSON to `from_db()` for each
verb (e.g., `{"unknown_key": 42}`), asserts `Raw` variant is returned, and confirms rendering
via `format_metrics_inline` / system view produces valid output without panic.

### ADR-5: Baseline capture and MCP payload diff

**Decision:** Before Phase 2 migration, capture `patina measure --json` output as a baseline
file. After migration, diff against the baseline to verify no field renames, key drops, or
structural changes.

**Rationale:** `#[serde(untagged)]` should preserve the flat JSON shape, but the diff catches
regressions that static analysis can't — e.g., field ordering changes, float formatting
differences, or accidental nesting from a wrong serde attribute. The diff also serves as
documentation of the exact output contract.

**UX note:** The one intentional change is search metrics: absent `p_at_5` / `mrr` /
`recall_at_5` will serialize as `null` (from `Option<f64>`) instead of being omitted or
defaulting to 0. Renderers show "n/a" for these. This is a user-visible change per ADR-2
that should be expected by anyone consuming the output.

## Commits

1. `define VerbMetrics enum and per-verb metric structs` — 7 typed structs + enum with Raw
   fallback + from_db() constructor. Pure additions, no consumers changed.

2. `migrate SourceSummary.latest_metrics from Value to VerbMetrics` — swap field type, update
   all 5 construction sites (builders), update all consumption sites (renderers + status
   decisions), update collect_measure_sources to use from_db() at DB boundary. Single-file
   atomic change in internal.rs.

3. `verify JSON shape and clean up remaining Value chains` — confirm MCP/JSON output preserved,
   verify zero .get().as_*() chains on metrics, check all exit criteria, run pre-push.

## Key Files

- `src/commands/measure/internal.rs` — sole target: types, construction, consumption, rendering
- `src/commands/measure/mod.rs` — unchanged (MeasureOptions already typed)

## Not Touched

- `src/spec.rs` — SpecStatus enum (reference pattern only)
- `src/mother/graph.rs` — BeliefStatus enum (reference pattern only)
- `src/commands/doctor.rs` — HealthStatus enum (reference pattern only)
- DB schema — no changes, metrics remain as JSON blobs in eventlog
- `HistoryEntry.metrics` — deferred per ADR-3

## Open Questions

_Resolved during review session 20260301-191035:_

1. ~~Separate structs vs generic?~~ Separate structs — verbs have genuinely different fields,
   generic map reintroduces type erasure. (Reviewer + design agent agree.)

2. ~~MCP consumers?~~ JSON shape preserved via `#[serde(untagged)]`. Exit criterion
   `json-shape-preserved` verifies with smoke test.

3. ~~Commit count?~~ 3 commits, 1 session. Could split commit 2 per-verb if diff feels large,
   but not required since it's single-file.

4. ~~HistoryEntry?~~ Deferred per ADR-3. Follow-up reuses from_db() parser.
