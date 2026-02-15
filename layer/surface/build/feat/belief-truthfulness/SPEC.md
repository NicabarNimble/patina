---
type: feat
id: belief-truthfulness
status: active
created: 2026-02-15
sessions:
  origin: 20260215-083121
related:
- layer/core/patina-identity.md
beliefs:
- beliefs-are-the-product
- belief-identity-is-slug-not-hash
- measure-the-measurement
---

# feat: Belief Truthfulness — Staleness Detection and Health Scoring

> Beliefs go stale. The audit command shows today's metrics but can't detect
> drift over time. Add staleness detection, health scoring, and a `--stale`
> flag to surface beliefs that need attention.

## Problem

126 beliefs exist. `patina belief audit` computes per-belief metrics (citations,
evidence, verification, grounding) — but all metrics are point-in-time snapshots.
There is no mechanism to detect:

1. **Staleness** — a belief hasn't been cited, verified, or referenced in N months
2. **Drift** — a belief's verification queries used to pass but now fail
3. **Isolation** — a belief has no grounding connections and no citations (floating)
4. **Contradiction** — two beliefs with `attacks` relationships both active

The health_warnings system (`src/commands/belief/mod.rs:101-125`) catches some of
these today: `no-evidence`, `unverified`, `unused`, `no-applications`, `floating`,
`verify-contested`, `verify-error`. But it's all static — no temporal dimension.

## What Exists Today

### Belief audit (`src/commands/belief/mod.rs`)

`run_audit()` reads from the `beliefs` table, displays metrics in columnar format.
Sort modes: `use`, `truth`, `weak`, `grounding`. The `--warnings-only` flag filters
to beliefs with health warnings. The `--grounding` flag runs E4.6a semantic neighbor
search.

**`BeliefRow.health_warnings()`** returns static checks:
- `no-evidence` — evidence_count == 0
- `unverified` — evidence exists but none verified
- `unused` — no citations (by beliefs or sessions)
- `no-applications` — applied_in == 0
- `verify-contested` — verification queries failed
- `verify-error` — verification queries errored
- `floating` — no grounding connections

### Verification engine (`src/commands/scrape/beliefs/verification/`)

Full infrastructure: `VerificationQuery` (sql, assay, temporal types),
`VerificationResult` (pass/contested/error), `VerificationAggregates`.
Stored in `belief_verifications` table with data_freshness tracking.

Verification types:
- **SQL** — direct database queries (`internal/exec.rs`)
- **Assay** — structural queries via assay engine (`internal/assay.rs`)
- **Temporal** — co-change queries (`internal/temporal.rs`)
- **Safety** — query sandboxing, no writes allowed (`internal/safety.rs`)

### Belief metrics (`src/commands/scrape/beliefs/mod.rs:44-67`)

`BeliefMetrics` struct computes:
- Use: `cited_by_beliefs`, `cited_by_sessions`, `applied_in`
- Truth: `evidence_count`, `evidence_verified`, `defeated_attacks`, `external_sources`
- Grounding: `grounding_score`, `grounding_code_count`, `grounding_commit_count`,
  `grounding_session_count`, `grounding_forge_count`

### Belief schema (`src/commands/scrape/beliefs/mod.rs:70-100`)

```sql
beliefs (
    id, statement, persona, facets, confidence, entrenchment, status,
    extracted, revised, file_path,
    -- E4 metrics
    cited_by_beliefs, cited_by_sessions, applied_in,
    evidence_count, evidence_verified, defeated_attacks, external_sources, endorsed,
    -- E4.6a grounding
    grounding_score, grounding_code_count, grounding_commit_count,
    grounding_session_count, grounding_forge_count
)
```

## What To Build

### Phase A: Staleness Detection in Scrape

Add temporal awareness to belief metrics during scrape:

1. **Last-activity tracking** — new column `last_activity TEXT` on beliefs table.
   Computed as MAX of: file mtime, most recent session citation, most recent
   verification run, `revised` frontmatter date.

2. **Staleness threshold** — belief is "stale" if last_activity > 90 days ago.
   Configurable via `.patina/config.toml` `[beliefs] stale_days = 90`.

3. **Verification drift** — compare current verification results against
   `belief_verifications.data_freshness`. If a query that previously passed now
   fails, flag as "drifted". New column: `verification_drifted INTEGER DEFAULT 0`.

**Code path:** `src/commands/scrape/beliefs/mod.rs` — extend `BeliefMetrics` with
`last_activity` and `verification_drifted`. Add columns to `create_materialized_views()`.

### Phase B: Health Score

Compute a single 0.0-1.0 health score per belief from the existing metrics:

```
health = w_use * use_score + w_truth * truth_score + w_fresh * freshness_score

where:
  use_score    = min(1.0, (cited_by_beliefs + cited_by_sessions) / 3)
  truth_score  = evidence_verified / max(1, evidence_count)
  freshness    = 1.0 - min(1.0, days_since_activity / stale_days)
```

Weights: `w_use = 0.3, w_truth = 0.4, w_fresh = 0.3` (tunable).

Store as `health_score REAL` column on beliefs table. Add `--sort health` to
belief audit. Add `--stale` flag to filter beliefs where freshness < 0.3.

**Code path:** `src/commands/belief/mod.rs` — add sort mode, stale filter.
`src/commands/scrape/beliefs/mod.rs` — compute health_score during scrape.

### Phase C: Contradiction Detection

Detect when two active beliefs have `attacks` relationships and both are active:

1. During belief scrape, parse `## Attacks` and `## Attacked-By` sections
   (already parsed for `defeated_attacks` metric)
2. If both beliefs are `status: active` and the attack is not `defeated`,
   flag both as "contested"
3. New warning: `contested-by:{other-belief-id}`

**Code path:** `src/commands/scrape/beliefs/mod.rs` — after all beliefs parsed,
cross-reference attacks. New table or column for active attack pairs.

## Exit Criteria

1. `patina belief audit --stale` shows beliefs with no activity in 90+ days
2. `patina belief audit --sort health` ranks beliefs by computed health score
3. Verification drift is detected: previously-passing queries that now fail
4. Active attack pairs flagged in audit warnings

## Non-Goals

- Automated belief archival (user decides what to do with stale beliefs)
- LLM-based belief challenge generation
- Belief promotion/demotion automation
- Cross-project staleness (that's cross-project-beliefs spec territory)

## Evidence

| Claim | Source |
|-------|--------|
| 126 beliefs, all static metrics | `patina belief audit` output |
| health_warnings has 7 static checks | `src/commands/belief/mod.rs:101-125` |
| Verification stores data_freshness | `src/commands/scrape/beliefs/verification/mod.rs:93-101` |
| BeliefMetrics has no temporal fields | `src/commands/scrape/beliefs/mod.rs:44-67` |
| beliefs table has no last_activity | `src/commands/scrape/beliefs/mod.rs:70-100` |
| Verification types: sql, assay, temporal | `src/commands/scrape/beliefs/verification/internal/` |
