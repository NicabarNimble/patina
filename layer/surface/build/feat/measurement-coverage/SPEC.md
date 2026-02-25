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
- src/eventlog.rs
- src/commands/eval/mod.rs
- src/commands/belief/mod.rs
- src/commands/scry/internal/logging.rs
- plugins/doctor/src/lib.rs
- wit/deps/patina-host/host.wit
beliefs:
- measure-the-measurement
- measure-first
- error-analysis-over-architecture
- plugins-are-three-prong-bundles
- mcp-is-shim-cli-is-product
- eventlog-is-truth
- eventlog-is-infrastructure
exit_criteria: []
---

# feat: Measurement Coverage System

> Patina's observability infrastructure. A standard way for core tools and WASM
> plugins to report "does this work?" metrics into the eventlog. One read command
> (`patina measure`) to see it all. The bucket comes first, producers and views
> follow.

## Problem

Patina's five protocol verbs (**capture, index, search, believe, evolve**) have
wildly uneven measurement. Some tools compute metrics and throw them away. Others
emit rich events but nobody aggregates them. Plugins can't report metrics at all.

**What's measured but lost:**
- `eval` computes P@5, P@10, MRR — prints to stdout, gone
- `bench` computes performance numbers — prints to stdout, gone
- `belief audit` computes health/staleness/grounding — reads beliefs table, no history

**What's measured and stored (but not called "measurement"):**
- `scry.query` + `scry.use` + `scry.feedback` events power the feedback loop views —
  real-world retrieval precision IS measurement, it's just not labeled as such
- `belief.surface` events carry metrics (grounding counts, evidence, citations,
  verification stats) — belief health IS measurement
- `session.ended` events carry commits_made, files_changed, beliefs_captured,
  patterns_modified — session productivity IS measurement

**What's not measured at all:**
- Capture: scrape runs, quality unquantified (parse success rate? coverage?)
- Index: oxidize builds embeddings, quality unknown (coverage? freshness?)
- Evolve: knowledge maturation invisible (distillation rate? layer growth?)

**What plugins can't do:**
- Doctor plugin is read-only — can't report what it observes
- Future plugins (grammars, analyzers) have no way to send metrics back to the host

The problem isn't "we don't measure" — it's "measurements go nowhere usable."

## Real-World Model: OpenTelemetry

This isn't a novel design. Patina's measurement system maps directly to the
OpenTelemetry (OTel) observability pattern, which is the industry standard:

| OTel Concept | Patina Equivalent |
|---|---|
| Metrics API (instrumentation) | WIT `measure` interface (for plugins) |
| SDK (emission helper) | `patina::measure::emit()` (for core tools) |
| Collector (storage) | Eventlog (append-only, already exists) |
| Exporter/Backend (views) | `patina measure` CLI, MCP tool, future TUI dashboard |
| Auto-instrumentation | Core tools emitting as side effect of running |
| Manual instrumentation | Plugins calling WIT `record-measurement` |

OTel is the compass, not the blueprint. We don't import OTel crates or follow its
protocol. We use our existing eventlog as the collector and WIT as the plugin API.
The insight is structural: **API and collector are separate from backends.**

## What Exists Today — Ground Truth

### The Eventlog (the pipe that already works)

`src/eventlog.rs` — Append-only SQLite table, LiveStore pattern:

```sql
CREATE TABLE eventlog (
    seq INTEGER PRIMARY KEY AUTOINCREMENT,
    event_type TEXT NOT NULL,
    timestamp TEXT NOT NULL,
    source_id TEXT NOT NULL,
    source_file TEXT,
    data TEXT NOT NULL  -- JSON, validated
);
```

One write API: `eventlog::insert_event(conn, event_type, timestamp, source_id, source_file, data)`

4 indexes (type, timestamp, source, type+time). Everything in Patina that persists
goes through here. This is the collector — it's built, it works, it's fast.

### Who Emits Events Today

| Emitter | Event Types | To Eventlog? |
|---|---|---|
| scrape git | `git.commit`, `git.tag` | YES |
| scrape layer | `pattern.core/topics/projects` | YES |
| scrape beliefs | `belief.surface` (with metrics) | YES |
| scrape forge | `forge.issue`, `forge.pr` | YES |
| session lifecycle | `session.started/update/observation/ended` | YES |
| scry | `scry.query`, `scry.use`, `scry.feedback` | YES |
| **eval** | — | **NO** (stdout only) |
| **bench** | — | **NO** (stdout only) |
| **belief audit** | — | **NO** (reads table, no emit) |
| **oxidize** | — | **NO** (builds indexes silently) |
| **doctor plugin** | — | **NO** (read-only, no write API) |

### WIT Interfaces Available to Plugins Today

From `wit/deps/patina-host/host.wit`:

- `patina:host/log` — structured logging (debug, info, warn, error)
- `patina:host/layer` — read project data (config, files, tools, environment)
- `patina:host/query` — run scry/context/assay queries
- `patina:host/schema` — fact schema introspection
- `patina:host/http` — domain-allowlisted HTTP (mother-child only)

**Missing: no event emission API.** Plugins cannot write to the eventlog.
This is the fundamental gap.

### Measurement That Already Happens (Unlabeled)

**Feedback loop views** (`src/eventlog.rs:166-302`):
6 SQL views join `scry.query` + `scry.use` + `git.commit` to compute real-world
retrieval precision. `eval --feedback` reads these. This IS measurement — query
results matched against actual commits to see if search is useful.

**Belief surface events:**
`belief.surface` events carry per-belief metrics: grounding counts, evidence count,
evidence verified, citations, verification stats. Scraped from YAML frontmatter +
computed during belief scrape. These are health measurements.

**Session ended events:**
`session.ended` carries: commits_made, files_changed, beliefs_captured,
patterns_modified, lines_changed. Session productivity measurement, already stored.

### The Three Gaps

1. **No write API for plugins** — WIT has `log`, `layer`, `query`, `schema`, `http`
   but no `measure` or `emit-event`. Plugins are read-only.

2. **Core tools that compute metrics don't emit** — eval, bench, oxidize, belief audit
   all compute valuable numbers and throw them away.

3. **No standard event type for measurements** — existing measurement-like data is
   scattered across `belief.surface`, `session.ended`, `scry.*` with no convention
   that says "this is a measurement event."

## Design

### `measure` Is a Core Plugin

In Obsidian terms: `measure` is a **core plugin** — infrastructure that ships with
the system and provides a capability other plugins depend on. It's not optional.

It has two sides:
- **Write side** — the WIT interface + core helper. The bucket that data flows into.
- **Read side** — `patina measure` command. The view over the bucket.

### Write Side: How Metrics Get In

#### For WASM plugins — WIT interface

```wit
/// Measurement reporting for plugins.
///
/// Plugins call this to record metrics. The host writes the
/// measurement event to eventlog with the plugin's name as source.
interface measure {
    record-measurement: func(
        verb: string,
        tool: string,
        mode: string,
        metrics-json: string,
    ) -> result<_, string>;
}
```

Host validates: verb is one of 5 protocol verbs, metrics are numeric JSON,
source is always the plugin name (host overrides — plugins can't impersonate core).

Added to both `command` and `mother-child` world imports.

#### For compiled-in core tools — Rust helper

```rust
// src/measure.rs (new module)
pub fn emit(
    conn: &Connection,
    verb: &str,        // capture, index, search, believe, evolve
    tool: &str,        // eval, scrape, oxidize, etc.
    mode: &str,        // nl, feedback, ablation, etc.
    metrics: &serde_json::Value,
) -> Result<()> {
    eventlog::insert_event(
        conn,
        &format!("measure.{}", verb),
        &Utc::now().to_rfc3339(),
        &format!("{}:{}", tool, mode),
        None,
        &serde_json::json!({
            "verb": verb,
            "tool": tool,
            "mode": mode,
            "metrics": metrics,
            "source": "core"
        }).to_string(),
    )
}
```

Same event schema whether it comes from a plugin or core. One format in eventlog.

### Measurement Event Schema

All measurement events use `event_type = "measure.<verb>"`:

```json
{
  "event_type": "measure.search",
  "timestamp": "2026-02-25T14:30:00Z",
  "source_id": "eval:nl",
  "data": {
    "verb": "search",
    "tool": "eval",
    "mode": "nl",
    "metrics": {
      "p_at_5": 0.72,
      "p_at_10": 0.48,
      "mrr": 0.672,
      "query_count": 52
    },
    "source": "core"
  }
}
```

Fields:
- **verb** — one of: capture, index, search, believe, evolve
- **tool** — which tool produced this (eval, scrape, oxidize, doctor, grammar-rust)
- **mode** — tool-specific sub-mode (nl, feedback, ablation, freshness-check)
- **metrics** — key-value pairs, all numeric (f64 or i64)
- **source** — "core" for compiled-in tools, plugin name for WASM plugins

History is free — eventlog is append-only. Every tool run creates a timestamped
snapshot. Trend detection reads the event timeline.

### Read Side: `patina measure`

One command, two views:

**User view** (default) — "Is Patina understanding my project?"
- Health language: "good", "needs attention", percentages
- Action items inline: "run `patina scrape`", "2 beliefs stale"
- Hides internals — no P@10, MRR, raw metric names
- Plugin contributions visible but not prominent

**Maintainer view** (`--system`) — Raw metrics, tool inventory, history, trends
- Shows P@5, MRR, co-retrieval rates
- Shows measurement history with trend arrows
- Regression detection: compares latest to previous
- Plugin measurements as distinct section

**MCP tool** — `measure` returns user view as JSON. LLMs check project health
before making recommendations.

**Empty state** — "No measurements recorded yet. Getting started: 1. patina scrape..."

### Consumer Detail

Detailed mock outputs for both views are preserved in git history
(see `git show HEAD~1:layer/surface/build/feat/measurement-coverage/SPEC.md`
for the full CLI mockups). The view design is stable — what changed is the
phase ordering and grounding in current infrastructure.

### What "Good" Looks Like — Thresholds

Thresholds ship as compiled defaults, overridable via `[measure.<verb>]` config:

| Verb | Metric | Good | Needs Attention |
|---|---|---|---|
| Capture | Parse success | > 95% | < 90% |
| Capture | Freshness | ≤ 1 commit behind | > 5 commits behind |
| Index | Embedding coverage | 100% | < 95% |
| Search | NL P@5 | > 60% | < 40% |
| Search | Train-test gap | < 10pp | > 15pp |
| Believe | Low-health count | 0 below 0.4 | > 10% of beliefs |
| Believe | Floating count | 0 | > 10% of beliefs |
| Evolve | Distillation rate | > 30% | < 15% |
| Evolve | Layer stagnation | Active growth | No growth 30+ days |

### Doctor Coexistence

Doctor remains its own command. Doctor becomes the **first measurement producer** —
in Phase 1, doctor's `run()` calls `measure::record_measurement()` to emit
capture-freshness and foundation-health events. Doctor is the proof of concept
for the WIT interface because it's already a WASM plugin.

`patina measure` reads doctor's stored events. It does not invoke doctor.
Producer/consumer split: doctor produces, measure consumes.

## Implementation Phases

### Phase 1 — The Bucket (WIT Interface + Event Schema)

The foundation. Define the measurement contract and build the host-side
infrastructure that both core tools and WASM plugins use to report metrics.

**Why WIT first:** Everything in Patina is heading toward WIT/WASM plugins.
If we build core emission first and add the plugin interface later, we'd have
two code paths. Build the bucket first — one contract, used everywhere.

- [ ] Define `measure.*` event_type convention in eventlog
- [ ] Define measurement event schema (verb, tool, mode, metrics, source)
- [ ] Add `patina:host/measure@0.1.0` interface to WIT
- [ ] Implement host-side: `record-measurement` validates and writes to eventlog
- [ ] Add `measure` to command world imports
- [ ] Add `measure` to mother-child world imports
- [ ] Update `patina-sdk` crate with measurement convenience functions
- [ ] Core-side helper: `patina::measure::emit()` for compiled-in tools
- [ ] Migrate doctor plugin to use measurement API (proof of concept)

**Exit criteria:** Doctor plugin writes measurements via WIT `record-measurement`.
Events land in eventlog with correct schema. Core tools have `emit()` helper.

### Phase 2 — Core Tool Emission

Wire existing tools into the bucket. Each tool emits events as a side effect
of running — no new commands yet, just data flowing.

- [ ] `patina eval` emits measurement events after each run (all modes)
- [ ] `patina bench` emits measurement events after each run
- [ ] `patina scrape` emits capture measurements (files parsed, functions, coverage)
- [ ] `patina scrape` emits believe measurements (total, stale, floating, median_health)
- [ ] `patina oxidize` emits index measurements (documents embedded, coverage, model)
- [ ] `patina scrape` emits evolve: entrenchment-change detection
- [ ] Session lifecycle emits evolve measurements at session-end

**Exit criteria:** All 5 verbs have at least one measurement producer.
Running existing tools leaves a measurement trail without workflow changes.

### Phase 3 — Consumer Views (`patina measure`)

Build the read side. One command, two views, querying eventlog.

- [ ] `patina measure` — user view (default): project health, actions
- [ ] `patina measure --system` — maintainer view: raw metrics, history, trends
- [ ] `patina measure --json` — machine-readable output
- [ ] `patina measure --verb <name>` — drill-down with history
- [ ] MCP `measure` tool — user view as JSON
- [ ] Plugin measurements appear alongside core measurements

**Exit criteria:** Both views render correctly. MCP tool works.

### Phase 4 — Regression Detection

- [ ] Compare latest to previous for each tool+mode
- [ ] Configurable thresholds from project config
- [ ] Trend arrows in maintainer view
- [ ] `patina measure --ci` exits non-zero on regression

**Exit criteria:** Regressions detected and surfaced in both views.

### Phase 5 — Per-Verb Enrichment (separate sub-specs)

- [ ] Capture: tree-sitter parse error tracking, per-language coverage
- [ ] Index: projection quality metric stored
- [ ] Evolve: pattern lifecycle tracking
- [ ] Search: per-project queryset generation with stored results

### Future — TUI Dashboard

The btop-style dashboard dream: a `ratatui` TUI that reads `measure.*` events
from the eventlog and renders live widgets. This doesn't need any special
infrastructure — it needs Phases 1-2 done right (good events flowing). The TUI
is just another backend/view, like `patina measure` is a CLI view and the MCP
tool is a JSON view. All read the same eventlog.

## Verification

### Phase 1

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

### Phase 2

```verify
-- Every verb has at least one measurement event
SELECT COUNT(DISTINCT json_extract(data, '$.verb')) FROM eventlog WHERE event_type LIKE 'measure.%';
expect: = 5
label: all-verbs-have-producers
```

### Phase 5

```verify
-- Every verb has at least 3 distinct metrics
SELECT json_extract(data, '$.verb') as verb, COUNT(DISTINCT k.key) as metric_count FROM eventlog, json_each(json_extract(data, '$.metrics')) as k WHERE event_type LIKE 'measure.%' GROUP BY verb HAVING metric_count < 3;
expect: = 0
label: all-verbs-have-depth
```

## Risks

1. **Event volume** — Measurement events are low-frequency (one per tool run).
   Eventlog handles thousands of high-frequency events already.

2. **Metric schema evolution** — JSON is schema-flexible. Old events keep old
   fields, new events add new. Append-only handles this naturally.

3. **Plugin trust** — Host validates verb, requires numeric metrics, overrides
   source with plugin name. Malformed measurements logged and skipped.

4. **Scope creep** — This spec is the measurement *system*. Per-verb enrichment
   (Phase 5) should be separate specs. Build the framework, let focused specs fill it.

5. **Eval history bootstrap** — Existing eval runs have no stored history. History
   begins when measurement events start flowing. No backfill attempt.

## Revision Log

- 2026-02-16: Created — original 5-phase design
- 2026-02-23: Alignment audit — minor reference fixes, code paths verified
- 2026-02-25: Major restructure (session 20260225-143514):
  - WIT interface promoted to Phase 1 (was Phase 3). Rationale: everything is
    heading toward WASM plugins — the bucket must exist before producers.
  - Added "Ground Truth" section: full inventory of current eventlog, WIT
    interfaces, what emits today, what doesn't, the three concrete gaps.
  - Added OTel model reference (compass not blueprint).
  - Identified unlabeled measurement that already exists (feedback views,
    belief.surface metrics, session.ended stats).
  - Trimmed detailed CLI mockups (preserved in git history).
  - Reframed as "core plugin" (Obsidian model) — infrastructure that ships
    with the system, not optional.
  - TODO: split spec apart and add more organization in next session.
