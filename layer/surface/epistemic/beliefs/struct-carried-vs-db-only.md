---
type: belief
id: struct-carried-vs-db-only
persona: architect
facets: [architecture, data-flow, rust]
entrenchment: medium
status: active
endorsed: true
extracted: 2026-02-16
revised: 2026-02-16
---

# struct-carried-vs-db-only

Data derived from file content flows through structs; data derived from table-to-table comparison stays in SQL — this separates struct-carried metrics from DB-only signals.

## Statement

Data derived from file content flows through structs; data derived from table-to-table comparison stays in SQL — this separates struct-carried metrics from DB-only signals.

## Evidence

- [[session-20260216-073845]]: Phase C contested_by (from file parsing) follows struct path through BeliefMetrics → insert_belief(); Phase A verification_drifted (from table diff) stays DB-only via post-insert UPDATE. Two valid approaches existed — the distinction clarified which to use where (weight: 0.9)
- [[belief-truthfulness]] SPEC.md: 11-concern review forced an explicit choice between struct-carried and DB-only for both drift and contest detection — the file-content vs table-diff distinction resolved the decision cleanly (weight: 0.85)

## Supports

- [[dependable-rust]] — keeping DB-only concerns out of struct interfaces preserves a small, stable public surface
- [[specs-require-zero-ambiguity]] — the distinction eliminates a judgment call (which path to use) by giving a clear rule

## Attacks

<!-- Add beliefs this defeats -->

## Attacked-By

- Simplicity counter-argument: always using post-insert UPDATE for everything would be simpler (one pattern). Counter: struct-carried data is computed during the existing parse/cross-reference pipeline — routing it through UPDATE would require extra bookkeeping and break the natural data flow.

## Applied-In

- [[belief-truthfulness]] SPEC.md Phase A: `verification_drifted` — DB-only path (table diff → Phase 3b UPDATE)
- [[belief-truthfulness]] SPEC.md Phase C: `contested_by` — struct-carried path (file parse → BeliefMetrics → insert_belief)

## Revision Log

- 2026-02-16: Created — metrics computed by `patina scrape`
