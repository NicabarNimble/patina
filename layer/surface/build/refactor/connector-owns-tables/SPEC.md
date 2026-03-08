---
type: refactor
id: connector-owns-tables
status: draft
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
beliefs:
- connectors-own-tables-schemas-are-contracts
- patina-is-domain-agnostic-knowledge-system
- pipes-are-processes-not-wasm
- mother-holds-connections-pipes-transform
exit_criteria:
- id: child-owns-materialize
  text: "Children expose a generic `materialize` capability; Mother invokes it without knowing table names, column mappings, or dedup rules"
  checked: false
- id: child-owns-search-contrib
  text: "Children expose a generic `contribute-search` capability that provides searchable corpus; core aggregates without knowing domain semantics"
  checked: false
- id: core-has-no-connector-knowledge
  text: "Core contains zero connector-specific table names, field mappings, event type conventions (no %.issue, no %.pr, no forge_*)"
  checked: false
- id: non-forge-connector-works
  text: "A non-forge child (e.g. slack) with a different data shape (messages, not issues) materializes and contributes to search with zero core code changes"
  checked: false
- id: domain-change-no-core-edit
  text: "Changing a connector's domain model (adding fields, renaming tables, changing dedup keys) requires zero edits to core or Mother"
  checked: false
---
# refactor: Connector-Owns-Tables — Children Own Contracts and Materializations

> Core owns routing, validation, and capability invocation; children own domain contracts, event semantics, materialization, and search/index contributions. If changing a connector's domain model requires editing core, the boundary is wrong.

## Core Invariant

**If adding or changing a connector's domain model requires core changes, the design is wrong.**

Core (Mother) knows how to:
- Ask: "what contracts do you provide?"
- Invoke: "materialize this contract" / "contribute searchable corpus"
- Route, validate, schedule, lifecycle

Children know how to:
- Declare contracts/capabilities
- Fetch and emit facts/events
- Materialize events into read models (tables, views)
- Contribute searchable documents/FTS rows
- Handle identity/dedup rules
- Manage schema migrations/evolution

## Current State

`schema-driven-projection` removed hardcoded event type strings from the pipeline. But core still owns connector-specific domain logic:

| What core knows today | Should own it |
|---|---|
| `CREATE TABLE forge_issues (number, title, body, state, ...)` | Child |
| `json_extract(e.data, '$.number')` → `number` column mapping | Child |
| `event_type LIKE '%.issue'` convention for search | Child (via search contract) |
| `ends_with(".pr")` for enrichment display | Child (via contract metadata) |
| `forge.issue` / `forge.pr` FTS5 labels | Child (via search contribution) |
| Dedup by `json_extract(e2.data, '$.number')` | Child |

Every row above is a boundary violation. Core contains hidden domain knowledge about issues and PRs.

## Target State

### Architecture

```
Child (github-connector)
  ├── fetch:       emit github.issue / github.pr events → events.db
  ├── materialize: project events → github_issues / github_prs tables
  ├── search:      contribute FTS5 rows from its read models
  └── contract:    declares "issues" and "pull-requests" capabilities

Child (slack-connector)
  ├── fetch:       emit slack.message events → events.db
  ├── materialize: project events → slack_messages table
  ├── search:      contribute FTS5 rows from its read model
  └── contract:    declares "messages" capability

Mother/Core
  ├── routes events from children to events.db
  ├── discovers child capabilities
  ├── invokes "materialize" generically (for all children)
  ├── invokes "contribute-search" generically (for all children)
  ├── aggregates search results across contracts
  └── knows zero domain semantics
```

### Event Log Stays

events.db remains the canonical write side. Children emit events through Mother. The CQRS audit trail is preserved. Materialization is a separate capability that transforms the write side into read models.

### Capability Protocol

Children declare capabilities in their manifest or schema:

```toml
[[capabilities]]
name = "materialize"
description = "Project events into read model tables"

[[capabilities]]
name = "contribute-search"
description = "Provide searchable documents for FTS5 index"
```

Mother invokes these generically:

1. `materialize` — child receives its own events from events.db, creates/updates its tables in patina.db
2. `contribute-search` — child provides (symbol_name, file_path, content, event_type) tuples for FTS5

Core never interprets the content. It passes the database connection and invokes the capability. The child does the domain work.

### Consumer Model

Consumers (scry, assay, user queries) ask for contracts, not connectors:

- "I want searchable documents" → Mother aggregates `contribute-search` from all children
- "I want issues" → Mother finds children declaring the "issues" contract
- "I want messages" → Mother finds children declaring the "messages" contract

If no child supports the requested contract, Mother fails clearly.

## Steps

### 1. Define capability protocol for materialize + contribute-search

Extend the pipe protocol (or child manifest) with generic capability declarations. Define the invocation interface — what Mother passes, what child returns.

### 2. Move projection into github-connector

Extract `project_from_events()` issue/PR logic from `events.rs` into the github-connector child. The child receives a database connection (or path) and handles its own CREATE TABLE, INSERT/SELECT, and dedup.

### 3. Move FTS5 contribution into github-connector

Extract `populate_fts5_issues()` / `populate_fts5_prs()` into the github-connector. The child provides FTS5 rows through the `contribute-search` capability.

### 4. Remove connector-specific code from core

Delete `create_materialized_views()` (the forge-shaped DDL), the hardcoded projection SQL, the convention-based search filters (`LIKE '%.issue'`), and the enrichment display logic (`ends_with(".pr")`). Replace with generic capability invocation.

### 5. Implement scrape as capability invocation

`patina scrape` becomes: discover children with `materialize` capability → invoke each → discover children with `contribute-search` → invoke each → aggregate FTS5. Core orchestrates, children execute.

### 6. Litmus: add a slack-connector

A slack child with completely different domain (messages, not issues). Declares its own materialize + contribute-search. Produces `slack_messages` table, contributes searchable text. Zero core changes.

## What This Means for Existing Code

| Current code | Disposition |
|---|---|
| `events.rs::create_materialized_views()` | Moves to github-connector |
| `events.rs::project_from_events()` | Moves to github-connector |
| `events.rs::populate_fts5_issues/prs()` | Moves to github-connector |
| `events.rs::issue_event_exists/pr_event_exists()` | Moves to github-connector |
| `events.rs::insert_issues/insert_prs()` | Moves to github-connector |
| `events.rs` domain types (Issue, PullRequest, etc.) | Moves to github-connector |
| `enrichment.rs` kind detection (`ends_with(".pr")`) | Replaced by contract metadata |
| `search.rs` event_type filter (`LIKE '%.issue'`) | Replaced by capability-contributed FTS5 |
| `oxidize/mod.rs` forge corpus query | Replaced by `contribute-search` capability |
| `schema_registry` table | Evolves into capability/contract registry |

## Data Flow Modes

Per [[data-architecture-v3]] and [[mother-maturation]], two modes coexist:

**Direct:** source → project (current github-connector flow)
```
github-connector → events.db → child.materialize() → patina.db
```

**Lake:** source → lake → block → project (future, multi-consumer)
```
github-connector → events.db (lake) → block extraction → project.materialize()
```

Both modes use the same capability protocol. The child's `materialize` works regardless of whether events come from a direct fetch or a lake block.

## Exit Criteria

- **child-owns-materialize:** children expose `materialize`; Mother invokes without domain knowledge
- **child-owns-search-contrib:** children expose `contribute-search`; core aggregates without domain semantics
- **core-has-no-connector-knowledge:** zero connector-specific table names, field mappings, or conventions in core
- **non-forge-connector-works:** non-forge child materializes and searches with zero core changes
- **domain-change-no-core-edit:** changing a connector's domain requires zero core/Mother edits
