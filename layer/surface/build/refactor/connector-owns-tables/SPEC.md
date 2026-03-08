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
- raw-lake-ingestion
beliefs:
- connectors-own-tables-schemas-are-contracts
- patina-is-domain-agnostic-knowledge-system
- pipes-are-processes-not-wasm
- mother-holds-connections-pipes-transform
exit_criteria:
- id: child-owns-materialize
  text: "Children expose a generic `materialize` capability for project scope; Mother invokes it without knowing table names, column mappings, or dedup rules"
  checked: false
- id: child-owns-search-contrib
  text: "Children expose a generic `contribute-search` capability that provides searchable corpus; core aggregates without knowing domain semantics"
  checked: false
- id: core-has-no-connector-knowledge
  text: "Core contains zero connector-specific table names, field mappings, event type conventions (no %.issue, no %.pr, no forge_*)"
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
  ├── fetch:       emit github.issue / github.pr events
  ├── materialize: events → read models (destination-dependent)
  │                  project: → github_issues / github_prs in patina.db
  │                  lake:    → raw github data in lake storage
  │                  block:   → shaped output in block storage
  ├── search:      contribute FTS5 rows from materialized read models
  └── contract:    declares "issues" and "pull-requests" capabilities

Child (slack-connector)
  ├── fetch:       emit slack.message events
  ├── materialize: events → read models (destination-dependent)
  │                  project: → slack_messages in patina.db
  │                  lake:    → raw messages in lake storage
  ├── search:      contribute FTS5 rows from its read model
  └── contract:    declares "messages" capability

Mother/Core
  ├── routes events from children to declared destinations
  ├── discovers child capabilities + supported consumer scopes
  ├── invokes "materialize" with destination context (scope + path)
  ├── invokes "contribute-search" generically (project scope)
  ├── aggregates search results across contracts
  └── knows zero domain semantics — routes by declaration, not content
```

### Event Log Stays

events.db remains the canonical write side. Children emit events through Mother. The CQRS audit trail is preserved. Materialization is a separate capability that transforms the write side into read models.

### Capability Protocol

Children declare capabilities and supported consumer scopes in their manifest:

```toml
[[capabilities]]
name = "materialize"
description = "Project events into read model tables"
scopes = ["project", "lake"]  # which consumer scopes this child supports

[[capabilities]]
name = "contribute-search"
description = "Provide searchable documents for FTS5 index"
scopes = ["project"]  # search is project-scoped
```

Mother invokes these with destination context:

1. `materialize(scope, source_path, destination_path)` — child receives its own events from the source, materializes into the destination. The child decides what materialization means for each scope. Project scope writes SQL tables; lake scope might write normalized data; block scope might write shaped output.
2. `contribute-search(destination_path)` — child provides searchable tuples for FTS5. Project-scoped (search index is per-project).

Core provides paths and scope. Core never interprets what the child writes. The child does the domain work.

Not every child supports all scopes. Mother matches capability requests to what children declare. If no child supports the requested scope for a contract, Mother fails clearly.

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
- A child may satisfy the same domain contract differently depending on destination
- Not every child supports all consumer scopes
- Mother matches capability requests to what children declare; fails clearly if no match
- Contracts are consumer-facing; capabilities are destination-aware

**Consumer queries** ask for contracts, not connectors:

- "I want searchable documents" → Mother aggregates `contribute-search` from all children
- "I want issues" → Mother finds children declaring the "issues" contract
- "I want messages" → Mother finds children declaring the "messages" contract

If no child supports the requested contract for the requested scope, Mother fails clearly.

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

## Data Flow by Consumer Scope

Per [[pipe-architecture]] §Data Layers and [[mother-maturation]], facts flow through different consumer scopes. All use the same capability protocol. Mother routes by destination declaration, not by data content.

**Project (direct):** source → project events.db → child.materialize(project) → project patina.db
```
github-connector → project events.db → child.materialize("project", events_db, patina_db)
```

**Lake:** source → lake event store → child.materialize(lake) → lake storage
```
github-connector → lake events.db → child.materialize("lake", events_db, lake_path)
```

**Block:** lake/project → transform child → block storage
```
lake data → transform-child.materialize("block", lake_path, block_path)
```

**Transform:** child → child (composition)
```
child-A output → child-B.materialize("transform", source_path, dest_path)
```

The child's `materialize` works regardless of consumer scope. The child decides what materialization means for each scope. Core provides paths and scope, never content interpretation.

**Alignment with [[pipe-architecture]]:** This matches the Data Layers flow (Sources → Lakes → Blocks → Projects → Beliefs) and Destination Declarations (pub/sub routing by type). connector-owns-tables is the materialization half of what pipe-architecture routes.

## Exit Criteria

- **child-owns-materialize:** children expose `materialize`; Mother invokes without domain knowledge
- **child-owns-search-contrib:** children expose `contribute-search`; core aggregates without domain semantics
- **core-has-no-connector-knowledge:** zero connector-specific table names, field mappings, or conventions in core
- **non-forge-connector-works:** non-forge child materializes and searches with zero core changes
- **domain-change-no-core-edit:** changing a connector's domain requires zero core/Mother edits
- **destination-aware-capabilities:** capability invocation includes consumer scope; children materialize differently per destination; adding a scope requires zero domain knowledge in core
- **lake-block-independent-write:** lake and block consumers have independent write paths; facts route to declared destinations, not forced through project events.db
## Scope Narrowing (session 20260308-134326)

This spec was narrowed to **project-scope materialization** only.
Lake-scope, block-scope, and transform-scope capabilities are
tracked by separate specs.

**Moved to [[raw-lake-ingestion]]:**
- Lake destination write path (raw Parquet capture)
- Lake-block independent write paths
- Same-child multi-scope demonstration (project + lake)

**Moved to future specs:**
- Full multi-consumer architecture (block, transform scopes)
- Consumer-scope-no-core-knowledge (architectural proof across all scopes)
- Non-forge connector litmus test (slack-connector)

**Relationship to [[raw-lake-ingestion]]:**
raw-lake-ingestion proves lake-scope capture (records → Parquet).
This spec proves project-scope materialization (events → SQLite
read models). Together they demonstrate destination-aware capabilities
work across scopes. Neither spec is blocked by the other.

The DESIGN.md retains the full multi-consumer architecture context
as future direction. The exit criteria are project-scope only.
