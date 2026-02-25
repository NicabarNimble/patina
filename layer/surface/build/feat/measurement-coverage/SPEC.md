---
type: feat
id: measurement-coverage
status: draft
created: 2026-02-16
sessions:
  origin: 20260216-091624
related:
- layer/core/patina-identity.md
- spec/belief-truthfulness (archived, recoverable via git show)
- src/commands/eval/mod.rs
- src/commands/belief/mod.rs
- plugins/doctor/src/lib.rs
- wit/deps/patina-host/host.wit
beliefs:
- measure-the-measurement
- measure-first
- error-analysis-over-architecture
- plugins-are-three-prong-bundles
- mcp-is-shim-cli-is-product
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

### Data-Flow Contract

`patina measure` reads **only measurement events** from eventlog. It never reads
materialized tables (beliefs, function_facts, usearch index) directly. This keeps
the contract clean and makes replay semantics obvious.

**The rule:** Producers snapshot their summary metrics into measurement events at
run time. `patina measure` reads those snapshots. Detail commands (belief audit,
eval) read their own tables for per-item drill-down.

```
                         ┌───────────────────────┐
patina scrape ──────────►│ beliefs table          │◄──── patina belief audit
       │                 │ (per-belief detail)    │      (reads table directly)
       │                 └───────────────────────┘
       │
       └── measure.believe ──► eventlog ◄──── patina measure
           (summary snapshot)                 (reads events only)
```

**Concrete example for believe:** When scrape runs, it computes the beliefs table
(per-belief metrics as today) AND emits a `measure.believe` event containing the
summary: `{total: 130, stale: 4, floating: 2, median_health: 0.73, ...}`. When
the user runs `patina measure`, it reads that event — not the beliefs table.

**Consequence:** If you modify a belief file after scrape, `patina measure` shows
stale data until the next scrape. This is the same staleness behavior as
`patina belief audit` today — both depend on scrape to refresh.

**Why events-only for measure?**
- **History is free** — eventlog is append-only, so every scrape/eval/oxidize run
  creates a timestamped snapshot. Trend detection reads the event timeline.
- **Uniform contract** — all verbs work the same way. No special cases for "believe
  reads a table but search reads events."
- **Replay** — `patina measure --verb believe` always returns the same answer for the
  same eventlog state. No dependency on whether tables have been rebuilt.

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

### Doctor Coexistence

`patina doctor` keeps its own CLI and its own UX. It is not subsumed by
`patina measure`. The long-term relationship:

- **Doctor remains a distinct command** — users run `patina doctor` for environment
  health (tools, adapter, config). Its output format, exit codes, and WASM plugin
  contract are unchanged.
- **Doctor becomes the first measurement producer** — in Phase 1, doctor's `run()`
  calls `measure::record_measurement()` to emit capture-freshness and foundation-health
  events. Doctor is the proof of concept for the WIT measurement interface because
  it's already a WASM plugin. These events appear in `patina measure` alongside
  core measurements once Phase 3 ships.
- **Doctor is not invoked by measure** — `patina measure` reads doctor's stored
  measurement events. It does not call doctor at measurement time. Doctor runs on
  its own schedule (user-invoked or future: mother-daemon tick).

```
User runs:  patina doctor    → environment health check + emits measure events
User runs:  patina measure   → reads doctor's stored events in foundation section
```

This follows the producer/consumer split: doctor produces, measure consumes. Doctor
proving the plugin measurement API (Phase 1) is what validates that any third-party
plugin can do the same.

### MCP Exposure

`measure` becomes an MCP tool in Phase 3, alongside `scry`, `assay`, and `context`.
AI agents are a primary consumer of the project-user view — an LLM using Patina via
MCP should be able to check project health before making recommendations.

**MCP tool: `measure`**
- Returns the user view as structured JSON (same data as `patina measure --json`)
- Accepts optional `verb` parameter to filter to one verb
- Does NOT expose the maintainer view (`--system`) — agents don't need system
  coverage maps, and exposing them would leak implementation details into agent context
- Registered in the MCP server alongside existing tools

**Example MCP interaction:**
```json
{"tool": "measure", "params": {}}
→ {"verbs": {"capture": {"status": "measured", "health": "94%", ...}, ...}}

{"tool": "measure", "params": {"verb": "believe"}}
→ {"verb": "believe", "latest": {...}, "history": [...], "thresholds": {...}}
```

This follows [[mcp-is-shim-cli-is-product]]: the MCP tool wraps the CLI's user view
logic. No separate implementation.

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

#### First-Run / Empty State

When no measurement events exist (fresh project, never scraped):

```
$ patina measure

  Project Measurement Health

  VERB       STATUS     HEALTH     LAST RUN        ACTION NEEDED
  ─────      ──────     ──────     ────────        ─────────────
  capture    no data    —          never           run `patina scrape`
  index      no data    —          never           run `patina oxidize`
  search     no data    —          never           (available after scrape + oxidize)
  believe    no data    —          never           create beliefs in layer/surface/epistemic/beliefs/
  evolve     no data    —          never           start a session with /session-start

  No measurements recorded yet.

  Getting started:
    1. patina scrape        — capture your project's code and history
    2. patina oxidize       — build search indexes
    3. patina eval          — measure search quality
    4. patina belief audit  — review belief health

  Each command now records measurements automatically.
```

**Exit status:** 0. No data is not an error — it's an onboarding state. `--ci` mode
also exits 0 when no data exists (can't regress from nothing). `--ci` only exits
non-zero when a previously-recorded metric crosses a threshold.

Partial data is handled naturally: if only scrape has run, capture shows "measured"
and the rest show "no data". Each verb independently reflects its own state.

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

#### Verb Drill-Down: `patina measure --verb <name>`

Filters to one verb. Shows the verb's detail section from the appropriate view
(user or system), plus extended measurement history. Does not launch or replace
the verb's detail command — just shows what measure knows and points you there.

```
$ patina measure --verb believe

  Believe — Measurement Detail

  Latest (measure.believe from 2026-02-16 12:14):
    beliefs:         130 total
    healthy:         118 (health ≥ 0.4)
    stale:           4 (>30d)
    floating:        2
    verify-drift:    0
    median health:   0.73
    evidence rate:   79% verified

  History (last 5 snapshots):
    2026-02-16  scrape  total=130 stale=4  floating=2  median_health=0.73
    2026-02-15  scrape  total=129 stale=4  floating=3  median_health=0.71
    2026-02-14  scrape  total=129 stale=5  floating=3  median_health=0.70
    2026-02-12  scrape  total=127 stale=5  floating=3  median_health=0.69
    2026-02-10  scrape  total=125 stale=6  floating=4  median_health=0.68

    Trend: health ↑0.05, stale ↓2, floating ↓2 over 6 days

  Thresholds (from config or defaults):
    health_threshold:  0.4     (12 beliefs below)
    stale_days:        30      (from [beliefs] config)

  For per-belief detail: patina belief audit
  For stale beliefs:     patina belief audit --stale
  For grounding:         patina belief audit --grounding
```

**Accepted flags:**
- `--verb <name>` — required, one of: capture, index, search, believe, evolve
- `--history <N>` — number of historical snapshots to show (default: 5)
- `--system` — show maintainer-level detail (raw metric names, tool/mode breakdown)
- `--json` — machine-readable output

The `--verb` view is the middle level of the three-level hierarchy. It bridges the
system summary (`patina measure`) and the item-level detail command (e.g.,
`patina belief audit`). For verbs without a detail command (capture, index, evolve),
the `--verb` view is the deepest available view.

### What "Good" Looks Like — Per Verb

These thresholds define when a verb's status shows `covered` vs `partial` vs `gap`
for the maintainer view, and `healthy` vs `needs attention` for the user view.

#### Threshold Provenance

Thresholds ship as **compiled defaults** in the measure command. Project config
can override any threshold via `[measure.<verb>]` sections. This follows the same
pattern as `[beliefs].stale_days` — sensible default, project-overridable.

```toml
# .patina/config.toml — all fields optional, defaults shown

[measure.capture]
parse_success_good = 0.95      # above this = healthy
parse_success_warn = 0.90      # below this = needs attention
freshness_commits = 5          # commits behind HEAD before warning

[measure.index]
embedding_coverage_warn = 0.95 # below this = needs attention

[measure.search]
nl_p5_good = 0.60
nl_p5_warn = 0.40
nl_mrr_good = 0.50
nl_mrr_warn = 0.30
train_test_gap_max = 15.0      # pp, above = overfit warning

[measure.believe]
health_threshold = 0.4         # reuses belief-truthfulness semantics
# stale_days inherited from [beliefs].stale_days — not duplicated

[measure.evolve]
distillation_good = 0.30
distillation_warn = 0.15
stagnation_days = 30           # no growth for this long = warning
```

Phase 4 regression detection uses the same thresholds — `--ci` exits non-zero when
any metric crosses its `warn` boundary. No separate CI config needed.

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
| Low-health count | 0 beliefs < 0.4 | > 10% of beliefs | measure.believe event (snapshotted by scrape) |
| Verify-drifted count | 0 | Any | measure.believe event |
| Evidence verified rate | > 70% | < 50% | measure.believe event |
| Floating count | 0 | > 10% of beliefs | measure.believe event |
| Median activity age | < stale_days | > stale_days | measure.believe event |

#### Evolve

| Metric | Good | Needs attention | Source |
|--------|------|-----------------|--------|
| Session→belief distillation | > 30% | < 15% | session archive `## Beliefs Captured` counts |
| Entrenchment changes | ≥ 1/month | 0 for 60+ days | **new: scrape emits entrenchment-diff events** |
| Layer growth | Positive (net new beliefs) | Stagnant for 30+ days | layer file counts (core/surface/dust) |
| Orphaned patterns | 0 surface patterns > 90d without maturation or archive | > 5 | layer file `created` dates in frontmatter |

**Entrenchment change instrumentation (required for Phase 1):** Today's eventlog
has `belief.surface` events that are full-replace snapshots — no change history.
Scrape must compare the current entrenchment value in the belief file against the
previous `belief.surface` event's entrenchment field. When they differ, scrape
emits a `measure.evolve` event with `mode: "entrenchment-change"` and metrics:
`{belief_id: "...", old: "medium", new: "high"}`. This is cheap — scrape already
reads both the file and the previous event — and gives real maturation history.

Without this instrumentation, Phase 1 cannot credibly claim "all verbs have a
producer" for evolve. Distillation rate (from session archives) and layer growth
(from file counts) are available today. Entrenchment change detection is the one
piece of new instrumentation this spec requires.

## Implementation Phases

### Phase 1 — WIT Measurement Interface + Event Schema (The Bucket)

The foundation. Define the measurement contract and build the host-side
infrastructure that both core tools and WASM plugins use to report metrics.
Everything else in this spec is a producer or consumer of this interface.

**Why WIT first:** Everything in Patina is heading toward WIT/WASM plugins.
Spec commands, grammars, doctor — all plugins or becoming plugins. If we build
measurement emission into core tools first (old Phase 1) and add the plugin
interface later, we'd have two code paths: one for core, one for plugins.
Instead, build the bucket first. Core tools and plugins use the same interface
from day one.

- [ ] Define `measure.*` event_type convention in eventlog
- [ ] Define measurement event schema (verb, tool, mode, metrics, source)
- [ ] Add `patina:host/measure@0.1.0` interface to WIT
- [ ] Implement host-side: `record-measurement` validates and writes to eventlog
      (verb must be one of 5 protocol verbs, metrics must be numeric,
      source is always the plugin name — plugins can't impersonate core)
- [ ] Add `measure` to command world imports
- [ ] Add `measure` to mother-child world imports
- [ ] Update `patina-sdk` crate with measurement convenience functions
- [ ] Migrate doctor plugin to use measurement API (proof of concept —
      doctor is already a WASM plugin, making it the natural first producer)
- [ ] Core-side helper: `patina::measure::emit()` for compiled-in tools
      (writes the same event schema, source = "core")

**Exit criteria:** Doctor plugin writes measurements via WIT `record-measurement`.
Events land in eventlog with correct schema. `patina-sdk` has convenience
functions. Any plugin developer can emit measurements using only the SDK docs.
Core tools have a helper to emit the same event format.

### Phase 2 — Core Tool Emission

Wire existing tools into the measurement bucket. Each tool emits events
as a side effect of running — no new commands yet, just data flowing.

- [ ] `patina eval` writes measurement events after each run (all modes)
- [ ] `patina bench` writes measurement events after each run
- [ ] `patina scrape` writes capture measurement events (files parsed, functions
      extracted, coverage rate)
- [ ] `patina scrape` writes believe measurement events (belief summary snapshot:
      total, stale, floating, median_health)
- [ ] `patina oxidize` writes index measurement events (documents embedded,
      coverage, model used)
- [ ] `patina scrape` emits evolve measurement: entrenchment-change detection by
      comparing current belief file entrenchment to previous `belief.surface` event
- [ ] Session lifecycle writes evolve measurement events at session-end
      (distillation: did this session produce beliefs? layer file count deltas)

**Exit criteria:** All 5 verbs have at least one measurement event producer.
All events stored in eventlog with valid schema. Running existing tools now
leaves a measurement trail without any workflow changes.

### Phase 3 — Consumer Views (`patina measure`)

Build the read side. One command, two views, querying what's already in
the eventlog from Phases 1-2.

- [ ] `patina measure` reads measurement events only (not tables) — basic
      verb-by-verb summary with empty-state onboarding guidance
- [ ] `patina measure` (default) — user view: project health, natural language,
      action items, hides internals
- [ ] `patina measure --system` — maintainer view: full tool inventory, raw metrics,
      measurement history, regression detection
- [ ] `patina measure --json` — machine-readable output for both views
- [ ] `patina measure --verb <name>` — verb drill-down with history and thresholds
- [ ] `patina measure --verb <name> --history <N>` — configurable history depth
- [ ] MCP `measure` tool — wraps user view as JSON, optional verb parameter
- [ ] `patina measure` includes plugin-contributed measurements in both views

**Exit criteria:** Both views render correctly. User view uses health language
("good", "needs attention"). Maintainer view shows raw metrics and history.
`--verb` shows detail + history + links to detail commands. MCP tool returns
user view JSON. Plugin measurements appear alongside core measurements.

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

## Verification Plan

Per-phase verification. Each phase has concrete checks that must pass before the
phase is complete.

### Phase 1 Verification (WIT Interface + Schema)

```verify
-- Plugin-sourced measurement events exist
SELECT COUNT(*) FROM eventlog WHERE event_type LIKE 'measure.%' AND json_extract(data, '$.source') != 'core';
expect: >= 1
label: plugin-measurements-stored
```

```verify
-- Measurement events have required schema fields
SELECT COUNT(*) FROM eventlog WHERE event_type LIKE 'measure.%' AND (json_extract(data, '$.verb') IS NULL OR json_extract(data, '$.tool') IS NULL OR json_extract(data, '$.metrics') IS NULL);
expect: = 0
label: measurement-schema-valid
```

Manual verification:
- Doctor plugin emits measurement events via WIT `record-measurement`
- Events land in eventlog with correct schema (verb, tool, mode, metrics, source)
- SDK integration test: minimal plugin calls `record-measurement`, event appears
- Plugin cannot set `source: "core"` — host overrides with plugin name
- Core-side `patina::measure::emit()` helper writes same event format

### Phase 2 Verification (Core Tool Emission)

```verify
-- Every verb has at least one measurement event
SELECT COUNT(DISTINCT json_extract(data, '$.verb')) FROM eventlog WHERE event_type LIKE 'measure.%';
expect: = 5
label: all-verbs-have-producers
```

Manual verification:
- Scrape emits `measure.capture` and `measure.believe` events
- Oxidize emits `measure.index` events
- Eval emits `measure.search` events (at least one mode)
- Session-end emits `measure.evolve` events
- Scrape detects entrenchment changes and emits evolve measurement

### Phase 3 Verification (Consumer Views)

Manual verification:
- `patina measure` exits 0 and shows all 5 verbs
- `patina measure` on a fresh project (no events) shows onboarding guidance
- `patina measure --json` produces valid JSON with all 5 verbs
- User view contains NO raw metric names (P@10, MRR, co-retrieval) — uses
  "good", "needs attention", percentages
- Maintainer view (`--system`) contains raw metric names and tool/mode detail
- `--verb believe` shows history and thresholds
- `--verb` for a verb with no events shows "no data" not an error
- `--json` output for both views validates against a JSON schema
- MCP `measure` tool returns user view JSON (same structure as `--json`)
- Plugin measurements appear alongside core measurements in both views

### Phase 4 Verification

Manual verification:
- Inject a declining metric (manually write measure event with lower P@5)
- `patina measure --system` shows regression flag with delta
- `patina measure` (user view) shows action item about declining quality
- `--ci` exits non-zero when regression detected
- `--ci` exits 0 when no regression
- `--ci` exits 0 when no data (fresh project)
- Threshold overrides in config.toml are respected by `--ci`

### Phase 5 Verification

Per sub-spec — each verb enrichment defines its own verification. The system-level
check:

```verify
-- Every verb has at least 3 distinct metrics
SELECT json_extract(data, '$.verb') as verb, COUNT(DISTINCT k.key) as metric_count FROM eventlog, json_each(json_extract(data, '$.metrics')) as k WHERE event_type LIKE 'measure.%' GROUP BY verb HAVING metric_count < 3;
expect: = 0
label: all-verbs-have-depth
```

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

## Alignment Audit (2026-02-23, session 20260223-132543)

**Disposition: ALIGN (minor)**

Reviewed against spec-workflow-rigor architectural decisions. Best-aligned spec
in the tree — no conflicts, already has the three-layer pattern (CLI Phase 1-2,
MCP Phase 2, Plugin API Phase 3).

**Reference fixes:**
- `belief-truthfulness/SPEC.md` was completed and archived (tag: `spec/belief-truthfulness`).
  Updated `related:` to reference the git tag instead of the dead file path.
- Added actual code paths to `related:` for traceability.
- Added `plugins-are-three-prong-bundles` and `mcp-is-shim-cli-is-product` to beliefs.

**Code references verified:** All file paths and line numbers in the spec body
are accurate (eval/mod.rs, bench/mod.rs, belief/mod.rs, scrape/beliefs/*,
plugins/doctor, report/internal.rs, patina-identity.md, WIT host interfaces).

**Structural change (2026-02-25, session 20260225-143514):** Restructured phases
to lead with WIT interface as Phase 1 (was Phase 3). Rationale: everything in
Patina is heading toward WIT/WASM plugins. The measurement interface is the bucket
that all producers — core and plugin — emit into. Building the bucket first means
core tools and plugins use the same contract from day one, avoiding two code paths.
Doctor plugin (already WASM) is the Phase 1 proof of concept.
