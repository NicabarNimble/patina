---
type: belief
id: connectors-own-tables-schemas-are-contracts
persona: architect
facets: [architecture, schemas, connectors, pipe-architecture]
entrenchment: medium
status: active
endorsed: true
extracted: 2026-03-08
revised: 2026-03-08
---

# connectors-own-tables-schemas-are-contracts

Each connector declares its own materialized tables via schema.toml; schemas are contracts between producer (connector) and consumer (project), not shared infrastructure

## Statement

Each connector declares its own materialized tables via schema.toml; schemas are contracts between producer (connector) and consumer (project), not shared infrastructure

## Evidence

- [[session-20260308-070818]]: [[session-20260308-070818]] - Schema-driven projection revealed forge_issues/forge_prs shared tables can't support non-forge connectors (Slack, Google Workspace). Connector-owns-tables means each schema.toml declares DDL, table names, projection shape. Different projects install different schemas. (weight: 0.9)

## Supports

- [[patina-is-domain-agnostic-knowledge-system]] — domain agnosticism requires connectors to own their own shapes, not share a forge-centric table
- [[pipes-are-processes-not-wasm]] — connectors as independent processes aligns with each owning its schema contract
- [[mother-holds-connections-pipes-transform]] — mother routes events; schemas tell consumers how to materialize them

## Attacks

- Shared tables (forge_issues/forge_prs) — the legacy approach where all connectors funnel into the same table structure. Works only when all sources share the same data shape.

## Attacked-By

- Shared tables simplify search (one table to query for "all issues"). Counter: registry-based discovery solves this.

## Applied-In

- [[spec-schema-driven-projection]] — schema_registry table populated from installed schemas, projection reads registry instead of hardcoded strings
- `wit/schema/gitea/schema.toml` — litmus test schema declaring its own event types that project with zero core changes

## Revision Log

- 2026-03-08: Created — metrics computed by `patina scrape`
