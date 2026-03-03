# Design: Structural Entropy Tracking in Measure System

## Approach

All structural metrics are **computed at scrape time** and emitted as a
`measure.capture` event with mode `structure`. The measure command reads
these events like any other verb metric — no live computation at display
time (Kelley: same freshness guarantee for all data in the same table).

### Data flow

```
patina scrape
  → walk src/ dirs (module count)
  → query function_facts + type visibility (pub interface count)
  → parse Cargo.toml (dependency count)
  → aggregate import_facts cross-module (coupling fan-out)
  → emit measure.capture { mode: "structure", metrics: {...} }

patina measure
  → read latest measure.capture.structure event
  → compare with previous event (delta)
  → warn if delta exceeds hardcoded thresholds

patina context
  → read latest structure metrics
  → render one-line summary in health section
```

### Coupling metric definition

**Cross-module fan-out**: for each top-level module under `src/`, count
how many *other* top-level modules it imports from (via `import_facts`
where `import_path` starts with `crate::`). Report:
- Per-module fan-out list
- Average fan-out across all modules
- Max fan-out (the most coupled module)

Uses existing `import_facts` data from the scraper. No new parsing.

### Threshold defaults (hardcoded v1)

| Metric | Warn when delta exceeds |
|--------|------------------------|
| Module count | +2 since last scrape |
| Public interfaces | +10% since last scrape |
| Dependency count | +1 since last scrape |
| Max fan-out | +2 since last scrape |

Config support deferred — add `.patina/config.toml` when needed.

### Pub interface count — scraper coverage

The scraper's `function_facts` tracks functions with visibility. Types
(struct/enum/trait) need visibility tracking too. Two options:
1. Extend scraper to emit type visibility into a queryable form
2. Query tree-sitter AST for `pub` items at scrape time (separate pass)

Option 1 is preferred (Gjengset: extend existing typed data, don't bolt
on a separate grep). Check what the scraper already captures for types
before implementing.

## Audit Decisions (session 20260303-172950)

- **Drop abstraction depth** — no EC, not a useful entropy proxy (unanimous)
- **Keep coupling metric (EC4)** — data exists in import_facts, use it (Gjengset)
- **Hardcode thresholds** — no config system yet, sensible defaults first (unanimous)
- **drift-detection depends on this spec** — measure produces, drift consumes (unanimous)
- **Collapse EC1-3 into one EC** — same pattern, one deliverable (Yegge)
- **All metrics from scrape time** — no live computation in measure (Kelley)
- **Extend scraper for type visibility** — don't grep at runtime (Gjengset)

## Commits

1. `feat(scrape): emit structural metrics during code scrape` — module count, pub interface count, dep count, coupling fan-out → measure.capture.structure event
2. `feat(measure): display structural metrics and delta warnings` — CaptureStructureMetrics variant, threshold comparison, render in full view
3. `feat(context): include structural entropy summary` — one-line codebase shape in context health section

## Key Files

- `src/commands/scrape/code/mod.rs` — where structural metrics are computed and emitted
- `src/commands/measure/internal.rs` — CaptureStructureMetrics struct, VerbMetrics variant, display logic
- `src/commands/measure/mod.rs` — public interface (no changes expected)
- `src/commands/context.rs` — add structure summary to health section
- `src/commands/assay/internal/imports.rs` — reference for import_facts query patterns

## Open Questions

- Does the scraper already track visibility on types (struct/enum/trait),
  or only on functions? Need to read scraper code before commit 1.
