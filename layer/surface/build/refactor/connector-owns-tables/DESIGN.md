# Design: Connector-Owns-Tables — Children Own Contracts and Materializations

## Why This Work Exists

[[schema-driven-projection]] removed hardcoded event type strings from
the pipeline — projection now discovers event types from the
`schema_registry` table. But core still contains hidden domain
knowledge: table DDL, column mappings, dedup rules, FTS5 labels,
and display conventions for issues and PRs.

The boundary test is: **if changing a connector's domain model
requires editing core, the boundary is wrong.** Today, adding a Slack
connector requires editing `events.rs` (new table DDL, new projection
SQL, new dedup logic), `search.rs` (new FTS5 filter), `enrichment.rs`
(new display logic), and `oxidize/mod.rs` (new corpus query). That's
4 subsystems in core for one connector's domain.

The fix is: **schemas declare projection contracts; core executes
generic materialization from declarations.** Connector children stay
pure source-boundary adapters — they fetch and emit facts, nothing
else.

An earlier version of this spec had connectors gaining `materialize`
and `contribute-search` capability modes. Session 20260308-164629
identified this as role-smearing: the same pattern where Mother was
writing Parquet inline (corrected to lakehouse child) applies here.
Connectors should not write SQLite tables for the same reason
Mother should not write Parquet — it conflates source-boundary with
storage-boundary.

The schema.toml already declares most of what's needed: field types,
identity fields, `[[indexes]]` with FTS5 fields. Adding
`[[projections]]` with column declarations makes materialization
fully declarative. Core projects mechanically from declarations —
no connector-specific code anywhere.

**Origin:** [[session-20260308-070818]] — user established that the
boundary in the initial spec draft was too weak. Core should not own
connector-specific projection logic.
**Revised:** [[session-20260308-164629]] — role-boundary alignment
applied. Connectors are source-boundary adapters, not materializers.
Schema declarations carry the projection contracts. Core executes
generic projection. Parallel to lakehouse: technology-appropriate
materializer per scope, connectors stay out of storage.

## What Exists Today

### Boundary Violations in Core

Every item below is connector-specific domain logic living in core:

| Location | What it knows | Why it's wrong |
|----------|---------------|----------------|
| `events.rs:176` | `CREATE TABLE forge_issues (number INTEGER PRIMARY KEY, title TEXT, ...)` | Core knows issue table shape |
| `events.rs:258` | `json_extract(e.data, '$.number')` → `number` | Core knows issue JSON structure |
| `events.rs:142` | `WHERE event_type LIKE '%.issue'` | Core knows naming convention |
| `events.rs:155` | Dedup by `json_extract(data, '$.number')` | Core knows issue identity |
| `events.rs:499` | FTS5 label `'forge.issue'` | Core brands connector data |
| `search.rs:162` | `LIKE '%.issue' OR LIKE '%.pr'` | Core infers domain from naming |
| `search.rs:191` | `ends_with(".issue")` → `[ISSUE]` | Core formats domain display |
| `enrichment.rs:62` | `ends_with(".pr")` → `"PR"` | Core classifies domain data |
| `oxidize/mod.rs:603` | `LIKE '%.issue' OR LIKE '%.pr'` | Core filters domain for embedding |
| `oxidize/mod.rs:616` | `ends_with(".pr")` → `"PR"` | Core classifies for embedding |

All of this moves to the github-connector (or equivalent child).

### What Core Should Keep

| Concern | Why core owns it |
|---------|-----------------|
| `events.db` eventlog schema | Universal write side, connector-agnostic |
| Event routing (Mother/broker) | Transport, not domain |
| Child lifecycle (spawn, health, shutdown) | Infrastructure |
| Capability discovery ("what can you do?") | Contract negotiation |
| Capability invocation ("do it") | Generic execution |
| FTS5 table schema (`code_fts`) | Shared search infrastructure |
| Embedding index infrastructure (USearch) | Shared embedding infrastructure |
| `schema_registry` → capability/contract registry | Discovery, not domain logic |

## Schema-Driven Projection

### Two Generic Engines

Core provides two generic engines that read schema declarations and
execute projection mechanically. No child involvement — these are
core protocol infrastructure, like routing or validation.

**1. Projection Engine**

Runs after events are captured (during scrape). For each installed
schema with `[[projections]]` entries:

Input: events.db (source), patina.db (destination), schema.toml declarations
Output: populated read model tables in patina.db

Mechanics:
- Read `[[projections]]` from schema.toml
- For each projection entry:
  - CREATE TABLE IF NOT EXISTS with declared columns
  - INSERT OR REPLACE from events.db using json_extract with
    declared json_paths
  - Dedup by declared primary_key
  - Handle schema evolution: if projection adds a column, ALTER TABLE
    ADD COLUMN (SQLite supports this)

```sql
-- Generated from [[projections]] declarations (github example):
CREATE TABLE IF NOT EXISTS github_issues (
    number INTEGER PRIMARY KEY,
    title TEXT,
    body TEXT,
    state TEXT,
    created_at TEXT,
    updated_at TEXT
);
INSERT OR REPLACE INTO github_issues (number, title, body, state, created_at, updated_at)
SELECT
    json_extract(data, '$.number'),
    json_extract(data, '$.title'),
    json_extract(data, '$.body'),
    json_extract(data, '$.state'),
    json_extract(data, '$.created_at'),
    json_extract(data, '$.updated_at')
FROM eventlog
WHERE event_type = 'github.issue';
```

This is the same SQL that lives in events.rs today — but generated
from declarations instead of hardcoded for one connector.

**2. FTS5 Contribution Engine**

Runs after projection. For each installed schema with `[[indexes]]`
entries:

Input: projected tables in patina.db, schema.toml declarations
Output: FTS5 rows in code_fts table

Mechanics:
- Read `[[indexes]]` from schema.toml
- For each index entry:
  - DELETE FROM code_fts WHERE event_type = declared event_type
  - INSERT FTS5 rows from the declared table using declared fts_fields
  - Label with event_type from corresponding `[[facts]]` entry

```sql
-- Generated from [[indexes]] declarations:
DELETE FROM code_fts WHERE event_type = 'github.issue';
INSERT INTO code_fts (symbol_name, file_path, content, event_type)
SELECT
    title,
    'github.issue#' || number,
    title || ' ' || COALESCE(body, ''),
    'github.issue'
FROM github_issues;
```

**Search is project-scoped.** FTS5 contribution operates on
project patina.db only. Lake and block consumers do not feed
per-project search. Search is part of the belief loop.

### Scope-Appropriate Materialization

Each consumer scope has a materializer appropriate to its technology
and complexity:

| Scope | Materializer | Why |
|-------|-------------|-----|
| Project | Generic projection engine (core) | SQLite → SQLite, mechanical, schema-driven |
| Lake | Lakehouse child | Parquet requires arrow/parquet, different technology |
| Block/Transform | Transform child (future) | Domain-specific derivation, requires runtime logic |

**The pattern:** generic operations use core infrastructure; specialized
technology boundaries use dedicated children. Connectors NEVER
materialize in any scope.

**Where scope-switching lives:** Schema declarations can specify
scope-specific behavior. For example, a future `[[projections]]` entry
could declare `scope = "lake"` with different column mappings than
`scope = "project"`. But the v1 scope is project-only.

### Domain Assumption Review

Schema-driven projection must be truly generic. If core starts
assuming projections have certain column shapes, or FTS5 means
issue/PR-style fields, we've recreated the problem under a new name.

**Projection engine — verified generic:**
- Core reads `[[projections]]` declarations: table name, column
  list with types and json_paths, primary key. All from schema.toml.
- Core generates SQL mechanically: CREATE TABLE, json_extract,
  INSERT OR REPLACE. No interpretation of column values.
- Risk: if core assumes specific column names or types beyond what
  the schema declares, domain leaks back. Core must treat column
  values as opaque.
- Risk: if core adds validation rules per column type (e.g., "INTEGER
  columns must be positive"), it becomes domain-aware. Core validates
  SQL execution, not data semantics.

**FTS5 engine — verified generic:**
- The FTS5 schema `(symbol_name, file_path, content, event_type)` is
  shared infrastructure. Column names are generic labels.
- Core generates FTS5 INSERT from `[[indexes]]` declarations. Core
  never interprets `event_type` values.
- Risk: if core uses `event_type` patterns (`LIKE '%.issue'`) to
  filter or classify, domain leaks back. The `include_issues` flag
  in search must use contract registry queries.
- Risk: if the FTS5 column set expands to accommodate one connector,
  it becomes domain-specific. The current 4-column schema is generic.

**Contract registry — verified generic:**
- `display_kind` from `[[contracts]]` answers "what is this event
  type for display?" without core hardcoding "Issue" or "PR".
- Core queries the registry. Schema provides the answer at install time.
- Risk: if `display_kind` grows into rich formatting, core becomes
  a domain renderer. Keep it to a single label string.

### Invocation Model

Children are source-boundary adapters invoked via the pipe protocol
for fetch only. Materialization and FTS5 contribution happen in core
from schema declarations.

**Fetch** (child): Mother spawns child, child fetches from external
API, emits events via pipe protocol. Unchanged from current behavior.

**Projection** (core engine): After fetch, core reads `[[projections]]`
from installed schemas, executes generic SQL against events.db → patina.db.
No child involved.

**FTS5 contribution** (core engine): After projection, core reads
`[[indexes]]` from installed schemas, executes generic FTS5 SQL.
No child involved.

**Contract registration** (core startup): Core reads `[[contracts]]`
from installed schemas, populates contract_registry in patina.db.
No child involved.

Child manifest declares fetch capabilities only:

```toml
# children/github-connector/child.toml — NO materialize/search modes
[child]
name = "github-connector"
version = "0.1.0"
type = "connector"
runtime = "native"
lifecycle = "poll"
description = "GitHub issues and pull requests via REST API"

[capabilities]
data_types = ["issues", "prs"]
supports_incremental = true
# NOTE: no materialize or contribute-search capabilities.
# Projection and FTS5 are driven by schema.toml, not child code.

[schemas.github]
package = "patina:schema/github@1.0.0"
```

### What Core Does at Scrape Time (Project Scope)

```
patina scrape (project scope):
  1. Local capture (code facts, patterns, commits, etc.) — unchanged
  2. Run projection engine: for each installed schema with
     [[projections]], generate and execute projection SQL
     (events.db → patina.db read model tables)
  3. Run FTS5 engine: for each installed schema with [[indexes]],
     generate and execute FTS5 contribution SQL
  4. Run contract registration: for each installed schema with
     [[contracts]], populate contract_registry
  5. Build embedding indices (oxidize) — uses schema corpus_query
     (already exists, no change needed)
```

No child invocation for materialization. Core reads schema
declarations and executes generic SQL. The projection engine
is new infrastructure; the SQL it generates is equivalent to
what events.rs hardcodes today.

### Lake/Block Consumer Scopes

Lake and block scopes use DIFFERENT materializers — not the
projection engine:

```
lake ingestion (raw-lake-ingestion spec):
  1. Mother routes connector output to lakehouse child via pipe/ingest
  2. Lakehouse child writes Parquet (its own technology boundary)
  3. No schema-driven projection involved — raw zone is capture-first

block/transform (future specs):
  1. Transform children read from lake/project data
  2. Produce shaped output — domain-specific runtime logic
  3. Different mechanism from schema-driven projection
```

Core never touches connector-specific tables, columns, or logic.
Projection is schema-driven. Lake is lakehouse-driven. Transform
is child-driven. Connectors only fetch.

## Migration: Explicit Incremental Steps

Migration is 5 phases, each independently verifiable. Each phase is
one commit or a small commit sequence. No phase depends on a future
phase being done.

**Key difference from earlier design:** No code moves to the
connector child. Projection logic moves from hardcoded SQL to
schema-driven generic SQL. The github-connector stays fetch-only.

### Phase 1: Extend schema.toml with `[[projections]]` and `[[contracts]]`

**What:** Add projection declarations to the github connector's
schema.toml. The `[[indexes]]` section already exists.

```toml
# children/github-connector/schema.toml additions:

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

[[contracts]]
name = "issues"
event_type = "github.issue"
display_kind = "Issue"

[[contracts]]
name = "pull-requests"
event_type = "github.pr"
display_kind = "PR"
```

**Verify:** `patina schema show github` includes projections and
contracts. Schema parsing validates declaration format.

**What doesn't change:** events.rs still works (hardcoded path).
Both old and new declarations coexist temporarily.

### Phase 2: Build generic projection engine

**New module:** `src/commands/scrape/projection.rs` (or similar)

The projection engine reads `[[projections]]` from all installed
schemas and generates+executes SQL mechanically:

```rust
fn project_from_schema(
    events_conn: &Connection,  // events.db
    patina_conn: &Connection,  // patina.db
    schema: &SchemaConfig,
) -> Result<ProjectionStats> {
    for projection in &schema.projections {
        // CREATE TABLE IF NOT EXISTS with declared columns
        let ddl = generate_create_table(&projection);
        patina_conn.execute(&ddl, [])?;

        // INSERT OR REPLACE from events.db using json_extract
        let insert = generate_insert_sql(&projection, &schema.facts);
        patina_conn.execute(&insert, [])?;
    }
    // ...
}
```

**Verify:** Generic projection produces identical tables to the
hardcoded events.rs functions. Diff the schema and row counts.

**Coexistence:** The generic engine runs alongside the old hardcoded
path until Phase 4 deletes it.

### Phase 3: Build contract registry from schema declarations

**New table in patina.db** (replaces/extends schema_registry):

```sql
CREATE TABLE IF NOT EXISTS contract_registry (
    schema_name TEXT NOT NULL,
    event_type  TEXT NOT NULL,
    contract    TEXT,           -- e.g. 'issues'
    display_kind TEXT,          -- e.g. 'Issue'
    table_name  TEXT,           -- e.g. 'github_issues'
    PRIMARY KEY (schema_name, event_type)
);
```

**Populated from `[[contracts]]` in installed schemas.** Replaces
the schema_registry table. Built on scrape startup.

**Enrichment:** Replace `ends_with(".pr")` → `"PR"` with:
```sql
SELECT display_kind FROM contract_registry WHERE event_type = ?
```
Fall back to the raw event_type string if no match.

**Search:** Replace `LIKE '%.issue' OR LIKE '%.pr'` with:
```sql
event_type IN (SELECT event_type FROM contract_registry)
```

**Verify:** `patina scry` and `patina assay` display correctly via
contract registry lookup. No hardcoded domain strings.

### Phase 4: Delete connector-specific code from core

**Now safe to delete (generic engines replace it):**

| File | What to delete |
|------|---------------|
| `events.rs:20-75` | Domain types (Issue, PullRequest, etc.) — not needed |
| `events.rs:146-169` | `issue_event_exists()`, `pr_event_exists()` |
| `events.rs:176-238` | `create_materialized_views()` — replaced by generic DDL |
| `events.rs:256-336` | `project_from_events()` — replaced by projection engine |
| `events.rs:350-495` | `insert_issues()`, `insert_prs()` (dead code) |
| `events.rs:506-545` | `populate_fts5_issues()`, `populate_fts5_prs()` — replaced by FTS5 engine |
| `enrichment.rs:62-66` | `ends_with(".pr")` → `"PR"` — replaced by contract registry |
| `search.rs:162-166` | `LIKE '%.issue' OR LIKE '%.pr'` — replaced by registry query |
| `search.rs:191-197` | `ends_with(".issue")` → `[ISSUE]` display |
| `oxidize/mod.rs:597-632` | Hardcoded forge corpus query — schema `corpus_query` already works |

**events.rs retains:** `create_schema_registry()` evolves into
`create_contract_registry()`. `ProjectionStats` struct stays.

**Verify:** `rg 'forge_issues\|forge_prs\|%.issue\|%.pr\|ends_with.*pr' src/`
returns zero matches. `cargo build --release` succeeds. `patina scrape`
works with schema-driven projection.

### Phase 5: Litmus test — add a non-forge connector schema

Add `slack/schema.toml` with completely different domain shape
(messages, not issues/PRs):

```toml
[schema]
name = "slack"
version = "1.0.0"

[[facts]]
name = "message"
event_type = "slack.message"
identity_fields = ["ts", "channel"]

[[projections]]
fact = "message"
table = "slack_messages"
primary_key = "ts"
columns = [
    { name = "ts", type = "TEXT", json_path = "$.ts" },
    { name = "channel", type = "TEXT", json_path = "$.channel" },
    { name = "user", type = "TEXT", json_path = "$.user" },
    { name = "text", type = "TEXT", json_path = "$.text" },
]

[[indexes]]
fact = "message"
fts_fields = ["text"]
table = "slack_messages"

[[contracts]]
name = "messages"
event_type = "slack.message"
display_kind = "Message"
```

**Verify:** Zero core code changes. Schema install + `patina scrape`
creates `slack_messages` table and FTS5 rows. `patina assay` searches
across both github and slack data. `patina scry` enrichment displays
"Message" via contract registry. The slack-connector binary is NOT
needed for this litmus test — only the schema declaration matters
for materialization. The binary is only needed for fetch.

### Table Rename / Legacy Migration

The `forge_issues`/`forge_prs` tables cease to exist. The generic
projection engine creates `github_issues`/`github_prs` from schema
declarations. Migration:

1. On first generic projection run, engine checks for old tables
2. If `forge_issues` exists: `ALTER TABLE forge_issues RENAME TO github_issues`
3. Same for `forge_prs` → `github_prs`
4. This is a one-time migration in the projection engine

The `forge` schema stays in `.patina/schemas/forge/` for backward
compatibility with old `forge.*` events in events.db. No new events
use `forge.*` — the github-connector emits `github.*`.

## Enrichment and Search Without Domain Knowledge

### Problem

Today, `enrichment.rs` uses `ends_with(".pr")` to decide whether a
forge event is a PR or an Issue. `search.rs` uses
`LIKE '%.issue' OR LIKE '%.pr'` to filter FTS5 results. Both are
domain knowledge in core.

### Solution: Contract Metadata

When the generic FTS5 engine writes rows from `[[indexes]]`
declarations, it uses the event_type from the corresponding
`[[facts]]` entry as the FTS5 label. The github schema uses
`github.issue` and `github.pr`. A slack schema would use
`slack.message`. Core never chooses these labels — they come from
schema declarations.

For enrichment (scry vector search), the key ID range check
(`key >= FORGE_ID_OFFSET`) already works generically — it looks up
the event in eventlog and uses whatever `event_type` is stored there.
The only domain-specific part is the `kind` classification ("PR" vs
"Issue"). This moves to contract metadata:

```toml
# In schema.toml
[[contracts]]
name = "issues"
event_type = "github.issue"
display_kind = "Issue"

[[contracts]]
name = "pull-requests"
event_type = "github.pr"
display_kind = "PR"
```

Core stores contract metadata in the contract_registry (populated
from `[[contracts]]` at scrape time). Enrichment queries the
registry to determine display kind:

```sql
SELECT display_kind FROM contract_registry WHERE event_type = ?
```

If no match, fall back to the event_type string itself. Core never
hardcodes "Issue" or "PR".

### Search Filter

The `include_issues` flag in assay search currently maps to
`LIKE '%.issue' OR LIKE '%.pr'`. With schema-driven search, this
becomes:

```sql
-- All FTS5 rows from schema-driven indexing (vs code.* from local scrape)
event_type NOT LIKE 'code.%'
```

Or, if the user wants a specific contract:

```sql
event_type IN (SELECT event_type FROM contract_registry WHERE contract = 'issues')
```

Either way, core doesn't know what "issues" looks like — it queries
the registry.

## Oxidize Without Domain Knowledge

### Problem

`query_knowledge_corpus()` in `oxidize/mod.rs` uses
`LIKE '%.issue' OR LIKE '%.pr'` and `ends_with(".pr")` to build the
forge/issue/PR embedding corpus.

### Solution

Each child's schema already declares a `corpus_query` in the
`[embedding]` section. Oxidize already knows how to load installed
schemas. The fix:

1. Remove the hardcoded forge event query from `query_knowledge_corpus()`
2. For each installed schema with `[embedding].corpus_query`, execute
   that query against events.db
3. The child's `corpus_query` handles its own domain logic (which
   event types, which fields to embed)

This is the one place where the schema.toml declaration (rather than
a runtime capability invocation) is appropriate — the corpus query is
static SQL, not domain logic that changes at runtime. The schema
declares it, oxidize executes it mechanically.

## Design Decisions

### 1. Schema-Driven Projection, Not Child-Owned Materialization

An earlier version of this design had children gaining `materialize`
and `contribute-search` modes — the connector binary writing SQLite
tables directly. This was identified as role-smearing (session
20260308-164629): the same pattern where Mother was writing Parquet
inline (corrected to lakehouse child).

The fix is NOT "move SQL from core to child." The fix is "move SQL
from hardcoded to declarative." Schema.toml already declares field
types, identity fields, FTS5 fields, and embedding corpus queries.
Adding `[[projections]]` makes it complete. Core generates SQL
mechanically from declarations — no domain knowledge anywhere.

**Why this is stronger than child-owned materialization:**
- Adding a connector requires only schema.toml — no Rust materialization code
- The boundary test is stricter: zero code changes anywhere (core, Mother, OR child)
- Consistent with lakehouse: both are domain-agnostic materializers
- Avoids making connectors into multi-mode binaries (fetch + materialize + search)
- Schema.toml is reviewable, diffable, and declarative

**When this might not be sufficient:** If a connector needs complex
materialization logic (joins across fact types, conditional aggregation,
custom denormalization), schema declarations won't cover it. That case
is handled by transform children (future scope) which explicitly own
domain-specific runtime logic. The escape hatch is a dedicated transform
child, not a capability bolted onto the connector.

### 2. Event Log Stays as Canonical Write Side

Children emit events through Mother to events.db. The CQRS audit trail
is preserved. Materialization is a separate read-side concern, invoked
after events are captured. This means:

- Events are never lost (write side is durable)
- Materialization can be re-run (idempotent)
- Multiple read models can coexist (different schemas, different tables)
- The event log is the source of truth, tables are derived

### 3. Connectors Are Fetch-Only

Connectors are source-boundary adapters. They own ONE thing: bridging
an external system. Per [[pipes-are-processes-not-wasm]], they are
separate binaries invoked via pipe protocol for fetch. They do not
gain materialize or contribute-search modes.

This is consistent with the role-boundary doctrine:
- Connector = source boundary (GitHub API)
- Lakehouse = storage boundary (Parquet files)
- Projection engine = protocol infrastructure (SQLite read models)
- Transform child = derivation boundary (future, domain-specific)

### 4. schema_registry Evolves Into Contract Registry

The current `schema_registry` table maps event_type → table_name.
This evolves into a `contract_registry` that maps:

- schema_name → event_types
- event_type → contract metadata (display_kind, table_name)
- contract_name → which schemas provide it

The registry is populated from `[[contracts]]` in installed schemas,
rebuilt on scrape startup.

### 5. Consumer Scopes Use Scope-Appropriate Materializers

Per [[pipe-architecture]] §Data Layers, four consumer scopes exist.
Each uses a different materialization mechanism:

| Consumer | Materializer | Technology |
|----------|-------------|------------|
| **Project** | Generic projection engine (core) | SQLite → SQLite |
| **Lake** | Lakehouse child ([[raw-lake-ingestion]]) | JSON → Parquet |
| **Block** | Transform child (future) | Various |
| **Transform** | Transform child (future) | Various |

The pattern: generic operations use core infrastructure; specialized
technology boundaries use dedicated children. Connectors fetch in
all scopes. They never materialize in any scope.

**Alignment with pipe-architecture:**
- Data Layers: Sources → Lakes → Blocks → Projects → Beliefs
- Destination Declarations: Mother routes by source declaration
- connector-owns-tables provides the project-scope materialization
  via schema-driven projection

**Alignment with core-extraction:**
- Core = protocol + stores. Schema-driven projection is protocol
  infrastructure — the same way routing and validation are protocol.
- schema_registry evolving into contract_registry is consistent
  with core owning generic infrastructure while schemas carry domain.

## Key Files

| File | Current State | Target State | Migration Phase |
|------|---------------|--------------|-----------------|
| `src/commands/scrape/events.rs` | Domain types, DDL, projection, FTS5 | Generic projection engine (schema-driven) | P2 (build engine), P4 (delete hardcoded) |
| `children/github-connector/schema.toml` | Facts, indexes, embedding | +`[[projections]]`, +`[[contracts]]` | P1 (extend schema) |
| `children/github-connector/` | Fetch mode only | Fetch mode only — UNCHANGED | — |
| `children/github-connector/child.toml` | Fetch capabilities | Fetch capabilities — UNCHANGED | — |
| `src/commands/scry/internal/enrichment.rs` | `ends_with(".pr")` | Contract registry lookup | P3 (registry), P4 (delete) |
| `src/commands/assay/internal/search.rs` | `LIKE '%.issue'` convention | Contract registry query | P3 (registry), P4 (delete) |
| `src/commands/oxidize/mod.rs` | Hardcoded forge corpus query | Schema `corpus_query` execution (already works) | P4 (delete hardcoded) |
| `src/broker/` | Child lifecycle (fetch only) | Fetch only — UNCHANGED | — |

## Scope Boundary: Projection vs Contract System

This spec owns **schema-driven projection** — how schemas declare
read models and how core materializes them generically. It does NOT
own contract compatibility, versioning, or cross-child negotiation.

| Concern | Owner | Why |
|---------|-------|-----|
| `[[projections]]` schema format | connector-owns-tables | This spec |
| `[[contracts]]` schema format | connector-owns-tables | This spec |
| Generic projection engine | connector-owns-tables | This spec |
| Generic FTS5 engine | connector-owns-tables | This spec |
| Contract registry table | connector-owns-tables | This spec |
| Schema versioning (v1 → v2 evolution) | pipe-protocol-types | Contract system |
| Compatibility checking (required/optional fields) | pipe-protocol-types | Contract system |
| Multiple schemas declaring same contract | pipe-protocol-types | Conflict resolution |

### Minimum Viable Contract Matching (v1)

For v1, contract matching is exact string equality:

```
Enrichment query: SELECT display_kind FROM contract_registry
                  WHERE event_type = 'github.issue'
Match → display "Issue". No match → display raw event_type string.
```

This is deliberately simple. No versioning, no negotiation.
One schema declares one contract name for one event type.

**When this breaks:** When two schemas declare "issues" with
different field sets. When a schema evolves and event shapes change.
These belong in [[spec-pipe-protocol-types]].

## Cross-References

### [[pipe-architecture]] Alignment

- **Data Layers** (§Data Layers): Sources → Lakes → Blocks → Projects
  → Beliefs. connector-owns-tables provides the project-scope
  materialization that pipe-architecture routes to.
- **Destination Declarations** (§Destination Declarations): Mother
  routes by declaration. connector-owns-tables provides the read-model
  infrastructure at the project destination.
- **Child Taxonomy** (§Child Taxonomy): Connector children bridge
  external sources. connector-owns-tables keeps them as fetch-only
  source adapters — no capability expansion.
- **Pipe Protocol** (§Pipe Protocol): No new pipe methods needed.
  Projection is core infrastructure, not child invocation.
- **No conflict.** pipe-architecture defines routing; connector-owns-tables
  defines schema-driven materialization at the project destination.

### [[core-extraction]] Alignment

- **Protocol vs Domain** (§The Line): Core = protocol + stores.
  connector-owns-tables moves domain knowledge from hardcoded SQL
  into schema declarations. Core becomes a generic projection engine.
  Domain logic doesn't move to children — it moves to declarations.
- **Forge extraction** (§What ISN'T Core): connector-owns-tables
  is the specific mechanism for extracting forge domain logic from
  events.rs. The ~500 LOC of domain SQL becomes ~30 lines of schema.toml
  declarations plus a generic engine.
- **schema_registry → contract_registry** evolves the store from
  event-type discovery to contract metadata (display_kind, table_name).
  Consistent with core owning generic infrastructure.

## Open Questions

1. **Incremental projection.** Full re-projection on every scrape
   is idempotent but slow for large event logs. Should the projection
   engine track a high-water mark per schema? INSERT OR REPLACE is
   already idempotent, so the main cost is the full scan.
   Recommendation: acceptable for v1 (typical event logs are < 10K
   rows). Add `WHERE seq > last_projected_seq` optimization later.

2. **forge legacy.** The `forge` schema describes events from the
   deleted ForgeReader (pre-v0.40). Should we keep projecting
   `forge.*` events? They exist in old event logs. Recommendation:
   the forge schema stays with its own `[[projections]]` that maps
   `forge.issue`/`forge.pr` events to the same `github_issues`/
   `github_prs` tables. Generic engine handles it.

3. **Cross-schema search ranking.** When multiple schemas contribute
   FTS5 rows, the per-table normalization in `assay_search()` needs
   to handle variable numbers of sources. Currently it normalizes
   code, commits, patterns, eventlog. Adding schema-contributed rows
   requires the normalization loop to be dynamic. Mechanical change.

4. **Projection schema evolution.** If a `[[projections]]` entry adds
   a column, the engine should ALTER TABLE ADD COLUMN. SQLite supports
   this. But what about removing a column? SQLite doesn't support
   ALTER TABLE DROP COLUMN in older versions. Recommendation: new
   columns only; removal requires table recreation (rare, manual).

5. **Complex projection escape hatch.** If a connector needs joins,
   aggregation, or conditional logic beyond what `[[projections]]`
   can declare, the schema-driven approach isn't sufficient. The
   escape hatch is a transform child (future scope). Document this
   boundary clearly so schema declarations don't become a Turing-
   complete DSL.
