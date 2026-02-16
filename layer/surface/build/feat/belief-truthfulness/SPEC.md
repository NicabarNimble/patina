---
type: feat
id: belief-truthfulness
status: active
created: 2026-02-15
sessions:
  origin: 20260215-083121
  amended: 20260216-064229
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

1. **Last-activity tracking** — four nullable component columns on the beliefs
   table, plus a computed `last_activity` column as their MAX:

   - `last_file_touch TEXT` — `std::fs::metadata(path).modified()` during
     `parse_belief_file()` (free — already reading the file)
   - `last_frontmatter_revision TEXT` — from `revised` YAML field (already parsed
     at `scrape/beliefs/mod.rs:257-263`)
   - `last_session_citation TEXT` — during `cross_reference_beliefs()` (line 528-541),
     the code already iterates session files checking `contains(bid)`. Session
     filenames ARE timestamps (`20260215-230959.md`). Track the MAX filename per
     belief — zero extra I/O, just a string comparison in the existing loop.
   - `last_verification_run TEXT` — from `belief_verifications.last_run_at` (already
     stored at `verification/internal/exec.rs:299`)
   - `last_activity TEXT` — MAX of the four above, computed after all signals collected.

   Storing pre-MAX components keeps audit output explainable (future `--show-activity`
   can display why a belief is stale) and avoids re-deriving during display.

2. **Staleness threshold** — belief is "stale" if last_activity > N days ago.
   Configurable via `.patina/config.toml` `[beliefs] stale_days = 90`. Audit
   summary emits freshness stats: "32/128 beliefs stale (>90d), median age 143d"
   for immediate signal without hardcoding policy.

3. **Verification drift** — snapshot-before-drop approach.

   **Constraint:** `create_tables()` (`verification/internal/exec.rs:257`) does
   `DROP TABLE IF EXISTS belief_verifications` on every scrape. There are no
   "previous results" to compare against — `data_freshness` is just the string
   "full" or "incremental", not a temporal marker.

   **Implementation:** Before the DROP, rename the table:
   ```sql
   ALTER TABLE belief_verifications RENAME TO belief_verifications_prev;
   ```
   After the new verification run completes, diff:
   ```sql
   SELECT p.belief_id, p.label
   FROM belief_verifications_prev p
   JOIN belief_verifications c ON p.belief_id = c.belief_id AND p.label = c.label
   WHERE p.last_status = 'pass' AND c.last_status != 'pass'
   ```
   Flag matched beliefs with `verification_drifted = 1`. Drop the `_prev` table.

   This delivers drift detection with minimal pipeline change. An append-only
   history table is deferred until there's proven need for trend analysis.

   New column: `verification_drifted INTEGER DEFAULT 0` on beliefs table.

**Code paths:**
- `src/commands/scrape/beliefs/mod.rs` — extend `BeliefMetrics` with four
  `last_*` fields plus `last_activity` and `verification_drifted`. Add columns
  to `create_materialized_views()`. Populate `last_file_touch` in
  `parse_belief_file()`, `last_session_citation` in `cross_reference_beliefs()`,
  `last_frontmatter_revision` from existing `revised` field.
- `src/commands/scrape/beliefs/verification/internal/exec.rs` — in
  `create_tables()`, rename before drop. After `run_verification_queries()`
  loop in `beliefs/mod.rs`, run drift diff query and set flags.

### Phase B: Health Score

Compute a single 0.0-1.0 health score per belief from the existing metrics:

```
health = w_use * use_score + w_truth * truth_score + w_fresh * freshness_score

where:
  use_score    = min(1.0, (cited_by_beliefs + cited_by_sessions) / 3)
  truth_score  = evidence_verified / max(1, evidence_count)
  freshness    = 1.0 - min(1.0, days_since_activity / stale_days)
```

Weights: `w_use = 0.3, w_truth = 0.4, w_fresh = 0.3` (tunable via config, not CLI).
Linear freshness curve — don't optimize the math until real score distributions
are observed.

Store as `health_score REAL` column on beliefs table. Compute during scrape,
expose via:
- `--sort health` sort mode in belief audit
- `--stale` flag to filter beliefs where freshness < 0.3
- `low-health` warning in `health_warnings()` when health_score < 0.4

Compute and expose only — health score does not gate any automated action.
Collect feedback from real audit usage before refining weights or adding
nonlinear curves.

**Code path:** `src/commands/belief/mod.rs` — add sort mode, stale filter,
low-health warning. `src/commands/scrape/beliefs/mod.rs` — compute
health_score during scrape after all signals collected (Phase A fields required).

### Phase C: Contradiction Detection

Detect when two active beliefs have `attacks` relationships and both are active:

1. During belief scrape, `extract_file_metrics()` (`scrape/beliefs/mod.rs:323-361`)
   already parses `## Attacked-By` sections and counts defeated attacks. Extend
   it to also collect non-defeated attacker IDs into a new `BeliefMetrics` field:
   `attacked_by_ids: Vec<String>`.

2. In `cross_reference_beliefs()`, after all beliefs are parsed, check each
   `attacked_by_ids` entry: if the attacking belief is also `status: active`
   and the attack is not `defeated`, flag both as "contested".

3. New column `contested_by TEXT` on beliefs table (comma-separated attacker IDs).
   Populated during scrape's cross-reference pass. New warning in
   `health_warnings()`: `contested-by:{other-belief-id}`, read from this column
   at display time. Uses the existing ALTER TABLE migration pattern
   (`scrape/beliefs/mod.rs:131-152`).

**Code path:** `src/commands/scrape/beliefs/mod.rs` — extend
`extract_file_metrics()` to collect attacker IDs. Add cross-reference pass
in `cross_reference_beliefs()`. `src/commands/belief/mod.rs` — add
`contested-by:` warning to `health_warnings()`.

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
| 128 beliefs, all static metrics | `patina belief audit` output |
| health_warnings has 7 static checks | `src/commands/belief/mod.rs:101-125` |
| BeliefMetrics has no temporal fields | `src/commands/scrape/beliefs/mod.rs:44-67` |
| beliefs table has no last_activity | `src/commands/scrape/beliefs/mod.rs:70-100` |
| Verification types: sql, assay, temporal | `src/commands/scrape/beliefs/verification/internal/` |
| **belief_verifications is DROP+CREATE every scrape** | `verification/internal/exec.rs:257-258` |
| data_freshness is "full"/"incremental", not temporal | `scrape/beliefs/mod.rs:1047` + `exec.rs:299` |
| `revised` frontmatter already parsed | `scrape/beliefs/mod.rs:257-263` |
| cross_reference_beliefs reads all session files | `scrape/beliefs/mod.rs:528-541` |
| Session filenames are timestamps (YYYYMMDD-HHMMSS.md) | `layer/sessions/` directory convention |
| extract_file_metrics parses Attacked-By for defeated count | `scrape/beliefs/mod.rs:350-354` |

### Amendment History

- **2026-02-16** (session 20260216-064229): Deep code read revealed belief_verifications
  is DROP+CREATE every scrape — original drift detection approach was impossible.
  Amended Phase A with snapshot-before-drop strategy. Added 4 component columns for
  last_activity explainability. Fixed Phase C to reuse existing parsing infrastructure
  (reuses parsing, adds one `contested_by TEXT` column). Added `low-health` warning
  to Phase B. Grounded all claims against actual line numbers. UX review confirmed
  terminal width constraint: show `last_activity` in main table, component columns
  behind future `--verbose` flag only.
