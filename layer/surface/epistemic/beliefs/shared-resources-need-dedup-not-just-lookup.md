---
type: belief
id: shared-resources-need-dedup-not-just-lookup
persona: architect
facets: [architecture, schema, migration]
entrenchment: medium
status: active
endorsed: true
extracted: 2026-03-08
revised: 2026-03-08
---

# shared-resources-need-dedup-not-just-lookup

Schema-driven migrations must handle shared resources (tables, indexes) as a dedup problem, not just a lookup problem — replacing hardcoded strings with schema lookups is necessary but not sufficient; the coordinator must enforce one-writer-per-resource semantics.

## Statement

Schema-driven migrations must handle shared resources (tables, indexes) as a dedup problem, not just a lookup problem — replacing hardcoded strings with schema lookups is necessary but not sufficient; the coordinator must enforce one-writer-per-resource semantics.

## Evidence

- [[session-20260308-222827]]: [[session-20260308-222827]] - FTS5 migration initially replaced hardcoded forge functions with schema-driven loop but indexed shared tables twice under different event_type labels, producing fabricated duplicates. External audit caught three correctness issues (P1/P2/P3). Fixed by adding HashSet dedup + stale entry cleanup + deterministic sort. (weight: 0.95)

## Supports

- [[contracts-before-consumers]] — contracts define the resource boundaries; dedup enforces them at consumption time
- [[connectors-own-tables-schemas-are-contracts]] — shared tables are the transitional state this belief guards against

## Attacks

<!-- None yet -->

## Attacked-By

<!-- None yet -->

## Applied-In

- `src/commands/scrape/events.rs:populate_fts5_from_schema()` — HashSet dedup over `IndexConfig.table`, deterministic sort by schema name, stale entry cleanup for skipped event_types. [[commit-bb92d940]], [[commit-cf3f9ae2]]

## Revision Log

- 2026-03-08: Created — metrics computed by `patina scrape`
