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
exit_criteria:
- Doctor plugin emits measurement via WIT record-measurement, events in eventlog
- Core measure::emit() helper exists and is used by at least one compiled-in tool
- All 5 protocol verbs have at least one measurement producer
- patina measure displays project health from both measure.* and existing events
- patina measure --system shows raw metrics with history
- MCP measure tool returns JSON health summary
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

3. **No unified consumer** — measurement data is scattered across `belief.surface`,
   `session.ended`, `scry.*`, `git.commit` with no single view that aggregates
   "how healthy is this project?" Existing events are correctly typed — the gap
   is a consumer that reads across them, not a new event type for what's already stored.

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

Added to `command`, `mother-child`, and `task` world imports. Not added to
`pipeline` (pure compute, no side effects).

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

#### Dual-Source Query Pattern

`patina measure` reads from TWO sources, not just `measure.*` events:

1. **`measure.*` events** — new events from tools that currently discard metrics
   (eval → `measure.search`, bench → `measure.search`, oxidize → `measure.index`,
   doctor plugin → `measure.capture`). These use the standard measurement schema.

2. **Existing typed events** — events that already carry measurement data under
   their own event types. These are NOT duplicated as `measure.*`:
   - `belief.surface` → believe verb (grounding, evidence, health metrics)
   - `session.ended` → evolve verb (commits, files_changed, beliefs_captured)
   - `scry.query` + feedback views → search verb (retrieval precision)
   - `git.commit` file counts → capture verb (coverage proxy)

This means existing tools don't need to change their event types. Scrape still
emits `belief.surface`, sessions still emit `session.ended`. The measurement
system adds NEW events where none exist, and READS existing events where they do.

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
in Phase 1, doctor's `run()` calls the WIT `record-measurement` function to emit
capture-freshness and foundation-health events. Doctor is a WASM plugin — it
uses the WIT interface, not the Rust `measure::emit()` helper (which is for
compiled-in core tools only). This distinction matters: doctor proves the plugin
path, eval/bench prove the core path.

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
- [ ] Add `measure` to task world imports
- [ ] Update `patina-sdk` crate with measurement convenience functions
- [ ] Core-side helper: `patina::measure::emit()` for compiled-in tools
- [ ] Migrate doctor plugin to use WIT measurement API (proof of concept)
- [ ] Add `host_measure` capability to manifest parsing

**Exit criteria:**
- [ ] Doctor plugin calls WIT `record-measurement`, events appear in eventlog
- [ ] Events have required schema: verb, tool, mode, metrics, source
- [ ] Host validates verb (one of 5), overrides source with plugin name
- [ ] `measure::emit()` compiles and is callable from core tool code
- [ ] `cargo build --release` succeeds, `patina doctor` emits measurement events
- [ ] Pre-push checks pass

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

**Exit criteria:**
- [ ] `patina eval` emits `measure.search` events (P@5, MRR persisted)
- [ ] `patina bench` emits `measure.search` events (Recall@K, latency persisted)
- [ ] `patina scrape` emits `measure.capture` events (files parsed, coverage)
- [ ] `patina oxidize` emits `measure.index` events (documents embedded, coverage)
- [ ] All 5 verbs have at least one event source (measure.* or existing typed events)
- [ ] Running existing tools leaves a measurement trail without workflow changes

### Phase 3 — Consumer Views (`patina measure`)

Build the read side. One command, two views, querying eventlog.

- [ ] `patina measure` — user view (default): project health, actions
- [ ] `patina measure --system` — maintainer view: raw metrics, history, trends
- [ ] `patina measure --json` — machine-readable output
- [ ] `patina measure --verb <name>` — drill-down with history
- [ ] MCP `measure` tool — user view as JSON
- [ ] Plugin measurements appear alongside core measurements

**Exit criteria:**
- [ ] `patina measure` renders user view from both measure.* and existing events
- [ ] `patina measure --system` renders maintainer view with raw metrics
- [ ] `patina measure --json` outputs machine-readable JSON
- [ ] MCP `measure` tool returns JSON health summary
- [ ] Empty state handled gracefully (no measurements yet)

### Phase 4 — Regression Detection

- [ ] Compare latest to previous for each tool+mode
- [ ] Configurable thresholds from project config
- [ ] Trend arrows in maintainer view
- [ ] `patina measure --ci` exits non-zero on regression

**Exit criteria:**
- [ ] Regressions detected by comparing latest to previous for each tool+mode
- [ ] Trend arrows visible in `--system` view
- [ ] `patina measure --ci` exits non-zero on regression
- [ ] Thresholds configurable via `[measure.<verb>]` in project config

### Future — Per-Verb Enrichment (separate sub-specs, not in scope)

These are separate specs that depend on Phases 1-3 being complete. Listed here
for visibility, not tracked as tasks in this spec:

- Capture: tree-sitter parse error tracking, per-language coverage
- Index: projection quality metric stored
- Evolve: pattern lifecycle tracking
- Search: per-project queryset generation with stored results

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
-- measure.* events exist for at least 3 verbs (search, capture, index are new emitters)
SELECT COUNT(DISTINCT json_extract(data, '$.verb')) FROM eventlog WHERE event_type LIKE 'measure.%';
expect: >= 3
label: new-measurement-producers-active
```

```verify
-- Existing event types still serve as measurement sources (not duplicated)
SELECT COUNT(*) FROM eventlog WHERE event_type = 'belief.surface';
expect: >= 1
label: existing-belief-measurements-intact
```

## Storage: Same Eventlog, Not a New One

Measurement events go into the existing `patina.db` eventlog table — same
append-only store everything else uses. No new database.

**Current state (2026-02-25):** `patina.db` is 260 MB total. The eventlog has
416K events, of which 97% (403K) are `code.*` events from scrape. Everything
else — git, sessions, beliefs, scry, forge — is 13K events (3%).

Measurement events are low-frequency: ~1 event per tool run, maybe 5-10 per
session. Over a year, a few thousand events. This is noise next to the code
analysis data that dominates the eventlog.

**If size becomes a concern later:**
1. **Rebuild** — `patina.db` is a rebuild cache. Delete it, re-scrape, fresh.
   This already works today.
2. **Compaction** — Future `patina eventlog compact --older-than 90d` could
   prune old events while keeping materialized views intact. Not needed now.
3. **Selective scrape** — The `code.*` events are the size driver (97%).
   Measurement events won't move the needle.

No special storage design needed. The eventlog handles this.

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
- 2026-02-25: Design clarification (session 20260225-173127):
  - Added phase-gated exit criteria (Phases 1-4) and frontmatter exit criteria.
  - Clarified dual-source read pattern: `patina measure` queries BOTH `measure.*`
    events AND existing typed events (belief.surface, session.ended, scry.*).
    Existing events are not duplicated — new measure.* events only for tools
    that currently discard metrics (eval, bench, oxidize, doctor).
  - Fixed doctor/WIT vs core/emit confusion: doctor uses WIT `record-measurement`,
    core tools use `measure::emit()`. Two paths, one event schema.
  - Added task world to WIT imports (command + mother-child + task, not pipeline).
  - Moved Phase 5 (per-verb enrichment) to "future sub-specs, not in scope."
  - Gap 3 reframed: not "no standard event type" but "no unified consumer."
  - Created DESIGN.md for Phase 1 implementation.
