---
type: belief
id: scraper-indexes-knowledge-specs-filter-work
persona: architect
facets: [architecture, separation-of-concerns, scraper, specs]
entrenchment: medium
status: active
endorsed: true
extracted: 2026-02-05
revised: 2026-02-05
---

# scraper-indexes-knowledge-specs-filter-work

The layer scraper indexes all knowledge for scry and context; spec commands filter that knowledge into work-item views — fix query logic, not the indexer.

## Statement

The layer scraper indexes all knowledge for scry and context; spec commands filter that knowledge into work-item views — fix query logic, not the indexer.

## Evidence

- [[session-20260205-163242]]: [[session-20260205-163242]] - Investigated sub-docs polluting spec list. Scraper correctly indexes all .md in layer/surface/ for scry/context. Bug was in spec queries using file_path LIKE without status filter. Fix: AND status IS NOT NULL in spec/internal.rs (weight: 0.9)

## Supports

- [[unix-philosophy]] — single responsibility: scraper indexes, spec commands query
- [[spec-is-milestone]] — specs are work items with status; knowledge docs are not

## Attacks

<!-- Add beliefs this defeats -->

## Attacked-By

<!-- Add beliefs that challenge this -->

## Applied-In

- `src/commands/spec/internal.rs` — `AND p.status IS NOT NULL` filter in `get_all_specs()`, `get_ready_specs()`, `get_blocked_specs()`
- `layer/surface/build/fix/spec-list-filter/SPEC.md` — spec documenting the fix

## Revision Log

- 2026-02-05: Created — metrics computed by `patina scrape`
