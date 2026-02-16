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

# feat: Measurement Coverage System

> A complete measurement framework for Patina. Every protocol verb gets measured.
> Every measurement gets stored. Plugins can register their own measurements. Two
> consumers — project users and system maintainers — see what they need to see.

## Problem

Patina's identity defines five protocol verbs: **capture, index, search, believe,
evolve** (`layer/core/patina-identity.md:19`). Measurement coverage is lopsided:

- **Search** has 10 eval/bench modes producing 12+ metrics
- **Believe** has 5 measurement surfaces (audit, grounding, staleness, health, verification)
- **Capture** has zero measurement — scrape runs, quality unquantified
- **Index** has zero measurement — oxidize builds embeddings, quality unknown
- **Evolve** has zero measurement — knowledge maturation invisible

But the problem is deeper than missing tools. Patina has **no measurement
infrastructure** — no event schema for metrics, no storage for results, no way for
plugins to report measurements, and no distinction between what a *project user*
needs to see versus what a *system maintainer* needs to see.

Today's measurement tools (`eval`, `bench`, `belief audit`) print results to stdout
and discard them. `eval` doesn't write to eventlog at all. There is no way to
answer "has search quality regressed since last week?" because no history exists.

This spec builds the measurement system: infrastructure, per-verb coverage, plugin
extensibility, and consumer-appropriate views.

## Consumers

Two consumers use this system. Their needs are different, and the CLI must serve both.

### Patina User (project consumer)

A developer using Patina on their project. They did NOT build Patina. They want to
understand how well Patina understands *their* codebase.

**What they control:**
- Their project files (source code, layer/, beliefs)
- Plugin selection and configuration (add/remove/configure plugins)
- Model selection (may switch embedding models)

**What they do NOT control:**
- Patina core code
- Protocol implementation
- Measurement tool internals

**What they need to see:**
- "Is Patina capturing my project well?" — parse coverage, file count, freshness
- "Are embeddings working for my code?" — index coverage, document count
- "Is search useful for my queries?" — feedback precision from their sessions
- "Are my beliefs healthy?" — stale count, floating count, contested count
- "Is knowledge growing?" — sessions producing beliefs, patterns maturing
- "Are my plugins reporting health?" — plugin-contributed measurements
- Actionable next steps: "run `patina scrape` to refresh", "2 beliefs need attention"

**UX principle:** Show project health. Hide system internals. Surface actions.

### Patina Maintainer (system builder)

A developer building Patina itself. They own the protocol implementation and are
responsible for measurement quality across all verbs.

**What they control:**
- Everything — core code, infrastructure, plugin APIs, measurement tools

**What they need to see:**
- Which verbs have measurement tools and which don't (coverage map)
- Measurement regression: has P@5 dropped since the last release?
- Instrument accuracy: are the measurement tools themselves correct?
- Infrastructure gaps: what's blocking new measurements?
- Per-tool detail: all 10 eval modes, all 5 belief surfaces, raw numbers
- Plugin measurement API health: are plugins using the measurement interface?

**UX principle:** Show everything. Enable regression detection. Support the
[[measure-the-measurement]] loop.

## What Exists Today

### Inventory: Measurement Tools by Protocol Verb

#### Search (10 tools, 12+ metrics — well-measured)

| Tool | Measures | Key Metrics | Code |
|------|----------|-------------|------|
| `patina eval` | Unified pipeline co-retrieval | P@5, P@10, vs-random | `src/commands/eval/mod.rs:55-295` |
| `patina eval` (ablation) | Per-oracle contribution | Delta P@10 with budget | `src/commands/eval/mod.rs:209-293` |
| `patina eval` (belief delta) | Belief oracle D1 impact | MRR, co-retrieval rate | `src/commands/eval/mod.rs:127-150` |
| `patina eval --nl` | NL query precision | P@5, P@10, MRR, train/test split | `src/commands/eval/mod.rs:1283-1515` |
| `patina eval --feedback` | Real-world session→commit precision | Precision by rank, per-session | `src/commands/eval/mod.rs:945-1158` |
| `patina eval --assay` | FTS5 factual retrieval | Independent assay quality | `src/commands/eval/mod.rs:1522-1524` |
| `patina eval --scry` | Semantic vector retrieval | Scry quality + scry-vs-assay | `src/commands/eval/mod.rs:1527-1529` |
| `patina eval --combined` | Full pipeline (assay+scry) | End-to-end quality | `src/commands/eval/mod.rs:1537-1539` |
| `patina bench` | Retrieval performance | Queryset scoring, RRF tuning | `src/commands/bench/mod.rs:40-55` |
| `patina bench --grammar` | Grammar dispatch perf | Compiled-in vs WASM A/B | `src/commands/bench/mod.rs:58-61` |

**Critical gap:** None of these write results to eventlog. All results are printed
to stdout and lost. No measurement history, no regression detection.

#### Believe (5 surfaces, 8+ metrics — well-measured)

| Tool | Measures | Key Metrics | Code |
|------|----------|-------------|------|
| `patina belief audit` | Per-belief use and truth | Citations, evidence, applications | `src/commands/belief/mod.rs:151-564` |
| `patina belief audit --grounding` | Semantic grounding | Nearest code/commit/session neighbors | `src/commands/belief/mod.rs:571-724` |
| `patina belief audit --stale` | Temporal staleness | Last activity age, stale count | `src/commands/belief/mod.rs:316-340` |
| `patina belief audit --sort health` | Health scoring | Weighted formula (E4 belief-truthfulness) | `src/commands/belief/mod.rs:177` |
| Verification engine | Structural correctness | sql/assay/temporal queries, drift | `src/commands/scrape/beliefs/verification/mod.rs:66-104` |

**Partial gap:** Belief metrics are stored in the `beliefs` table (materialized view
from scrape), but no historical snapshots exist. Can see today's health, can't see
last month's.

#### Capture (zero measurement)

`scrape` processes source files, git history, layer files, forge data. Writes events
to eventlog. Creates materialized views (function_facts, co_changes, beliefs).
~15K lines across scrape, scanner, eventlog, forge, git modules.

Nothing measures scrape quality: parse success rate, coverage, freshness, accuracy.

#### Index (zero measurement)

`oxidize` builds ONNX embeddings (E5-base-v2), FTS5 indexes, structural graphs,
temporal co-change matrices. Transforms scrape data into searchable form.

Nothing measures index quality: embedding coverage, index freshness, FTS5
completeness. (`patina eval --scry-raw` partially tests projection quality but
doesn't surface it as a metric.)

#### Evolve (zero measurement)

Knowledge maturation: patterns move core → surface → dust, sessions distill into
beliefs, beliefs gain/lose entrenchment. This is the *product* of Patina — the layer
accumulating wisdom over time.

Nothing measures evolution: maturation velocity, distillation rate, layer health
ratios, knowledge growth.

#### Foundation (cross-cutting)

| Tool | Measures | Code |
|------|----------|------|
| `patina doctor` | Environment health (tools, adapter, layer) | `plugins/doctor/src/lib.rs:81-183` |
| `patina report` | Project state snapshot via scry/assay | `src/commands/report/internal.rs:22-59` |

Doctor is already a WASM command plugin — it proves that plugins can do health
checks. Report composes from existing query tools.

### Infrastructure Gaps

1. **No measurement event schema** — no event_type for metrics in eventlog
2. **No measurement storage** — eval/bench print and discard
3. **No measurement history** — can't compare today vs last week
4. **No plugin measurement API** — plugins can't report metrics to the system
5. **No consumer-specific views** — one output for everyone

## Reference Pattern: Belief Audit

The believe verb's measurement stack — built across E4 (use/truth metrics), E4.6a
(grounding), and [[belief-truthfulness]] (staleness, health, drift, contradiction) —
already implements the measurement system pattern that this spec generalizes to all
five verbs. It is the reference implementation.

### The Pattern

```
producer        →  storage         →  system view      →  detail view
─────────          ───────            ───────────         ───────────
patina scrape   →  beliefs table   →  patina measure   →  patina belief audit
(computes)         (materialized)     (verb summary)      (per-belief drill-down)
```

Belief audit demonstrates every layer of the measurement system:

| Layer | Belief Audit Implementation | Code |
|-------|---------------------------|------|
| **Metrics** | health_score, last_activity, verification_drifted, contested_by | `src/commands/belief/mod.rs:57-80` |
| **Thresholds** | health < 0.4 → low-health, last_activity > stale_days → stale | `src/commands/belief/mod.rs:112-148` |
| **Warnings** | health_warnings() returns actionable flags per belief | `src/commands/belief/mod.rs:112-148` |
| **Summary stats** | stale count, median age, floating count, warning breakdown | `src/commands/belief/mod.rs:429-555` |
| **Filters** | --stale, --warnings-only, --sort health — consumer drill-down | `src/commands/belief/mod.rs:151-153` |
| **Storage** | beliefs table (materialized view from scrape) | `src/commands/scrape/beliefs/mod.rs` |

### What Each Verb Needs

Every verb needs this same stack. Belief audit proves the pattern; the measurement
system generalizes it:

| Layer | Believe (exists) | Search (partial) | Capture (gap) | Index (gap) | Evolve (gap) |
|-------|-----------------|-----------------|--------------|------------|-------------|
| **Producer** | scrape | eval, bench | scrape | oxidize | session lifecycle |
| **Storage** | beliefs table | stdout (lost) | none | none | none |
| **System view** | (new: measure) | (new: measure) | (new: measure) | (new: measure) | (new: measure) |
| **Detail view** | belief audit | eval output | (future) | (future) | (future) |
| **Metrics** | 8+ per belief | 12+ per run | 0 | 0 | 0 |
| **Thresholds** | health < 0.4, stale_days | D1 budget 5pp | (undefined) | (undefined) | (undefined) |
| **Warnings** | 10 warning types | PASS/FAIL only | none | none | none |

The critical gap is visible: **search has metrics but no storage**, and the other
three verbs have **no metrics at all**. Belief audit's progression from zero to
full measurement stack (across 4 specs over 3 weeks) is the path each verb follows.

### Three-Level View

The measurement system creates a three-level hierarchy. Belief audit already fills
the bottom two levels for the believe verb:

```
patina measure              ← system view (all 5 verbs, one line each)
  └─ patina measure --verb  ← verb detail (one verb, summary stats + history)
       └─ verb-specific cmd ← item detail (per-belief, per-query, per-file)
```

For believe, the item-detail level is `patina belief audit`. For search, it's
`patina eval`. For capture/index/evolve, the item-detail commands don't exist yet
and are out of scope for this spec — but the measurement system's verb-detail level
(`patina measure --verb capture`) will show whatever metrics the producers emit,
even before dedicated detail commands exist.

## Design: The Measurement System

### Core Idea

Measurements are events. Like everything else in Patina, they flow through the
eventlog. Measurement events have a standard schema that both core tools and plugins
write to. The `patina measure` command reads these events and presents consumer-
appropriate views.

```
                ┌─────────────────────────────────┐
                │       Measurement Producers      │
                │                                  │
                │  Core:     scrape, oxidize, eval │
                │  Plugins:  doctor, grammar, ...  │
                └──────────────┬───────────────────┘
                               │
                       measurement events
                               │
                               ▼
                ┌─────────────────────────────────┐
                │           Eventlog               │
                │  event_type: measure.*           │
                │  verb, tool, metric, value, ts   │
                └──────────────┬───────────────────┘
                               │
                        read + compose
                               │
                               ▼
                ┌─────────────────────────────────┐
                │       patina measure             │
                │                                  │
                │  User view:  project health      │
                │  Maint view: system coverage     │
                └─────────────────────────────────┘
```

### Measurement Event Schema

All measurements are eventlog events with `event_type = "measure.<verb>"`:

```json
{
  "event_type": "measure.search",
  "timestamp": "2026-02-16T14:30:00Z",
  "data": {
    "verb": "search",
    "tool": "eval",
    "mode": "nl",
    "metrics": {
      "p_at_5": 0.72,
      "p_at_10": 0.48,
      "mrr": 0.672,
      "query_count": 52,
      "train_test_gap_pp": -3.2
    },
    "source": "core"
  }
}
```

Fields:
- **verb** — one of: capture, index, search, believe, evolve
- **tool** — which tool produced this measurement (eval, bench, scrape, oxidize, etc.)
- **mode** — tool-specific sub-mode (nl, feedback, ablation, etc.)
- **metrics** — key-value pairs, all numeric (f64 or i64)
- **source** — "core" for compiled-in tools, plugin name for WASM plugins

This schema is the contract. Core tools and plugins both write to it. The `measure`
command reads from it. History is automatic — eventlog is append-only.

### Plugin Measurement Interface

Plugins report measurements through a new WIT host interface:

```wit
/// Measurement reporting for plugins.
///
/// Plugins call this to record metrics. The host writes the
/// measurement event to eventlog with the plugin's name as source.
interface measure {
    /// Record a measurement.
    ///
    /// `verb` — protocol verb being measured (capture, index, search, believe, evolve)
    /// `tool` — tool name (e.g., "doctor", "grammar-python")
    /// `mode` — measurement mode (e.g., "health-check", "parse-coverage")
    /// `metrics_json` — JSON object of metric name → numeric value
    record-measurement: func(
        verb: string,
        tool: string,
        mode: string,
        metrics-json: string,
    ) -> result<_, string>;
}
```

This adds to the existing host capabilities: `log`, `layer`, `query`, `http`.
Command plugins and mother-child plugins both get access.

**Example: Doctor plugin measuring capture freshness:**
```rust
// Inside doctor's run(), after checking environment
measure::record_measurement(
    "capture",
    "doctor",
    "freshness-check",
    &serde_json::json!({
        "eventlog_age_hours": hours_since_last_scrape,
        "commits_behind": commits_since_last_scrape,
    }).to_string(),
)?;
```

**Example: Grammar plugin measuring parse coverage:**
```rust
// After parsing files
measure::record_measurement(
    "capture",
    "grammar-rust",
    "parse-coverage",
    &serde_json::json!({
        "files_attempted": total,
        "files_parsed": succeeded,
        "parse_success_rate": succeeded as f64 / total as f64,
        "partial_parses": partial,
    }).to_string(),
)?;
```

### CLI Surface: `patina measure`

#### User View (default)

What a Patina project user sees. Focused on *their project's health*.

```
$ patina measure

  Project Measurement Health

  VERB       STATUS     HEALTH     LAST RUN        ACTION NEEDED
  ─────      ──────     ──────     ────────        ─────────────
  capture    measured   94%        2h ago
  index      measured   100%       2h ago
  search     measured   good       1d ago
  believe    measured   12 warn    2h ago          2 beliefs stale
  evolve     partial    33%        5d ago          low distillation rate

  Overall: 4/5 verbs healthy, 1 needs attention

  ── Details ──

  capture:
    files parsed:    195/198 (98.5%)
    function_facts:  4,231 across 183 files
    freshness:       1 commit behind HEAD → run `patina scrape`

  index:
    documents:       12,847 embedded (100%)
    index fresh:     yes
    model:           e5-base-v2

  search:
    feedback P@5:    41.2% (last 3 sessions)
    queries served:  847 this month

  believe:
    beliefs:         130 total
    healthy:         118 (health ≥ 0.4)
    stale:           4 (>30d)         → review with `patina belief audit --stale`
    floating:        2                → ground with evidence
    verify-drift:    0

  evolve:
    sessions:        12 this month
    distilled:       4 (33%)          → capture more beliefs with /session-end
    maturation:      0 entrenchment changes
    layer growth:    +8 beliefs, +3 patterns

  plugins reporting: doctor (foundation), grammar-rust (capture)
```

**Key UX decisions:**
- Status is `measured`, `partial`, or `gap` — not "covered" (too jargony)
- Health is expressed in the unit natural to the verb (%, count, quality word)
- Action items are inline, not a separate section
- Plugin contributions are visible but not prominent
- No tool internals — user doesn't see "P@10" or "MRR", they see "good" or "degraded"

#### Maintainer View

What a Patina developer sees. Focused on *system-wide measurement coverage*.

```
$ patina measure --system

  Measurement System Coverage — 5 protocol verbs

  VERB       TOOLS  METRICS  STORED  STATUS     NOTES
  ─────      ─────  ───────  ──────  ──────     ─────
  search       10      12    yes     covered    eval(8) + bench(2)
  believe       5       8    yes     covered    audit(3) + verification(1) + truthfulness(1)
  capture       2       6    yes     covered    scrape(1) + grammar-rust(1)
  index         1       4    yes     covered    oxidize(1)
  evolve        1       4    yes     partial    session-lifecycle(1), no maturation tracking

  Coverage: 4/5 verbs covered, 1 partial (19 tools, 34 metrics, all stored)

  ── Measurement History ──

  search (last 5 runs):
    2026-02-16  eval/nl       P@5=72% P@10=48% MRR=0.672  train-test=-3.2pp
    2026-02-15  eval/nl       P@5=70% P@10=47% MRR=0.658  train-test=-2.8pp
    2026-02-14  bench/qs      Δ unified: +1.2pp P@10
    2026-02-12  eval/feedback P@5=41.2% (3 sessions, 47 queries)
    2026-02-10  eval/nl       P@5=68% P@10=45% MRR=0.641

    Trend: P@5 ↑4pp over 6 days, MRR ↑0.031

  believe:
    total: 130, median health: 0.73
    stale: 4/130 (>30d), floating: 2/130, drifted: 0
    (no historical snapshots — compare with next scrape)

  capture:
    2026-02-16  scrape        files=198 parsed=195 (98.5%) functions=4231
    2026-02-16  grammar-rust  files=183 parsed=183 (100%) partial=0
    (2 runs stored)

  index:
    2026-02-16  oxidize       docs=12847 embedded=12847 (100%) model=e5-base-v2

  evolve:
    2026-02-16  session-life  sessions=12 distilled=4 (33%) entrenchment_changes=0
    GAP: no maturation velocity tracking, no pattern lifecycle metrics

  ── Plugin Measurements ──

  PLUGIN          VERB      MODE              METRICS  LAST RUN
  doctor          capture   freshness-check   2        2h ago
  grammar-rust    capture   parse-coverage    4        2h ago

  ── Regressions ──

  (none detected — all metrics stable or improving)
```

**Key UX decisions:**
- Shows raw metrics (P@5, MRR) because maintainers need precision
- Shows measurement history with trends
- Shows plugin contributions as a distinct section
- Regression detection: compares latest run to previous, flags declines
- `STORED` column: confirms measurements are in eventlog, not just printed

### What "Good" Looks Like — Per Verb

These thresholds define when a verb's status shows `covered` vs `partial` vs `gap`
for the maintainer view, and `healthy` vs `needs attention` for the user view.

#### Capture

| Metric | Good | Needs attention | Source |
|--------|------|-----------------|--------|
| Parse success rate | > 95% | < 90% | scrape measurement event |
| function_facts coverage | > 90% of parseable files | < 80% | scrape measurement event |
| Eventlog freshness | ≤ 1 commit behind HEAD | > 5 commits behind | compare git HEAD to latest event |
| co_changes populated | > 80% of files with 3+ commits | < 60% | scrape measurement event |

#### Index

| Metric | Good | Needs attention | Source |
|--------|------|-----------------|--------|
| Embedding coverage | 100% of scrape output | < 95% | oxidize measurement event |
| Index freshness | 0 unembedded documents | > 0 | compare eventlog to usearch count |
| FTS5 completeness | All content types indexed | Missing types | oxidize measurement event |
| Projection quality | Cosine correlation > 0.95 | < 0.90 | eval --scry-raw (if stored) |

#### Search

| Metric | Good | Needs attention | Source |
|--------|------|-----------------|--------|
| NL P@5 | > 60% | < 40% | eval/nl measurement event |
| NL MRR | > 0.5 | < 0.3 | eval/nl measurement event |
| Train-test gap | < 10pp | > 15pp (overfit risk) | eval/nl measurement event |
| Feedback precision trending | Stable or improving | Declining over 3+ sessions | eval/feedback history |
| Belief D1 | PASS | FAIL | eval/d1 measurement event |

#### Believe

| Metric | Good | Needs attention | Source |
|--------|------|-----------------|--------|
| Low-health count | 0 beliefs < 0.4 | > 10% of beliefs | beliefs table |
| Verify-drifted count | 0 | Any | beliefs table |
| Evidence verified rate | > 70% | < 50% | beliefs table |
| Floating count | 0 | > 10% of beliefs | beliefs table |
| Median activity age | < stale_days | > stale_days | beliefs table |

#### Evolve

| Metric | Good | Needs attention | Source |
|--------|------|-----------------|--------|
| Session→belief distillation | > 30% | < 15% | session archive metadata |
| Entrenchment changes | ≥ 1/month | 0 for 60+ days | eventlog belief events |
| Layer growth | Positive (net new beliefs) | Stagnant for 30+ days | layer file counts |
| Orphaned patterns | 0 surface patterns > 90d without maturation or archive | > 5 | layer file dates |

## Implementation Phases

### Phase 1 — Measurement Event Infrastructure

Build the storage and schema. Make core tools write measurement events.

- [ ] Define `measure.*` event_type convention in eventlog
- [ ] `patina eval` writes measurement events after each run (all modes)
- [ ] `patina bench` writes measurement events after each run
- [ ] `patina scrape` writes capture measurement events (files parsed, functions
      extracted, coverage rate)
- [ ] `patina oxidize` writes index measurement events (documents embedded,
      coverage, model used)
- [ ] Session lifecycle writes evolve measurement events at session-end
      (distillation: did this session produce beliefs?)
- [ ] `patina measure` reads measurement events — basic verb-by-verb summary

**Exit criteria:** All 5 verbs have at least one measurement event producer.
`patina measure` displays a summary table. All events stored in eventlog.

### Phase 2 — Consumer Views

Build the two distinct CLI experiences.

- [ ] `patina measure` (default) — user view: project health, natural language,
      action items, hides internals
- [ ] `patina measure --system` — maintainer view: full tool inventory, raw metrics,
      measurement history, regression detection
- [ ] `patina measure --json` — machine-readable output for both views
- [ ] `patina measure --verb search` — drill into one verb

**Exit criteria:** Both views render correctly. User view uses health language
("good", "needs attention"). Maintainer view shows raw metrics and history.

### Phase 3 — Plugin Measurement API

Enable WASM plugins to report measurements.

- [ ] Add `patina:host/measure@0.1.0` interface to WIT
- [ ] Implement host-side: `record-measurement` writes to eventlog
- [ ] Add `measure` to command world imports
- [ ] Add `measure` to mother-child world imports
- [ ] Update `patina-sdk` crate with measurement convenience functions
- [ ] Migrate doctor plugin to use measurement API (proof of concept)
- [ ] `patina measure` includes plugin-contributed measurements in both views

**Exit criteria:** Doctor plugin writes measurements via WIT interface.
`patina measure` shows plugin measurements alongside core measurements.
A third-party plugin developer can add measurements to their plugin using
only the SDK docs.

### Phase 4 — Regression Detection

Surface measurement trends and flag regressions.

- [ ] Compare latest measurement event to previous for each tool+mode
- [ ] Flag significant declines (configurable thresholds from project config)
- [ ] `patina measure --system` shows trend arrows (↑ ↓ →) and delta values
- [ ] `patina measure` (user view) shows "search quality declining" as action item
- [ ] Optional: `patina measure --ci` exits non-zero on regression (for CI pipelines)

**Exit criteria:** Regressions are detected and surfaced in both views.
`--ci` mode enables measurement-gated CI workflows.

### Phase 5 — Measurement Completeness (per-verb enrichment)

Deepen measurement coverage for each verb. Each is a focused sub-spec.

- [ ] Capture: tree-sitter parse error tracking, per-language coverage
- [ ] Index: projection quality metric stored (from eval --scry-raw)
- [ ] Evolve: pattern lifecycle tracking (days in surface, maturation events)
- [ ] Search: per-project queryset generation (bench generate) with stored results

**Exit criteria:** All 5 verbs have "covered" status in maintainer view.
No verb has fewer than 3 metrics.

## Relationship to Existing Specs

- **[[belief-truthfulness]]** (complete) — The reference implementation. Built the
  full measurement stack for the believe verb: metrics → thresholds → warnings →
  summaries → drill-down. This spec generalizes that pattern to all five verbs.
  Belief audit's summary statistics (`mod.rs:429-555`) feed directly into
  `patina measure`'s believe-verb section — same data, system-level view.
- **[[cross-project-beliefs]]** (mother-v2 Phase 2) — Cross-project belief index will
  need measurement: federation health, cross-project search quality. The measurement
  event schema supports this — `measure.believe` events from multiple projects can
  be compared once mother-v2 enables cross-project queries.
- **[[patina-identity]]** — `patina measure` is protocol tooling. It uses the protocol
  (reads eventlog, reads beliefs) but isn't the protocol itself. Extraction path:
  command plugin once formats stabilize.

## Risks

1. **Event volume** — Measurement events are low-frequency (one per tool run, not
   per-query). Eventlog handles this trivially — it already stores thousands of
   high-frequency events (git commits, session observations, code facts).

2. **Metric schema evolution** — As tools add metrics, the JSON metrics object grows.
   This is fine: JSON is schema-flexible, and measurement events are append-only.
   Old events keep old fields. New events add new fields. The measure command handles
   both.

3. **Plugin trust** — Plugins can write arbitrary measurement events. The host
   validates: verb must be one of the 5 protocol verbs, metrics must be numeric,
   source is always the plugin name (plugins can't impersonate core). Malformed
   measurements are logged and skipped.

4. **Scope creep** — This spec is the measurement *system*. Individual verb
   enrichment (Phase 5 sub-items) should be separate focused specs. This spec
   builds the framework; those specs fill it.

5. **Eval history bootstrap** — Existing eval runs have no stored history. Phase 1
   starts fresh — history begins when the measurement events start flowing. No
   attempt to backfill.
