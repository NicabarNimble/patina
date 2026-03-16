---
type: belief
id: contracts-signal-active-writer
persona: architect
facets: [architecture, schemas, seam-2]
entrenchment: medium
status: active
endorsed: true
extracted: 2026-03-09
revised: 2026-03-09
---

# contracts-signal-active-writer

Schemas with [[contracts]] declarations are active connectors that own the write path; schemas without contracts are legacy read-only. Use structural presence of contracts to determine write priority, never name-based heuristics.

## Statement

Schemas with [[contracts]] declarations are active connectors that own the write path; schemas without contracts are legacy read-only. Use structural presence of contracts to determine write priority, never name-based heuristics.

## Evidence

- [[session-20260308-233938]]: [[session-20260308-233938]] - P1 audit revealed alphabetical-first schema selection kept forge.* labels active despite DESIGN.md forbidding new forge.* writes. Fixed by adding priority column to schema_registry keyed on contracts presence. (weight: 0.95)

## Supports

- [[contracts-before-consumers]] — contracts are the authority, and their presence distinguishes active from legacy
- [[connectors-own-tables-schemas-are-contracts]] — connectors express ownership through contract declarations
- [[shared-resources-need-dedup-not-just-lookup]] — priority-based resolution is the write-side complement to dedup

## Applied-In

- `src/commands/scrape/events.rs`: `resolve_event_type()` uses `ORDER BY priority DESC` where priority is derived from `schema.contracts.is_empty()`
- `src/commands/scrape/events.rs`: `populate_schema_registry()` sets `priority = if schema.contracts.is_empty() { 0 } else { 1 }`
- [[commit-f4180f3d]]: schema_registry gains priority column, github (with contracts) gets priority=1, forge (without) gets priority=0

## Revision Log

- 2026-03-09: Created — metrics computed by `patina scrape`
