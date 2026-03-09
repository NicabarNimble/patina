---
type: refactor
id: connector-owns-tables
status: active
created: 2026-03-08
sessions:
  origin: 20260308-070818
related:
- pipe-architecture
- schema-driven-projection
- core-extraction
- mother-maturation
- scrape-simplification
- patina-connect
- raw-lake-ingestion
beliefs:
- connectors-own-tables-schemas-are-contracts
- patina-is-domain-agnostic-knowledge-system
- pipes-are-processes-not-wasm
- mother-holds-connections-pipes-transform
exit_criteria:
- id: schema-drives-projection
  text: Schema declarations (`[[projections]]` in schema.toml) drive read model creation; core materializes generically without knowing table names, column mappings, or dedup rules
  checked: false
- id: schema-drives-search
  text: Schema declarations (`[[indexes]]` in schema.toml) drive FTS5 contribution; core aggregates search without knowing domain semantics
  checked: false
- id: core-has-no-connector-knowledge
  text: Core contains zero connector-specific table names, field mappings, event type conventions (no %.issue, no %.pr, no forge_*)
  checked: false
- id: domain-change-schema-only
  text: Changing a connector's domain model (adding fields, renaming tables, changing dedup keys) requires only schema.toml edits — zero changes to core, Mother, or child binaries
  checked: false
---
# refactor: Connector-Owns-Tables — Schema-Declared Contracts, Generic Materialization

> Schemas declare domain contracts. Core materializes generically from
> declarations. Connectors stay pure source-boundary adapters (fetch only).
> If changing a connector's domain model requires editing anything other
> than schema.toml, the boundary is wrong.

## Core Invariant

**If adding or changing a connector's domain model requires core changes or child binary changes, the design is wrong.**

Core (Mother) knows how to:
- Read schema declarations (projections, indexes, contracts)
- Execute generic materialization from declarations
- Execute generic FTS5 contribution from declarations
- Route, validate, schedule, lifecycle

Schemas (schema.toml) declare:
- Fact types, field types, identity fields
- Projection rules (events → read model tables)
- FTS5 contribution rules (which fields to index)
- Contract metadata (display kind, embedding config)

Connector children know how to:
- Fetch and emit facts from external sources
- Nothing else — they are source-boundary adapters

## Current State

`schema-driven-projection` removed hardcoded event type strings from the pipeline. But core still owns connector-specific domain logic:

| What core knows today | Moves to |
|---|---|
| `CREATE TABLE forge_issues (number, title, body, state, ...)` | Schema declarations (`[[projections]]` in schema.toml) |
| `json_extract(e.data, '$.number')` → `number` column mapping | Schema declarations (`json_path` in projections) |
| `event_type LIKE '%.issue'` convention for search | Schema declarations (`[[contracts]]` in schema.toml) |
| `ends_with(".pr")` for enrichment display | Schema declarations (`display_kind` in contracts) |
| `forge.issue` / `forge.pr` FTS5 labels | Schema declarations (`[[indexes]]` in schema.toml) |
| Dedup by `json_extract(e2.data, '$.number')` | Schema declarations (`primary_key` in projections) |

Every row above is a boundary violation. Core contains hidden domain
knowledge about issues and PRs. This domain knowledge moves to
schema.toml declarations — NOT to child runtime code. Core executes
generic projection from declarations. Connectors stay fetch-only.

## Target State

### Architecture

```
Schema (github/schema.toml)
  ├── facts:       declares github.issue, github.pr with field types
  ├── projections: declares github_issues, github_prs table DDL
  │                  column mappings from JSON → table columns
  │                  identity fields for dedup
  ├── indexes:     declares FTS5 contribution (title, body fields)
  └── contracts:   declares display_kind ("Issue", "PR")

Schema (slack/schema.toml)
  ├── facts:       declares slack.message with field types
  ├── projections: declares slack_messages table DDL
  ├── indexes:     declares FTS5 contribution (text field)
  └── contracts:   declares display_kind ("Message")

Child (github-connector) — fetch ONLY
  ├── fetch:       emit github.issue / github.pr events via pipe protocol
  └── does NOT:    materialize, write tables, contribute search

Child (slack-connector) — fetch ONLY
  ├── fetch:       emit slack.message events via pipe protocol
  └── does NOT:    materialize, write tables, contribute search

Mother/Core — generic projection engine
  ├── reads schema declarations for installed connectors
  ├── executes generic projection: events.db → read model tables
  │     (CREATE TABLE from [[projections]], INSERT from event JSON)
  ├── executes generic FTS5: read models → FTS5 rows
  │     (from [[indexes]] declarations)
  ├── populates contract registry from schema [[contracts]]
  ├── routes events from children to declared destinations
  └── knows zero domain semantics — all behavior from declarations
```

**Why connectors don't materialize:** The same role-boundary logic that
says "Mother doesn't write Parquet" says "connectors don't write SQLite
tables." Connectors are source-boundary adapters. Materialization is a
storage concern. For lake scope, the lakehouse child handles storage.
For project scope, generic schema-driven projection handles storage.
In both cases, the connector just fetches.

**Why materialization is core infrastructure, not a child:** Project
materialization is SQLite → SQLite generic projection. The same
technology stack core already owns. Compare to lakehouse: Parquet is
a different technology boundary requiring a dedicated child with
arrow/parquet crates. SQLite projection from JSON events is mechanical
SQL driven by declarations — no domain-specific runtime code needed.

### Event Log Stays (Project Scope)

events.db is the **project-scope** canonical write side. When a source
has `destination.type = "project"` (or absent), Mother routes connector
output to events.db. The CQRS audit trail is preserved. Materialization
is a separate capability that transforms the write side into read models.

**Lake-bound data bypasses events.db entirely.** When a source has
`destination.type = "lake"`, Mother routes connector output to the
lakehouse child via pipe/ingest — records never touch events.db. The
lake has its own audit trail: append-only Parquet files, lake_sync
cursor tracking, and provenance columns on every written record.

**Dual routing** (same connector data to both project and lake) is
achieved via two source entries in sources.toml pointing to the same
connection with different destinations. This is explicit fan-out via
config, not implicit dual-write.

**Dedup ownership per scope:**
- Project scope: events.db content_hash dedup (Mother-managed)
- Lake scope: identity field dedup against existing Parquet (lakehouse-managed)

### Schema-Driven Projection Protocol

Schemas declare projections and indexes in schema.toml. Core executes
them mechanically. No runtime capability negotiation needed — the schema
IS the contract.

```toml
# children/github-connector/schema.toml (extended)

[[projections]]
fact = "issue"
table = "github_issues"
primary_key = "number"
columns = [
    { name = "number", type = "INTEGER", json_path = "$.number" },
    { name = "title", type = "TEXT", json_path = "$.title" },
    { name = "body", type = "TEXT", json_path = "$.body" },
    { name = "state", type = "TEXT", json_path = "$.state" },
    { name = "created_at", type = "TEXT", json_path = "$.created_at" },
    { name = "updated_at", type = "TEXT", json_path = "$.updated_at" },
]

[[projections]]
fact = "pull-request"
table = "github_prs"
primary_key = "number"
columns = [
    { name = "number", type = "INTEGER", json_path = "$.number" },
    { name = "title", type = "TEXT", json_path = "$.title" },
    { name = "body", type = "TEXT", json_path = "$.body" },
    { name = "state", type = "TEXT", json_path = "$.state" },
]

[[indexes]]
fact = "issue"
fts_fields = ["title", "body"]
table = "github_issues"

[[indexes]]
fact = "pull-request"
fts_fields = ["title", "body"]
table = "github_prs"

[[contracts]]
name = "issues"
event_type = "github.issue"
display_kind = "Issue"

[[contracts]]
name = "pull-requests"
event_type = "github.pr"
display_kind = "PR"
```

Core reads these declarations and executes generic SQL:

1. **Projection:** For each `[[projections]]` entry, CREATE TABLE IF NOT
   EXISTS with declared columns. INSERT OR REPLACE from events.db using
   json_extract with declared json_paths. Dedup by declared primary_key.
2. **FTS5 contribution:** For each `[[indexes]]` entry, DELETE existing
   FTS5 rows for this event_type, INSERT FTS5 rows from the projected
   table using declared fts_fields.
3. **Contract registry:** For each `[[contracts]]` entry, populate the
   contract registry with display_kind metadata.

Core never interprets column values or field semantics. It mechanically
maps declarations to SQL. The domain knowledge lives in schema.toml,
authored by the connector developer.

### Consumer Classes

All consumer scopes are first-class. The architecture does not privilege project-scoped projection over other consumer scopes.

| Consumer | Write side | Purpose |
|----------|-----------|---------|
| **Project** | project events.db → project patina.db | Facts inside a project: read models, search index, embeddings |
| **Lake** | lake storage (not project events.db) | Raw/normalized shared data, consumed by multiple projects |
| **Block** | block storage (materialized product) | Shaped data for a purpose: weekly summary, metrics, curated dataset |
| **Transform** | another contract or block | Input from another child/lake; transforms compose |

**Key properties:**
- Same source, different consumers, different write sides
- Each scope has a technology-appropriate materializer (not the connector)
- Schemas declare what data a connector provides; materializers decide how to write it
- Contracts are consumer-facing; schemas are the declaration mechanism

**Consumer queries** ask for contracts, not connectors:

- "I want searchable documents" → Core runs generic FTS5 engine from `[[indexes]]` declarations
- "I want issues" → Core queries contract_registry for schemas declaring "issues" contract
- "I want messages" → Core queries contract_registry for schemas declaring "messages" contract

If no schema declares the requested contract, Mother fails clearly.

## Steps

### 1. Extend schema.toml with `[[projections]]` and `[[contracts]]`

Add `[[projections]]` section to github-connector's schema.toml
declaring table DDL (column names, types, json_paths, primary key).
Add `[[contracts]]` section declaring display_kind metadata. The
`[[indexes]]` section already exists.

### 2. Build generic projection engine in core

New module in core that reads `[[projections]]` from installed schemas,
generates CREATE TABLE DDL and INSERT/REPLACE SQL from declarations,
executes against patina.db. No connector-specific SQL — all behavior
derived from schema declarations.

### 3. Build generic FTS5 contribution engine in core

Extend the projection engine to read `[[indexes]]` from installed
schemas, generate FTS5 INSERT SQL from declarations, execute against
patina.db code_fts table. Replace the hardcoded populate_fts5_issues/prs
functions.

### 4. Build contract registry from schema declarations

Replace schema_registry with contract_registry populated from
`[[contracts]]` sections. Core reads display_kind from registry for
enrichment display, replacing the hardcoded `ends_with(".pr")` logic.

### 5. Rewire scrape to use generic engines

`patina scrape` becomes: run generic projection engine (reads all
installed schemas, projects all event types) → run generic FTS5 engine
→ contract registry for enrichment. Delete all hardcoded domain SQL.

### 6. Litmus: add a slack-connector schema

Add `slack/schema.toml` with different domain shape (messages, not
issues). Declares `[[projections]]` for `slack_messages` table,
`[[indexes]]` for FTS5, `[[contracts]]` for display_kind. Zero core
changes. Zero child binary changes (slack-connector only needs fetch).
Generic engines handle projection automatically.

## What This Means for Existing Code

| Current code | Disposition |
|---|---|
| `events.rs::create_materialized_views()` | Replaced by generic projection engine reading `[[projections]]` |
| `events.rs::project_from_events()` | Replaced by generic projection engine |
| `events.rs::populate_fts5_issues/prs()` | Replaced by generic FTS5 engine reading `[[indexes]]` |
| `events.rs::issue_event_exists/pr_event_exists()` | Replaced by generic dedup using declared primary_key |
| `events.rs::insert_issues/insert_prs()` | Replaced by generic INSERT from json_extract declarations |
| `events.rs` domain types (Issue, PullRequest, etc.) | Deleted — not needed; projection works from JSON directly |
| `enrichment.rs` kind detection (`ends_with(".pr")`) | Replaced by contract registry lookup (from `[[contracts]]`) |
| `search.rs` event_type filter (`LIKE '%.issue'`) | Replaced by contract registry query |
| `oxidize/mod.rs` forge corpus query | Replaced by schema `corpus_query` execution (already exists) |
| `schema_registry` table | Evolves into contract_registry populated from `[[contracts]]` |
| github-connector binary | Stays as fetch-only — NO materialize or search modes added |

## Data Flow by Consumer Scope

Per [[pipe-architecture]] §Data Layers and [[mother-maturation]], facts
flow through different consumer scopes. Each scope has its own
materialization mechanism. Mother routes by destination declaration.

**Project (this spec):** source → project events.db → generic projection → project patina.db
```
github-connector → events.db → core projection engine (from schema.toml) → patina.db
```
Materialization is core infrastructure: schema-driven, generic,
no domain code. Technology: SQLite → SQLite.

**Lake (raw-lake-ingestion):** source → Mother → lakehouse child → Parquet
```
github-connector → Mother routes → lakehouse child → raw Parquet files
```
Materialization is a dedicated child: Parquet requires arrow/parquet
crates, different technology boundary. Lakehouse child is domain-agnostic.

**Block/Transform (future specs):** transform children read from lake or
project, produce shaped output. Different concern, different mechanism.

**Pattern:** each scope has a materializer appropriate to its technology:
- Project scope: generic SQL projection (core infrastructure)
- Lake scope: Parquet writer (lakehouse child)
- Block scope: transform child (future)
Connectors never materialize in any scope. They fetch and emit.

**Alignment with [[pipe-architecture]]:** This matches the Data Layers
flow (Sources → Lakes → Blocks → Projects → Beliefs). connector-owns-tables
provides the project-scope materialization mechanism via schema-driven
projection.

## Exit Criteria

- **schema-drives-projection:** `[[projections]]` in schema.toml drive read model creation; core materializes generically
- **schema-drives-search:** `[[indexes]]` in schema.toml drive FTS5 contribution; core aggregates without domain semantics
- **core-has-no-connector-knowledge:** zero connector-specific table names, field mappings, or conventions in core
- **domain-change-schema-only:** changing a connector's domain requires only schema.toml edits — zero changes to core, Mother, or child binaries
## Scope Narrowing (session 20260308-134326, revised 20260308-164629)

This spec is **project-scope materialization** only.
Lake-scope, block-scope, and transform-scope are tracked by
separate specs.

**Session 20260308-164629 revision — schema-driven projection:**
The original spec had connectors gaining `materialize` and
`contribute-search` capability modes. This was role-smearing: it
makes connectors both source-boundary (fetcher) AND storage-boundary
(materializer). The same pattern we corrected for lakehouse
("Mother doesn't write Parquet inline") applies here:
**connectors don't write SQLite tables.**

**Revised approach:** Schemas declare projection contracts.
Core executes generic materialization from declarations.
Connectors stay pure source-boundary adapters. See
"Schema-Driven Projection Protocol" section above.

**Moved to [[raw-lake-ingestion]]:**
- Lake destination write path (raw Parquet capture via lakehouse child)

**Moved to future specs:**
- Full multi-consumer architecture (block, transform scopes)
- Non-forge connector litmus test (slack-connector) — can be proved
  with schema.toml only (no slack-connector binary needed for the
  materialization proof; only fetch requires the binary)

**Relationship to [[raw-lake-ingestion]]:**
raw-lake-ingestion proves lake-scope capture (records → Parquet via
lakehouse child). This spec proves project-scope materialization
(events → SQLite read models via schema-driven projection). Together
they demonstrate that connectors fetch, dedicated mechanisms
materialize, and schemas declare the contracts.

**Role-boundary consistency:**

| Scope | Materializer | Technology | Domain code? |
|-------|-------------|------------|--------------|
| Project | Generic projection engine (core) | SQLite → SQLite | No — schema-driven |
| Lake | Lakehouse child | JSON → Parquet | No — schema-driven |
| Block | Transform child (future) | Various | Domain-specific (by design) |

Connectors are source-boundary in ALL scopes. They never materialize.
