---
type: belief
id: measure-reads-tables-not-events
persona: architect
facets: [measure, eventlog, data-integrity]
entrenchment: medium
status: active
endorsed: true
extracted: 2026-02-25
revised: 2026-02-25
---

# measure-reads-tables-not-events

Measure reads from materialized source tables for current state, not from eventlog events. The beliefs table has post-grounding truth; belief.surface events have pre-grounding snapshots. Use tables for current state, eventlog for history.

## Statement

Measure reads from materialized source tables for current state, not from eventlog events. The beliefs table has post-grounding truth; belief.surface events have pre-grounding snapshots. Use tables for current state, eventlog for history.

## Evidence

- [[session-20260225-221415]]: [[session-20260225-221415]] - believe verb was reading stale belief.surface events (pre-grounding scores, per-creation-date batches) instead of beliefs table (current grounding after oxidize). Fixed in prior session [[commit-d9bdb6cc]]. Drill-down also showed misleading per-date data, fixed in [[commit-8b9b1df0]] by adding current beliefs table summary. (weight: 0.95)

## Supports

- [[seq-order-is-not-timestamp-order]] — related: both address data ordering/freshness assumptions in eventlog queries

## Attacks

<!-- Add beliefs this defeats -->

## Attacked-By

<!-- Add beliefs that challenge this -->

## Applied-In

- `src/commands/measure/internal.rs:build_believe_summary` — reads from `beliefs` table (current grounding scores after oxidize) instead of `belief.surface` events (stale pre-grounding snapshots)
- `src/commands/measure/internal.rs:render_believe_current_state` — drill-down shows beliefs table summary first, then belief.surface history as secondary context
- `src/commands/scrape/code/mod.rs` — measure.capture reads `function_facts` and `type_vocabulary` tables (materialized counts) rather than counting eventlog `code.function` rows (may have duplicates)

## Revision Log

- 2026-02-25: Created — metrics computed by `patina scrape`
