---
type: belief
id: connectors-never-materialize
persona: architect
facets: [architecture, connectors, lake, materialization, role-boundary]
entrenchment: medium
status: active
endorsed: true
extracted: 2026-03-08
revised: 2026-03-08
---

# connectors-never-materialize

Connectors are source-boundary adapters. They fetch and emit facts — nothing else. Each consumer scope has a technology-appropriate materializer: generic SQL projection engine for project scope, lakehouse child for lake scope, transform child for future scopes. Connectors never write to storage in any scope.

## Statement

Connectors are source-boundary adapters that own one external system boundary. They fetch data, emit domain records via pipe protocol, and declare their schema. They never materialize (write read models, Parquet files, or any storage). Each consumer scope has a dedicated materializer matched to its technology: project scope uses a generic schema-driven SQL projection engine (core infrastructure), lake scope uses a lakehouse child (Parquet technology boundary), block/transform scopes use transform children (domain-specific derivation). The pattern: generic operations use core infrastructure; specialized technology boundaries use dedicated children. Connectors stay out of storage in ALL scopes.

## Evidence

- [[session-20260308-164629]]: Role-boundary alignment session. Earlier spec had connectors gaining `materialize` and `contribute-search` modes. Identified as role-smearing: same pattern as Mother writing Parquet inline (which was corrected to lakehouse child in session 20260308-134326). If connectors write SQLite tables, they become both source-boundary AND storage-boundary — violating unix-philosophy (one tool, one job). (weight: 0.9)
- [[session-20260308-134326]]: Established lakehouse child as real boundary. Mother governs, children execute bounded roles. The lakehouse correction (Mother doesn't write Parquet) naturally extends: connectors don't write SQLite either. (weight: 0.85)

## Supports

- [[mother-owns-destination-format]] — Mother routes, lakehouse writes. This belief extends the principle: Mother routes, scope-appropriate materializers write. Connectors only fetch.
- [[pipes-are-processes-not-wasm]] — children are single-purpose services. A connector that also materializes is two services in one binary.
- [[raw-lake-is-capture-contract-first]] — raw lake is about what gets captured, not how it's stored. Connector owns the "what" (domain records). Lakehouse owns the "how" (Parquet format).

## Attacks

- Child-owns-materialize — the approach where connectors gain runtime materialization code. Makes connectors multi-purpose (fetch + materialize + search). Violates source-boundary role.
- Core-owns-domain-logic — the concern that schema-driven projection puts domain knowledge in core. Counter: core executes GENERIC SQL from declarations. Domain knowledge is in schema.toml, authored by the connector developer. Core is a mechanical executor, not a domain interpreter.

## Attacked-By

- "Connector knows its domain best for materialization" — the connector developer understands issue/PR semantics. Counter: that knowledge goes into schema.toml declarations, not runtime code. The developer still controls materialization — through declarations, not binary modes.
- "Schema declarations can't express complex projections" — joins, aggregations, conditional logic. Counter: complex materialization belongs to transform children (future scope). Schema-driven projection handles the 90% case. The escape hatch is explicit and typed.

## Applied-In

- [[spec-connector-owns-tables]] — revised to schema-driven projection. Connectors stay fetch-only. Generic projection engine reads [[projections]] from schema.toml.
- [[spec-raw-lake-ingestion]] — lakehouse child writes Parquet (technology-appropriate materializer for lake scope). Connector unchanged from project-scope behavior.

## Revision Log

- 2026-03-08: Created — scope-appropriate materializer pattern established during role-boundary alignment session
