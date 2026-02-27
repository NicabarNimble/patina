---
type: refactor
id: data-mother-schema
status: active
created: 2026-02-26
sessions:
  origin: 20260226-124149
related:
- data-architecture-v2
beliefs:
- if-its-patina-its-git
exit_criteria:
- id: graph-db-beliefs-table-includes-grounding-score-grounding-count-verification-last-activity-columns
  text: graph.db beliefs table includes grounding_score, grounding_*_count, verification_*, last_activity columns
  checked: false
- id: graph-sync-reads-and-syncs-all-new-columns-from-project-patina-db
  text: graph sync reads and syncs all new columns from project patina.db
  checked: false
- id: dangling-edges-auto-cleaned-during-sync-not-just-warned
  text: dangling edges auto-cleaned during sync (not just warned)
  checked: false
- id: belief-applied-in-queryable-via-patina-mother-search-results
  text: belief_applied_in queryable via `patina mother` search results
  checked: false
- id: fts5-search-results-include-health-score-in-output-consumer-decides-ranking-no-blending-formula-ship-the-data-defer-the-tuning-per-andrew-ng-principle-measure-before-optimize
  text: 'FTS5 search results include health_score in output (consumer decides ranking; no blending formula — ship the data, defer the tuning per Andrew Ng principle: measure before optimize)'
  checked: false
---
# refactor: Mother Schema Alignment — Grounding + Verification in graph.db

> Align graph.db's beliefs schema with patina.db so cross-project queries
> have grounding depth, verification health, and temporal freshness.
> Area 3 of [[data-architecture-v2]]. Independent of Areas 1-2.

## Current State

graph.db syncs 13 columns from each project's patina.db beliefs table.
patina.db has 37 columns. The gap means mother can't answer questions
about belief quality — only belief existence.

**What syncs today:**

```
id, statement, entrenchment, status, facets,
cited_by_beliefs, cited_by_sessions, applied_in,
evidence_count, evidence_verified, health_score,
contested_by, imported
```

**What doesn't sync (column groups that matter for federation):**

| Column Group | Columns | Federation Value |
|-------------|---------|-----------------|
| Grounding | `grounding_score`, `grounding_code_count`, `grounding_commit_count`, `grounding_session_count`, `grounding_forge_count` | "Is this belief anchored in code?" — the difference between opinion and evidence |
| Verification | `verification_total`, `verification_passed`, `verification_failed`, `verification_errored` | "Has this belief been tested?" — verification pass rate across projects |
| Temporal | `last_activity` | "Is this belief stale?" — detect beliefs nobody's touched in months |

**Other gaps:**

- 71 dangling edges detected during sync but only warned, not cleaned
- `belief_applied_in` is populated but never surfaced in search results
- FTS5 `belief_search` returns results unranked by health — a floating
  belief ranks the same as one grounded in 47 functions

**What should NOT sync** (project-internal detail):
`file_path`, `persona`, `confidence`, `extracted`, `revised`,
`last_file_touch`, `last_frontmatter_revision`, `last_session_citation`,
`last_verification_run`, `defeated_attacks`, `external_sources`, `endorsed`,
`verification_drifted`. These are local to each project's analysis.

## Target State

graph.db beliefs table gains 10 columns across 3 groups. Sync pipeline
reads them from each project and writes them to graph.db. Cross-project
queries can filter and rank by grounding quality, verification health,
and freshness.

**New columns in graph.db beliefs:**
```sql
-- E4.6a Grounding
grounding_score REAL DEFAULT 0.0,
grounding_code_count INTEGER DEFAULT 0,
grounding_commit_count INTEGER DEFAULT 0,
grounding_session_count INTEGER DEFAULT 0,
grounding_forge_count INTEGER DEFAULT 0,

-- Verification
verification_total INTEGER DEFAULT 0,
verification_passed INTEGER DEFAULT 0,
verification_failed INTEGER DEFAULT 0,
verification_errored INTEGER DEFAULT 0,

-- Temporal
last_activity TEXT
```

## Steps

1. **Add columns to graph.db schema** — ALTER TABLE in `src/mother/graph.rs`
   `init_schema()`. Add the 10 columns with DEFAULT values. Existing graph.db
   files get columns via ALTER TABLE IF NOT EXISTS on next sync.

2. **Expand BeliefEntry struct** — Add 10 fields to the `BeliefEntry` struct
   in `src/mother/graph.rs`.

3. **Expand collect query** — Update `collect_project_beliefs()` SELECT in
   `src/commands/mother/graph.rs` to read all 23 columns (13 existing + 10 new).
   Handle missing columns gracefully — older project databases may not have
   grounding columns yet.

4. **Expand sync INSERT** — Update `sync_beliefs()` in `src/mother/graph.rs`
   to write all new columns during the per-source rebuild.

5. **Auto-clean dangling edges** — In `sync_belief_edges()`, after detecting
   dangling edges with `find_dangling_edges()`, DELETE them instead of just
   warning. Log the cleanup count. Dangling edges are stale references from
   removed beliefs — warning without cleanup accumulates garbage.

6. **Surface belief_applied_in in search** — When `patina mother search`
   returns results, include the project list from `belief_applied_in` in each
   result. "This belief exists in projects: patina, dojo, myapp."

7. **FTS5 health_score in output** — Include `health_score` as a returned
   field in FTS5 search results. Do NOT blend health into the ranking formula.
   The consumer (LLM or human) sees both text relevance and health and decides
   how to weight them. Rationale: hand-tuning ranking coefficients without an
   evaluation dataset is wasted effort. Ship the data. When `scry.use` logs
   show ranking is producing bad results because health is ignored, *then*
   introduce blending with real data to evaluate against.

## Non-Goals

- **Full temporal sync.** Only `last_activity` syncs. Per-column temporal
  signals (`last_file_touch`, `last_session_citation`, etc.) stay local.
- **Verification detail sync.** Pass/fail counts sync. Individual verification
  results (which functions passed/failed) stay in project patina.db.
- **Graph topology changes.** The `nodes`/`edges`/`edge_usage` tables in
  graph.db are a separate concern. This spec is beliefs-only.
- **Cross-project belief reconciliation.** Projects are sovereign. Divergent
  beliefs coexist. See SPEC § Federation conflict resolution.

## Key Files

- `src/mother/graph.rs` — graph.db schema, BeliefEntry struct, sync_beliefs()
- `src/commands/mother/graph.rs` — sync orchestration, collect_project_beliefs()
- `src/commands/scrape/beliefs/mod.rs` — patina.db beliefs table definition
