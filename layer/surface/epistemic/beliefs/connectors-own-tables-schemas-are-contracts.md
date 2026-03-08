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

Schemas declare domain contracts (projections, indexes, display metadata). Core materializes generically from declarations. Connectors are source-boundary adapters (fetch only) — they never materialize. Each consumer scope has a technology-appropriate materializer: generic projection engine for project scope, lakehouse child for lake scope, transform child for future scopes. If changing a connector's domain model requires editing anything other than schema.toml, the boundary is wrong.

## Statement

Schemas declare domain contracts via `[[projections]]`, `[[indexes]]`, and `[[contracts]]` in schema.toml. Core owns routing, validation, and generic materialization from declarations. Connectors are source-boundary adapters that fetch and emit facts — they do not own materialization, search contribution, or storage. Each scope has a technology-appropriate materializer (project: generic SQL engine, lake: lakehouse child, block/transform: transform child). If changing a connector's domain model requires editing anything other than schema.toml, the boundary is wrong.

## Evidence

- [[session-20260308-070818]]: Schema-driven projection revealed forge_issues/forge_prs shared tables can't support non-forge connectors (Slack, Google Workspace). Initial spec had core reading schema.toml and building tables — user identified this as too weak. (weight: 0.9)
- [[session-20260308-081423]]: Expanded scope beyond project-only projection. Consumer classes (project, lake, block, transform) are all first-class. Same source, different consumers, different write sides. (weight: 0.85)
- [[session-20260308-164629]]: Role-boundary alignment applied. Earlier revision had connectors gaining materialize/contribute-search modes — identified as role-smearing (same pattern as Mother writing Parquet inline). Corrected: connectors are source-boundary adapters (fetch only). Schemas declare projection contracts. Core materializes generically. Each scope has a technology-appropriate materializer: project=generic SQL engine, lake=lakehouse child. (weight: 0.9)

## Supports

- [[patina-is-domain-agnostic-knowledge-system]] — domain agnosticism requires children to own their domain logic, not leak it into core
- [[pipes-are-processes-not-wasm]] — children are independent processes that own their full domain lifecycle
- [[mother-holds-connections-pipes-transform]] — Mother routes and invokes; children transform and materialize
- [[pipe-protocol-is-transport-agnostic]] — the capability protocol (materialize, contribute-search) is transport-agnostic; same invocation interface regardless of scope or destination

## Attacks

- Child-owns-materialize — the intermediate approach where connectors gain materialize/contribute-search modes and write SQLite tables directly. Role-smearing: makes connectors both source-boundary AND storage-boundary. Same pattern as Mother writing Parquet inline. Corrected to schema-driven projection.
- Shared tables (forge_issues/forge_prs) — the legacy approach where all connectors funnel into the same table structure. Works only when all sources share the same data shape.

## Attacked-By

- Schema-driven projection puts materialization in core — isn't that "core owns domain logic"? Counter: core executes GENERIC projection from declarations. Domain knowledge is in schema.toml, authored by the connector developer. Core interprets declarations mechanically (CREATE TABLE, json_extract, INSERT). Same as how the lakehouse child writes Parquet generically without domain knowledge.
- Schemas can't express complex materialization (joins, aggregations). Counter: complex materialization belongs to transform children (future scope), not to the schema-driven projection engine. The engine handles the 90% case; transform children handle the rest.

## Applied-In

- [[spec-schema-driven-projection]] — foundation layer: schema_registry table, dynamic event type discovery (precursor to full capability model)
- [[spec-connector-owns-tables]] — full spec: children own materialize + contribute-search capabilities

## Revision Log

- 2026-03-08: Created — initial framing as "connector declares tables via schema.toml"
- 2026-03-08: Revised — strengthened to "children own contracts and materializations; core invokes capabilities"
- 2026-03-08: Revised — expanded to multi-consumer architecture: consumer classes (project, lake, block, transform), destination-aware capabilities, contracts are consumer-facing
- 2026-03-08: Revised — role-boundary alignment: connectors are source-boundary adapters (fetch only), not materializers. Schemas declare projection contracts. Core materializes generically from declarations. Each scope has a technology-appropriate materializer.
