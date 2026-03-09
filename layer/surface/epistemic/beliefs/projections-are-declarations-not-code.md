---
type: belief
id: projections-are-declarations-not-code
persona: architect
facets: [architecture, schema, projection]
entrenchment: medium
status: active
endorsed: true
extracted: 2026-03-09
revised: 2026-03-09
---

# projections-are-declarations-not-code

Schemas declare table structure via [[projections]]; core generates SQL mechanically from declarations. Zero domain knowledge in core. Adding a new connector's read model requires only schema.toml edits — no code changes to projection engine, search, enrichment, or embeddings.

## Statement

Schemas declare table structure via [[projections]]; core generates SQL mechanically from declarations. Zero domain knowledge in core. Adding a new connector's read model requires only schema.toml edits — no code changes to projection engine, search, enrichment, or embeddings.

## Evidence

- [[session-20260309-075229]]: [[spec-connector-owns-tables]] - all 4 exit criteria passed: schema-drives-projection, schema-drives-search, core-has-no-connector-knowledge, domain-change-schema-only (weight: 0.95)

## Supports

- [[connectors-own-tables-schemas-are-contracts]]
- [[patina-is-domain-agnostic-knowledge-system]]

## Attacks

<!-- Add beliefs this defeats -->

## Attacked-By

<!-- Add beliefs that challenge this -->

## Applied-In

- `src/commands/scrape/projection.rs` — generic projection engine: `project_from_schemas()` reads `[[projections]]`, generates DDL + INSERT SQL
- `src/commands/scrape/events.rs` — `populate_fts5_from_schema()` reads `[[indexes]]`, generates FTS5 SQL
- `src/commands/scrape/events.rs` — `insert_issues()`/`insert_prs()` resolve event types from schema at runtime
- `src/commands/assay/internal/search.rs` — `display_kind_for_event_type()` reads `[[contracts]]`
- `src/commands/scry/internal/enrichment.rs` — `display_kind_for_event_type()` reads `[[contracts]]`
- `src/commands/oxidize/mod.rs` — corpus queries from `[embedding].corpus_query` in schemas

## Revision Log

- 2026-03-09: Created — metrics computed by `patina scrape`
