---
type: feat
id: data-measure-surface
status: ready
created: 2026-02-27
sessions:
  origin: 20260227-062333
related:
- data-architecture-v2
beliefs:
- measure-reads-tables-not-events
- events-are-autobiography-not-telemetry
- parse-at-boundary-type-the-interior
- correctness-by-construction-not-convention
- eventlog-is-infrastructure
- eventlog-is-truth
- measure-the-measurement
- structure-over-content-for-llm-tools
- llm-readable-code
- mcp-is-shim-cli-is-product
exit_criteria:
- id: measure-full-json-returns-structured-health-across-all-5-verbs
  text: '`patina measure --full --json` returns structured health JSON covering all 5 verbs with status, metrics, and diagnostics'
  checked: false
- id: each-catalog-question-answerable-from-measure-full-output
  text: each of the 9 catalog questions (DESIGN.md OQ#8) is answerable from `--full` output alone
  checked: false
- id: health-field-present-with-overall-project-health-summary
  text: top-level `health` field present with overall project health summary (status + one-sentence reason)
  checked: false
- id: mcp-measure-returns-full-json-for-llm-consumption
  text: '`mcp_measure()` returns the same `--full` JSON — MCP and CLI share one code path'
  checked: false
- id: temporal-fields-present-for-point-in-time-verbs
  text: each verb section includes `latest_timestamp`, `age_hours`, and freshness status
  checked: false
- id: belief-grounding-chain-visible-in-believe-verb
  text: believe verb exposes grounding breakdown — grounded count, floating count, contested count, avg evidence
  checked: false
- id: execute-feedback-rewritten-to-read-from-events-db-via-attach
  text: '`eval --feedback` rewritten to query events.db (scry.query) via ATTACH joined with patina.db (commits)'
  checked: false
- id: diagnostics-field-lists-actionable-problems
  text: each verb has a `diagnostics` array listing specific problems (not generic suggestions)
  checked: false
- id: no-serde-json-value-in-new-code
  text: zero `serde_json::Value` in new FullMeasureReport types — all fields are typed Rust structs/enums. Raw fallback only in existing VerbMetrics::Raw path.
  checked: false
- id: no-get-chains-in-new-code
  text: zero `.get().and_then().unwrap_or()` chains in new measure code — DB rows parse into typed structs at the query boundary via `from_db()` or `#[derive(Deserialize)]`
  checked: false
- id: diagnostics-derived-from-typed-structs
  text: diagnostic strings are computed from typed metric structs (e.g., `BelieveMetrics.floating_count`), never from raw JSON or ad-hoc DB queries
  checked: false
- id: health-summary-derived-from-typed-report
  text: '`health.summary` is generated from `FullMeasureReport` typed fields, not hand-assembled from separate queries'
  checked: false
---
# feat: Measure as LLM Query Surface — Structured Health for AI Consumers

> patina measure exists but answers a narrow question. An LLM cannot answer
> "is this project healthy?" with data-backed claims across all domains.
> Area 4 of [[data-architecture-v2]]. Blocked by Area 2 (data-emission-completeness).

## Problem

`patina measure` today returns a verb-status dashboard: 5 verbs, each
good/needs-attention/no-data, with the latest timestamp and a few metrics.
This is useful for a human glancing at terminal output. It is not useful for
an LLM that needs to reason about project health.

The gaps:

1. **No overall health judgment.** The LLM gets 5 independent statuses but no
   aggregate answer to "is this project healthy?" It has to synthesize across
   verbs itself — and it doesn't have the domain knowledge to weight them.

2. **No diagnostic detail.** "needs attention" tells you *something* is wrong
   but not *what*. An LLM needs: "believe verb: 23 floating beliefs (14% of
   total), highest-priority: `correctness-by-construction-not-convention`
   has 0 evidence links."

3. **No freshness signal.** Timestamps exist but no interpretation. Is 3 days
   old stale? For capture yes, for believe maybe not. The surface needs
   domain-aware freshness thresholds.

4. **Missing temporal context.** The architecture supports trend queries
   (events.db is append-only with timestamps) but measure doesn't expose them.
   "How has scrape performance changed?" requires manual SQL.

5. **Feedback evaluation is orphaned.** `eval --feedback` computes precision
   from scry.query events joined with git commit data. After the db-split,
   scry.query events live in events.db while commits live in patina.db. The
   ATTACH-based rewrite belongs here (DESIGN.md OQ#1).

6. **MCP and CLI diverge.** `mcp_measure()` wraps `build_report()` but
   `--json` serializes `MeasureReport` directly. Two code paths means two
   contracts — an LLM calling MCP gets a different shape than `--json`.

## Solution

### 1. Define the `--full` JSON contract

`patina measure --full --json` returns a single JSON document. This is the
LLM query surface — the contract an AI agent relies on.

```json
{
  "health": {
    "status": "good | needs_attention | degraded | no_data",
    "summary": "4/5 verbs healthy. believe: 23 floating beliefs need grounding.",
    "assessed_at": "2026-02-27T11:00:00Z"
  },
  "verbs": {
    "capture": {
      "status": "good",
      "latest_timestamp": "2026-02-27T10:45:00Z",
      "age_hours": 0.25,
      "freshness": "fresh",
      "sources": [ ... ],
      "diagnostics": []
    },
    "index": { ... },
    "search": { ... },
    "believe": {
      "status": "needs_attention",
      "latest_timestamp": "2026-02-27T09:00:00Z",
      "age_hours": 2.0,
      "freshness": "fresh",
      "sources": [{
        "source_type": "beliefs",
        "metrics": {
          "total_beliefs": 168,
          "grounded_count": 145,
          "floating_count": 23,
          "contested_count": 3,
          "avg_evidence": 2.4,
          "avg_health": 0.82
        }
      }],
      "diagnostics": [
        "23 beliefs have no code grounding (14% floating)",
        "3 beliefs have active attacks without resolution"
      ]
    },
    "evolve": { ... }
  },
  "event_counts": {
    "total_runtime_events": 142,
    "by_type": { "measure.capture": 12, "scry.query": 45, ... }
  }
}
```

**Design principles:**
- `health.status` is the aggregate: worst-verb-wins. If any verb is `degraded`,
  health is `degraded`. If any is `needs_attention` and none `degraded`,
  health is `needs_attention`.
- `health.summary` is a one-sentence natural language explanation. An LLM can
  quote it directly.
- Each verb is a key (not array) — `output.verbs.believe.status` is a stable
  path.
- `diagnostics` are specific and actionable — not "run patina scrape" but
  "capture verb: git scraper last ran 72h ago (threshold: 24h)".

### 2. Map catalog questions to queries

Each catalog question (DESIGN.md OQ#8) maps to a specific query or
combination:

| # | Question | Verb(s) | Query |
|---|----------|---------|-------|
| 1 | "Is this project healthy?" | all | `health.status` + `health.summary` |
| 2 | "Are beliefs grounded in code?" | believe | `beliefs` table: `grounding_score`, `evidence_count` |
| 3 | "Are tools running and capturing data?" | capture | events.db: `measure.capture` events by recency |
| 4 | "What's drifting?" | believe + capture | Freshness thresholds: beliefs `last_activity`, scrape_meta timestamps |
| 5 | "What changed since last session?" | evolve | `session.ended` events + git commit count since last session tag |
| 6 | "Which beliefs are contested?" | believe | `belief_attacks` table: beliefs with unresolved attacks |
| 7 | "Is the event stream flowing?" | capture | events.db: event count in last 24h, 7d, 30d windows |
| 8 | "How has scrape performance trended?" | capture | events.db: `measure.capture` duration_ms over last 10 runs (stretch) |
| 9 | "What knowledge is stale?" | believe + index | `beliefs.last_activity` age, embeddings `scrape_meta` age |

Questions 1-7 are point-in-time (minimum). Questions 8-9 are temporal
(stretch goal — simple window queries over events.db).

### 3. Freshness thresholds

Each verb has a domain-appropriate freshness window:

| Verb | Fresh | Aging | Stale | Rationale |
|------|-------|-------|-------|-----------|
| capture | < 24h | 24-72h | > 72h | Active project scrapes daily |
| index | < 48h | 48h-7d | > 7d | Embeddings drift slower than code |
| search | < 7d | 7-30d | > 30d | Eval quality degrades slowly |
| believe | < 7d | 7-30d | > 30d | Beliefs are long-lived knowledge |
| evolve | < 7d | 7-30d | > 30d | Sessions track active development |

`age_hours` is computed; `freshness` is derived from thresholds. These
thresholds are constants in the code, not configurable — measure is
opinionated about what "healthy" means.

### 4. Cross-database access pattern

Measure uses the ATTACH pattern (DESIGN.md cross-cutting decision):

```rust
let conn = Connection::open(PATINA_DB)?;
conn.execute("ATTACH DATABASE ?1 AS events", [EVENTS_DB])?;
// Query: SELECT ... FROM events.eventlog JOIN beliefs ...
```

This is already the pattern in `measure/internal.rs`. The `--full` path
extends it — no new access pattern needed.

### 5. Rewrite `execute_feedback()`

`eval --feedback` currently parses `git.commit` JSON blobs from the eventlog
to compute feedback precision. After db-split, the query becomes:

```sql
-- scry.query events from events.db (what did the LLM search?)
SELECT * FROM events.eventlog WHERE event_type = 'scry.query'
-- JOIN with structured commit data from patina.db (what files actually changed?)
JOIN commits ON ...
JOIN commit_files ON ...
```

This is a simplification: structured table reads replace JSON blob parsing.
The feedback precision metric (`P@5`) feeds into the `search` verb's
health status.

### 6. Unify MCP and CLI code paths

`mcp_measure()` and CLI `--full --json` must return identical JSON.
Single function: `build_full_report() -> FullMeasureReport`. Both paths
call it and serialize.

The existing `build_report() -> MeasureReport` remains for the default
user view (no `--full`). Two report types:
- `MeasureReport` — compact, for terminal display
- `FullMeasureReport` — comprehensive, for LLMs

## Pre-Implementation Review Concerns

These items were flagged during the data-architecture-v2 alignment review
(session 20260227-075037) and must be resolved before implementation begins:

1. **Pin VerbStatus enum.** Add `degraded` as a 4th status (currently: Good,
   NeedsAttention, NoData). Define derivation: degraded = multiple verbs
   failing or critical system down. needs_attention = one verb has issues.

2. **Define required vs optional fields.** The JSON example is illustrative,
   not normative. Write a field table with types and nullability in DESIGN.md
   before coding. Required per-verb: `status`, `latest_timestamp` (nullable),
   `age_hours` (nullable), `freshness` (nullable), `sources` (array, may be
   empty), `diagnostics` (array, may be empty).

3. **Clarify `execute_feedback()` ownership.** The rewrite lives in
   `src/commands/eval/mod.rs` (eval owns the computation). Measure reads the
   result (the precision metric) via the `search` verb's health status. The
   interface is a value, not a shared connection. Keep eval and measure as
   separate concerns with a data contract between them.

4. **ATTACH helper pattern.** Per Gjengset principle: if a function requires
   ATTACH, make it explicit at the call site. Extract `fn attach_events(conn)`
   helper. When a second consumer appears, promote to newtype wrapper. Don't
   build type machinery until reuse justifies it.

5. **`health.status` derivation rule.** Worst-verb-wins: if any verb is
   `degraded` → health is `degraded`. If any is `needs_attention` → health
   is `needs_attention`. All `good` → `good`. All `no_data` → `no_data`.

6. **Point-in-time vs temporal scope.** Questions 1-7 are v1 (point-in-time).
   Questions 8-9 (temporal trends) are stretch. Don't block v1 on temporal.

7. **MCP/CLI unification means shared function.** One Rust function
   `build_full_report(conn) -> Result<FullMeasureReport>`. Both CLI `--full
   --json` and `mcp_measure()` call it and serialize. Same binary, same
   library function, different serialization contexts. No env var or auth
   differences — patina is local.

## Type Safety Constraints (Anti-Soup)

The structural audit ([[session-20260301-090927]]) found 78 type soup
operations in `measure/internal.rs` — `.get()` chains, `.as_*()` casts,
`.unwrap_or()` fallbacks. The `measure-type-polish` spec (v0.35.7) cleaned
the external surface (typed enums for SourceType, ToolName, Mode, typed
VerbMetrics structs). This spec MUST NOT reintroduce soup.

### Rules

1. **No `serde_json::Value` in new types.** `FullMeasureReport`,
   `HealthSummary`, `FullVerbSummary`, `Diagnostic` — all fields must be
   concrete Rust types. The only acceptable `Value` is the existing
   `VerbMetrics::Raw` fallback for unrecognized legacy shapes.

2. **No `.get("key")` chains.** All DB row parsing goes through either
   `#[derive(Deserialize)]` structs or explicit `from_db()` methods with
   typed dispatch (the existing pattern in `VerbMetrics::from_db()`).

3. **Diagnostics are computed, not assembled.** A diagnostic like
   "23 beliefs have no code grounding (14% floating)" must be generated
   from `BelieveMetrics { floating_count: 23, total_beliefs: 168, .. }`,
   not from `row.get("floating")?.as_i64().unwrap_or(0)`.

   ```rust
   // GOOD: diagnostic from typed struct
   impl BelieveMetrics {
       fn diagnostics(&self) -> Vec<Diagnostic> {
           let mut diags = vec![];
           if self.floating_count > 0 {
               let pct = (self.floating_count as f64 / self.total_beliefs as f64) * 100.0;
               diags.push(Diagnostic {
                   severity: Severity::Warning,
                   message: format!(
                       "{} beliefs have no code grounding ({:.0}% floating)",
                       self.floating_count, pct
                   ),
               });
           }
           diags
       }
   }

   // BAD: diagnostic from raw JSON
   let floating = data.get("floating_count").and_then(|v| v.as_i64()).unwrap_or(0);
   let total = data.get("total_beliefs").and_then(|v| v.as_i64()).unwrap_or(1);
   diagnostics.push(format!("{} beliefs floating", floating));
   ```

4. **`health.summary` is derived, not assembled.** The summary string
   is generated by a method on `FullMeasureReport` that reads its own
   typed fields — not by a separate function querying the DB.

   ```rust
   // GOOD: summary from typed report
   impl FullMeasureReport {
       fn health_summary(&self) -> String {
           let healthy = self.verbs.values().filter(|v| v.status == VerbStatus::Good).count();
           let worst = self.worst_verb();
           format!("{}/{} verbs healthy. {}", healthy, self.verbs.len(), worst.one_line_reason())
       }
   }
   ```

5. **ATTACH query results parse into typed structs.** The
   `execute_feedback()` rewrite uses cross-database JOINs. Row results
   must parse into a typed struct (e.g., `FeedbackRow { query: String,
   timestamp: String, hit_files: Vec<String>, commit_files: Vec<String> }`)
   at the query boundary. No raw row indexing downstream.

### Pre-existing typed infrastructure to build on

These types from `measure-type-polish` (v0.35.7) are the foundation:

- `SourceType` — 8 variants with `from_verb()` boundary parser
- `ToolName` — 6 variants with `from_db_str()` boundary parser
- `Mode` — 9 variants with `from_db_str()` boundary parser
- `VerbMetrics` — typed per-verb metrics with `from_db()` dispatch
- `VerbSummary`, `SourceSummary`, `MeasureReport` — typed report structs

New types extend these. `FullMeasureReport` wraps the existing
`MeasureReport` data plus the new health/freshness/diagnostics layer.
No parallel type hierarchy — one builds on the other.

## Non-Goals

- **Custom thresholds.** Freshness windows are hardcoded constants. Measure
  is opinionated. Configuration adds complexity for zero demonstrated need.
- **Historical trend visualization.** Trend data is in the JSON for LLMs.
  Human-facing trend charts (sparklines, ASCII plots) are out of scope.
- **Alert/notification system.** Measure reports; it doesn't push. Hooks
  and monitoring belong elsewhere.
- **Per-belief drill-down.** The believe verb reports aggregates. Individual
  belief health is `patina scry --belief <id>`, not measure's job.
- **New event types.** This spec consumes events that Area 2 produces.
  No new `measure::emit()` calls.
