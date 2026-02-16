---
type: feat
id: measurement-coverage
status: draft
created: 2026-02-16
sessions:
  origin: 20260216-091624
related:
- layer/core/patina-identity.md
- layer/surface/build/feat/belief-truthfulness/SPEC.md
beliefs:
- measure-the-measurement
- measure-first
- error-analysis-over-architecture
---

# feat: Measurement Coverage — Inventory, Gaps, and System Health

> Patina measures some things well and other things not at all. This spec maps
> every measurement tool to the protocol verb it serves, identifies the three
> unmeasured verbs, and defines a CLI surface that shows system-wide measurement
> health at a glance.

## Problem

Patina's identity defines five protocol verbs: **capture, index, search, believe,
evolve** (`layer/core/patina-identity.md:19`). Each verb has a different level of
measurement coverage today:

- **Search** has 8 distinct eval modes across `eval` and `bench`
- **Believe** has audit, verification, grounding, staleness, and health scoring
- **Capture** has zero measurement — scrape runs, but its quality is unquantified
- **Index** has zero measurement — oxidize builds embeddings, but per-project quality is unknown
- **Evolve** has zero measurement — knowledge maturation is invisible

A Patina maintainer today cannot answer: "Which parts of my system are measured,
and which are flying blind?" The data exists to answer this for search and believe.
For the other three verbs, even the *questions* haven't been defined.

This is the [[measure-the-measurement]] belief applied at system scale: before
building new measurement tools, organize what exists and name what's missing.

## What Exists Today

### Inventory: Measurement Tools by Protocol Verb

#### Search (8 eval modes — well-measured)

| Tool | Measures | Metrics | Code |
|------|----------|---------|------|
| `patina eval` | Unified pipeline co-retrieval | P@5, P@10, vs-random | `src/commands/eval/mod.rs:55-295` |
| `patina eval` (ablation) | Per-oracle contribution | Delta P@10 with budget | `src/commands/eval/mod.rs:209-293` |
| `patina eval` (belief delta) | Belief oracle D1 impact | MRR, co-retrieval rate | `src/commands/eval/mod.rs:127-150` |
| `patina eval --nl` | NL query precision | P@5, P@10, MRR, train/test split | `src/commands/eval/mod.rs:1283-1515` |
| `patina eval --feedback` | Real-world session→commit precision | Precision by rank, per-session | `src/commands/eval/mod.rs:945-1158` |
| `patina eval --assay` | FTS5 factual retrieval | Independent assay quality | `src/commands/eval/mod.rs:1522-1524` |
| `patina eval --scry` | Semantic vector retrieval | Scry quality + scry-vs-assay | `src/commands/eval/mod.rs:1527-1529` |
| `patina eval --combined` | Full pipeline (assay+scry) | End-to-end quality | `src/commands/eval/mod.rs:1537-1539` |
| `patina bench` | Retrieval performance | Queryset scoring, RRF tuning, oracle ablation | `src/commands/bench/mod.rs:40-55` |
| `patina bench --grammar` | Grammar dispatch perf | Compiled-in vs WASM A/B test | `src/commands/bench/mod.rs:58-61` |

**What "good" looks like for Search:**
- P@5 > 60% and P@10 > 40% on co-retrieval tests
- NL MRR > 0.5 on test split (no overfit: train-test gap < 10pp)
- Feedback precision increasing over sessions (real-world signal)
- Belief delta D1: PASS (knowledge gains positive, structural regression within budget)

#### Believe (5 measurement surfaces — well-measured)

| Tool | Measures | Metrics | Code |
|------|----------|---------|------|
| `patina belief audit` | Per-belief use and truth | Citations, evidence, verification, applications | `src/commands/belief/mod.rs:151-564` |
| `patina belief audit --grounding` | Semantic grounding | Nearest code/commit/session neighbors (E4.6a) | `src/commands/belief/mod.rs:571-724` |
| `patina belief audit --stale` | Temporal staleness | Last activity age, stale count, median age | `src/commands/belief/mod.rs:316-340` |
| `patina belief audit --sort health` | Health scoring | Weighted formula: evidence+verification+staleness+grounding | `src/commands/belief/mod.rs:177` |
| Verification engine | Structural correctness per belief | sql/assay/temporal queries, drift detection | `src/commands/scrape/beliefs/verification/mod.rs:66-104` |

**What "good" looks like for Believe:**
- 0 beliefs with `low-health` warning (health_score >= 0.4 for all)
- 0 beliefs with `verify-drifted` warning
- Evidence verified rate > 70%
- 0 floating beliefs (all grounded to code, commits, or sessions)
- Median activity age < stale_days threshold

#### Foundation (cross-cutting, not a protocol verb)

| Tool | Measures | Metrics | Code |
|------|----------|---------|------|
| `patina doctor` | Environment health | Tool availability, adapter version, layer patterns | `plugins/doctor/src/lib.rs:81-183` |
| `patina report` | Project state snapshot | File/function counts, architecture queries, RAG health | `src/commands/report/internal.rs:22-59` |

These serve all verbs. Doctor checks prerequisites. Report composes from scry/assay.

### Gaps: Unmeasured Protocol Verbs

#### Capture (zero measurement)

**What `scrape` does:** Reads source files, git history, layer files, and forge data.
Writes events to eventlog. Creates materialized views in SQLite (function_facts,
co_changes, beliefs, etc.). ~15K lines across scrape, scanner, eventlog, forge, git.

**What is unmeasured:**
1. **Completeness** — What fraction of source files were successfully parsed? Are
   there files that tree-sitter skipped or partially parsed?
2. **Freshness** — How old is the scraped data? When was the last scrape? Does the
   eventlog reflect current git HEAD?
3. **Accuracy** — Are function_facts correct? Do co_changes match real git history?
   No ground truth comparison exists.
4. **Coverage rate** — For a project with N source files, what % have entries in the
   function_facts table? What % of commits have associated co_change entries?

**What "good" looks like for Capture:**
- Parse success rate > 95% of source files
- function_facts coverage > 90% of parseable files
- Eventlog freshness < 1 commit behind HEAD
- co_changes table populated for > 80% of files with 3+ commits

#### Index (zero measurement)

**What `oxidize` does:** Builds ONNX embeddings (E5-base-v2), FTS5 indexes,
structural graphs, temporal co-change matrices. Transforms raw scrape data into
searchable form.

**What is unmeasured:**
1. **Embedding coverage** — What fraction of scraped content has embeddings? Are
   there documents that failed to embed?
2. **Index freshness** — Is the usearch index current with the latest scrape? Are
   there new eventlog entries without corresponding embeddings?
3. **FTS5 completeness** — Does the FTS5 index cover all content types? Are there
   event types that aren't indexed?
4. **Projection quality** — Do 256-dim projections preserve relative distances? (This
   is partially tested by `patina eval --scry-raw` but not surfaced as a metric.)

**What "good" looks like for Index:**
- Embedding coverage = 100% of scrape output
- Index freshness: 0 unembedded documents
- FTS5 coverage: all content types indexed
- Projection error: cosine correlation > 0.95 between raw and projected

#### Evolve (zero measurement)

**What `evolve` represents:** Patterns move through core → surface → dust. Sessions
distill into beliefs. Beliefs gain or lose entrenchment through evidence. The layer
is a living document (`layer/core/patina-identity.md:52-55`).

**What is unmeasured:**
1. **Maturation velocity** — How fast do surface patterns move to core? What's the
   average time from belief creation to entrenchment change?
2. **Knowledge growth rate** — Beliefs per week, patterns per month, session
   distillation rate (sessions that produce beliefs vs sessions that don't).
3. **Layer health** — Ratio of active to archived beliefs. Ratio of core to surface
   patterns. Are patterns accumulating without maturing?
4. **Distillation rate** — What fraction of sessions produce at least one belief?
   What fraction of beliefs link back to a session?

**What "good" looks like for Evolve:**
- Session→belief distillation rate > 30% (sessions producing beliefs)
- Belief maturation: at least 1 entrenchment change per month
- Layer growth: beliefs growing, dust not accumulating faster than core
- No orphaned patterns (every surface pattern either matures or archives within 90d)

## Design: Measurement Health View

### Philosophy

This is **not** a new measurement engine. This is an **inventory and gap reporter**
that composes existing signals into a single view. The belief is [[measure-the-measurement]]:
fix the instrument before doubting the observation.

### CLI Surface: `patina measure`

A new subcommand that reports measurement coverage across all five protocol verbs.
Analogous to `patina belief audit` but for the measurement system itself.

```
$ patina measure

  Measurement Coverage — 5 protocol verbs

  VERB       TOOLS  METRICS  STATUS     NOTES
  ─────      ─────  ───────  ──────     ─────
  search       10      12    covered    eval(8) + bench(2)
  believe       5       8    covered    audit(3) + verification(1) + truthfulness(1)
  capture       0       0    gap        scrape runs but quality unmeasured
  index         0       0    gap        oxidize runs but coverage unmeasured
  evolve        0       0    gap        layer lifecycle unmeasured

  Coverage: 2/5 verbs measured (15 tools, 20 metrics)

  ── Verb Details ──

  search:
    last eval:     2026-02-14 (2d ago)
    nl test MRR:   0.672
    feedback P@5:  41.2%
    belief D1:     PASS

  believe:
    total beliefs: 130
    median health: 0.73
    stale:         4/130 (>30d)
    floating:      2/130
    verify-drift:  0

  capture: NOT MEASURED
    last scrape:   2026-02-16
    (no quality metrics available)

  index: NOT MEASURED
    last oxidize:  2026-02-16
    (no quality metrics available)

  evolve: NOT MEASURED
    (no maturation metrics available)
```

### Data Sources

`patina measure` reads from existing data — it creates no new tables and runs no
new computations. It composes:

1. **Search metrics**: Read from `patina.db` tables populated by `eval` and `bench`.
   If no eval has been run, show "not run" instead of failing.
2. **Believe metrics**: Read from `beliefs` table (same data as `belief audit`).
   Summary statistics only — median health, stale count, floating count.
3. **Capture/Index/Evolve**: Show "NOT MEASURED" with timestamp of last scrape/oxidize.
   These are honest gaps, not fabricated metrics.

### Phase 1 — Inventory and Gaps (this spec)

**Scope:** Read-only reporter. Composes existing data into the measurement health view.

- [ ] Create `src/commands/measure/mod.rs` following dependable-rust pattern
- [ ] Read eval history from `patina.db` (look for eval result events in eventlog,
      or fall back to "not run" if no eval events exist)
- [ ] Read belief summary from `beliefs` table (reuse existing SQL from belief audit)
- [ ] Display verb-by-verb coverage table with status: `covered`, `partial`, `gap`
- [ ] Display verb details for covered verbs (latest metrics)
- [ ] Display honest "NOT MEASURED" for gap verbs with last tool run timestamp
- [ ] Wire to CLI as `patina measure` subcommand
- [ ] Register in `patina-identity.md` Protocol Tooling table (measurement reporter)

### Phase 2 — Capture Measurement (future spec)

NOT in scope for this spec. Placeholder for what a capture-quality measurement would
need:

- Parse success rate: count tree-sitter parse errors during scrape
- function_facts coverage: `SELECT COUNT(DISTINCT file) FROM function_facts` / total files
- Eventlog freshness: compare latest eventlog timestamp to git HEAD timestamp
- These metrics would be computed during `patina scrape` and stored as measurement events

### Phase 3 — Index Measurement (future spec)

NOT in scope. Placeholder:

- Embedding coverage: count documents in usearch index vs documents in eventlog
- Index freshness: compare usearch document count to scrape event count
- Projection quality: sample cosine correlation between raw and projected vectors

### Phase 4 — Evolve Measurement (future spec)

NOT in scope. Placeholder:

- Session→belief distillation rate: sessions with `## Beliefs Captured: >0` / total sessions
- Belief maturation velocity: entrenchment changes over time from eventlog
- Layer health ratios: computed from file counts in core/surface/dust

## Exit Criteria

Phase 1 is complete when:

1. `patina measure` runs and displays the verb-by-verb coverage table
2. Search verb shows latest eval metrics (or "not run")
3. Believe verb shows summary stats from beliefs table
4. Capture, Index, Evolve verbs show "NOT MEASURED" honestly
5. No new tables, no new computations — read-only composition
6. Zero clippy warnings, all existing tests pass
7. Registered in `patina-identity.md` Protocol Tooling table

## Verification Plan

```verify
-- Phase 1: Command exists and runs without error
SELECT 1 WHERE EXISTS (SELECT 1);
expect: >= 1
label: trivial-check
```

Post-implementation manual verification:
- `patina measure` exits 0 and displays all 5 verbs
- `patina measure` with no eval history shows "not run" for search details
- `patina measure` with no beliefs shows "0 beliefs" not an error
- Output fits in 80-column terminal

## Risks

1. **Eval history not persisted** — `patina eval` prints results but may not store
   them in eventlog. If so, Phase 1 falls back to "eval available but no stored results"
   and a follow-up stores eval results as events.
2. **Scope creep** — This spec deliberately does NOT build new measurement tools for
   capture/index/evolve. Those are separate specs. This spec inventories and surfaces.
3. **Metric staleness** — The measurement health view shows point-in-time data. It
   does not detect when eval results are outdated. This is acceptable for Phase 1;
   Phase 2+ can add freshness warnings.
