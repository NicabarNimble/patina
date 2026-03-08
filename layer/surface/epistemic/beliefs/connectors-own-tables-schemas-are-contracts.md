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

Core owns routing, validation, and capability invocation; children own domain contracts, event semantics, materialization, and search/index contributions. Capabilities are destination-aware — the same child may materialize differently for project, lake, block, or transform consumer scopes. If changing a connector's domain model or adding a consumer scope requires editing core, the boundary is wrong.

## Statement

Core owns routing, validation, and capability invocation; children own domain contracts, event semantics, materialization, and search/index contributions. Capabilities are destination-aware across consumer scopes (project, lake, block, transform). Contracts are consumer-facing; capabilities are destination-aware. If changing a connector's domain model requires editing core, the boundary is wrong.

## Evidence

- [[session-20260308-070818]]: Schema-driven projection revealed forge_issues/forge_prs shared tables can't support non-forge connectors (Slack, Google Workspace). Initial spec had core reading schema.toml and building tables — user identified this as too weak. Stronger boundary: children own materialization and search contributions as capabilities; Mother invokes generically. (weight: 0.9)
- [[session-20260308-081423]]: Expanded scope beyond project-only projection. Consumer classes (project, lake, block, transform) are all first-class. Same source, different consumers, different write sides. Capability invocation must include destination context (scope + path). Contracts are consumer-facing; capabilities are destination-aware. Not every child supports all scopes — Mother matches and fails clearly. (weight: 0.85)

## Supports

- [[patina-is-domain-agnostic-knowledge-system]] — domain agnosticism requires children to own their domain logic, not leak it into core
- [[pipes-are-processes-not-wasm]] — children are independent processes that own their full domain lifecycle
- [[mother-holds-connections-pipes-transform]] — Mother routes and invokes; children transform and materialize
- [[pipe-protocol-is-transport-agnostic]] — the capability protocol (materialize, contribute-search) is transport-agnostic; same invocation interface regardless of scope or destination

## Attacks

- Core-reads-schema-and-builds-tables — the intermediate approach where core generates DDL/projection from schema.toml declarations. Still makes core the hidden executor of connector domain logic. Core should invoke capabilities, not interpret schemas.
- Shared tables (forge_issues/forge_prs) — the legacy approach where all connectors funnel into the same table structure. Works only when all sources share the same data shape.

## Attacked-By

- Shared tables simplify search (one table to query for "all issues"). Counter: contract-driven search contribution solves this — children contribute FTS5 rows, core aggregates.
- Children writing to patina.db creates coupling. Counter: database path passing is a minimal contract; children don't depend on core's schema.

## Applied-In

- [[spec-schema-driven-projection]] — foundation layer: schema_registry table, dynamic event type discovery (precursor to full capability model)
- [[spec-connector-owns-tables]] — full spec: children own materialize + contribute-search capabilities

## Revision Log

- 2026-03-08: Created — initial framing as "connector declares tables via schema.toml"
- 2026-03-08: Revised — strengthened to "children own contracts and materializations; core invokes capabilities"
- 2026-03-08: Revised — expanded to multi-consumer architecture: consumer classes (project, lake, block, transform), destination-aware capabilities, contracts are consumer-facing
