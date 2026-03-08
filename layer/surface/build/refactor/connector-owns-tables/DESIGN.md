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

The fix is not "core reads schema.toml and builds tables on behalf of
connectors" — that still makes core the hidden owner of connector
domains. The fix is: **children own their materializations and search
contributions; Mother invokes them through generic capabilities.**

**Origin:** [[session-20260308-070818]] — user established that the
boundary in the initial spec draft was too weak. Core should not own
connector-specific projection logic. Children should be self-contained.
The tighter rule: "Core owns routing, validation, and capability
invocation; children own domain contracts, event semantics,
materialization, and search/index contributions."

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

## Capability Protocol

### Two Core Capabilities

Every data-producing child can expose two capabilities:

**1. `materialize`**

Mother invokes after events are captured. Child receives:
- **scope**: consumer scope — `"project"`, `"lake"`, `"block"`, or `"transform"`
- **source_path**: path to event source (events.db, lake path, block path)
- **destination_path**: path to write destination (patina.db, lake storage, block storage)
- Optional: last materialization timestamp (for incremental)

Child does whatever it needs for the given scope:
- CREATE TABLE IF NOT EXISTS (its own tables)
- INSERT OR REPLACE from source eventlog
- Dedup by its own identity rules
- Schema migrations if table shape changed
- **Different behavior per scope is the child's decision** — e.g.,
  project scope might create denormalized tables for fast query, lake
  scope might write normalized rows, block scope might write shaped
  output for a downstream consumer

Child returns:
- Count of rows materialized
- Table names or artifact names created/updated (for registry)

**2. `contribute-search`**

Mother invokes after materialization. Child receives:
- Path to project `patina.db` (read/write, specifically `code_fts` table)
- Its own table names (from materialize result)

Child does:
- DELETE its own FTS5 rows (by its own label)
- INSERT FTS5 rows from its read model tables
- Choose which fields to index, what labels to use

Child returns:
- Count of documents contributed

**Search is project-scoped.** Only project-consumer materialization
feeds the search index. Lake and block consumers do not contribute
to per-project FTS5. This is not a limitation — it's a boundary.
Search is part of the belief loop, which operates per-project.

### Capability Splitting vs Scope-Switching

Scope should change destination and context, not hide a completely
different operation. If a child's implementation for two scopes
shares no meaningful logic, they should be separate capabilities —
not scope-switched branches of `materialize`.

**Scope-switching (correct):** github-connector materializes for
project scope (denormalized tables in patina.db) and lake scope
(normalized rows in lake storage). Both read from events.db, both
project github events, both handle dedup. The core logic is shared;
the output shape and destination differ.

**Capability splitting (correct):** A transform child that aggregates
weekly metrics from lake data is doing fundamentally different work
than a connector child that projects events into tables. These should
be separate capabilities (`transform` and `materialize`), not
`materialize("block", ...)` pretending to be projection.

**The test:** Can the child reuse its core domain logic across scopes?
If yes → scope-switch. If no → the capability is different and should
be named differently. `materialize` must not become a junk drawer
for all write-side work.

### Domain Assumption Review

The capability protocol must be truly generic. If core starts
assuming "materialize" means SQL tables with certain shapes, or
"contribute-search" means issue/PR-style fields, we've recreated
the problem under a new name.

**materialize — verified generic:**
- Core passes (scope, source_path, destination_path). No column
  names, table names, or SQL in the invocation.
- The child opens databases/files and does its own work.
- Core reads back artifact names from the result for the registry,
  but never interprets them.
- Risk: if core validates table shapes post-materialize, domain leaks
  back. Core must treat child output as opaque.

**contribute-search — verified generic:**
- The FTS5 schema `(symbol_name, file_path, content, event_type)` is
  shared infrastructure, not domain logic. Column names are generic
  labels (a name, a locator, searchable text, a classifier).
- Core aggregates FTS5 rows. Core never interprets `event_type`
  values — it doesn't know what `github.issue` or `slack.message`
  means.
- Risk: if core uses `event_type` patterns (`LIKE '%.issue'`) to
  filter or classify, domain leaks back. The `include_issues` flag
  in search must become `include_child_data` or use registry queries.
- Risk: if the FTS5 column set expands to accommodate one connector's
  needs (e.g., adding `labels` column), it becomes domain-specific.
  The current 4-column schema is sufficient and generic.

**contract metadata — the third interface:**
- `display_kind` in the capability registry answers "what is this
  event type for display?" without core hardcoding "Issue" or "PR".
- Core queries the registry; child provides the answer at
  registration time.
- Risk: if `display_kind` grows into a rich display template with
  formatting rules, core becomes a domain renderer. Keep it to a
  single label string.

### Invocation Modes

Children are native binaries invoked via the pipe protocol. Three
invocation patterns:

**Fetch mode** (existing): Mother spawns child, child fetches from
external API, emits events via pipe protocol to destination event
store.

**Materialize mode** (new): Mother spawns child with a `materialize`
command plus scope and path env vars. Child reads from source, writes
to destination, exits.

**Search mode** (new): Mother spawns child with a `contribute-search`
command plus patina.db path. Child populates FTS5 rows, exits.

This is not a new runtime. It's the same native child binary with a
different entry point. The child manifest declares which capabilities
and scopes the child supports:

```toml
# children/github-connector/child.toml
[child]
name = "github-connector"
binary = "github-connector"

[[capabilities]]
name = "fetch"
description = "Fetch issues and PRs from GitHub API"

[[capabilities]]
name = "materialize"
description = "Project github.* events into read model tables"
scopes = ["project", "lake"]

[[capabilities]]
name = "contribute-search"
description = "Index github issues and PRs for FTS5 search"
scopes = ["project"]
```

### What Mother Does at Scrape Time (Project Consumer)

```
patina scrape (project scope):
  1. Local capture (code facts, patterns, commits, etc.) — unchanged
  2. Discover children with "materialize" capability for scope "project"
  3. For each: invoke child materialize("project", events_db, patina_db)
  4. Discover children with "contribute-search" capability
  5. For each: invoke child contribute_search(patina_db)
  6. Build embedding indices (oxidize) — children contribute via
     corpus_query (schema.toml) or contribute-search (capability)
```

### What Mother Does for Lake/Block Consumers

```
lake materialization (Mother-scheduled or on-demand):
  1. Discover children with "materialize" capability for scope "lake"
  2. For each: invoke child materialize("lake", lake_events_path, lake_storage_path)
  3. No search contribution (lake is not project-scoped)

block materialization (downstream of lake or project):
  1. Discover children with "materialize" capability for scope "block"
  2. For each: invoke child materialize("block", source_path, block_path)
  3. No search contribution (blocks are products, not search targets)
```

Core orchestrates. Children execute. Core never touches
connector-specific tables, columns, or logic. Core routes by scope
declaration, never by data content.

## Migration: Explicit Incremental Steps

The github-connector currently has one mode (fetch). It gains
materialize + contribute-search. Migration is 6 phases, each
independently verifiable. Each phase is one commit or a small
commit sequence. No phase depends on a future phase being done.

### Phase 1: Add materialize capability to github-connector

**What:** Copy projection SQL from `events.rs` into the
github-connector binary. The child gains a `materialize` mode.

**Code movement (copy, not delete):**

| Source | Destination | What |
|--------|------------|------|
| `events.rs:176-238` | `github-connector/src/materialize.rs` | `create_materialized_views()` DDL (rename tables: `forge_issues` → `github_issues`, `forge_prs` → `github_prs`) |
| `events.rs:256-336` | `github-connector/src/materialize.rs` | `project_from_events()` projection SQL (change event_type to `github.issue`/`github.pr`) |
| `events.rs:146-169` | `github-connector/src/materialize.rs` | `issue_event_exists()`/`pr_event_exists()` dedup helpers |
| `events.rs:20-75` | `github-connector/src/types.rs` | Domain types: `Issue`, `PullRequest`, `Comment`, `IssueState`, `PrState` |

**github-connector binary entry point:**

```rust
fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().collect();
    match args.get(1).map(|s| s.as_str()) {
        Some("fetch") | None => fetch_mode()?,
        Some("materialize") => materialize_mode()?,
        Some("contribute-search") => search_mode()?,
        _ => bail!("unknown mode: {}", args.get(1).unwrap_or(&String::new())),
    }
    Ok(())
}
```

**materialize_mode reads env vars for destination context:**

```rust
fn materialize_mode() -> Result<()> {
    let scope = std::env::var("PATINA_SCOPE").unwrap_or_else(|_| "project".into());
    let source = std::env::var("PATINA_SOURCE_DB")?;
    let dest = std::env::var("PATINA_DEST_DB")?;

    match scope.as_str() {
        "project" => materialize_project(&source, &dest)?,
        "lake" => materialize_lake(&source, &dest)?,
        _ => bail!("unsupported scope: {}", scope),
    }
    Ok(())
}
```

**Verify:** `cargo build -p github-connector --release && github-connector materialize`
with env vars set to a test project. Tables `github_issues`/`github_prs` should
be created and populated from events.db.

**What doesn't change:** Core's `events.rs` still works. Both paths
coexist temporarily. No deletion yet.

### Phase 2: Add contribute-search capability to github-connector

**Code movement (copy, not delete):**

| Source | Destination | What |
|--------|------------|------|
| `events.rs:506-523` | `github-connector/src/search.rs` | `populate_fts5_issues()` FTS5 indexing |
| `events.rs:528-545` | `github-connector/src/search.rs` | `populate_fts5_prs()` FTS5 indexing |

**Change FTS5 labels:** `'forge.issue'` → `'github.issue'`,
`'forge.pr'` → `'github.pr'`. The child owns its own labels.

**Verify:** `github-connector contribute-search` with PATINA_DB
set. FTS5 rows in `code_fts` should have `event_type = 'github.issue'`
and `'github.pr'`.

### Phase 3: Add capability registry to core

**New table in patina.db** (replaces/extends schema_registry):

```sql
CREATE TABLE IF NOT EXISTS capability_registry (
    child_name  TEXT NOT NULL,
    capability  TEXT NOT NULL,  -- 'fetch', 'materialize', 'contribute-search'
    scope       TEXT,           -- 'project', 'lake', 'block', NULL for all
    event_type  TEXT,           -- e.g. 'github.issue'
    contract    TEXT,           -- e.g. 'issues'
    display_kind TEXT,          -- e.g. 'Issue' (for enrichment display)
    PRIMARY KEY (child_name, capability, event_type)
);
```

**Populated from child.toml manifests.** The broker already parses
child.toml — extend it to read `[[capabilities]]` with `scopes` and
`[[contracts]]` sections, populate registry on startup.

**Verify:** `patina scrape` populates `capability_registry`.
`SELECT display_kind FROM capability_registry WHERE event_type = 'github.issue'`
returns `'Issue'`.

### Phase 4: Rewire core to use capability invocation

**Scrape:** Replace direct `project_from_events()` call with:
1. Discover children with `materialize` capability for scope `"project"`
2. For each: spawn child with `materialize` command + env vars
3. Discover children with `contribute-search` capability
4. For each: spawn child with `contribute-search` command + env vars

**Enrichment:** Replace `ends_with(".pr")` → `"PR"` with:
```sql
SELECT display_kind FROM capability_registry WHERE event_type = ?
```
Fall back to the raw event_type string if no match.

**Search:** Replace `LIKE '%.issue' OR LIKE '%.pr'` with:
```sql
event_type IN (SELECT event_type FROM capability_registry
               WHERE capability = 'contribute-search')
```
Or simply: `event_type NOT LIKE 'code.%'` (all non-code FTS5 rows
come from children).

**Oxidize:** Replace hardcoded forge corpus query with: for each
installed schema with `[embedding].corpus_query`, execute that query.

**Verify:** `patina scrape` with github-connector installed produces
the same tables, FTS5 rows, and search results. `patina scry` and
`patina assay` display forge/github events correctly.

### Phase 5: Delete connector-specific code from core

**Now safe to delete (child owns it):**

| File | What to delete |
|------|---------------|
| `events.rs:20-75` | Domain types (Issue, PullRequest, etc.) |
| `events.rs:146-169` | `issue_event_exists()`, `pr_event_exists()` |
| `events.rs:176-238` | `create_materialized_views()` |
| `events.rs:256-336` | `project_from_events()` |
| `events.rs:350-495` | `insert_issues()`, `insert_prs()` (dead code) |
| `events.rs:506-545` | `populate_fts5_issues()`, `populate_fts5_prs()` |
| `enrichment.rs:62-66` | `ends_with(".pr")` → `"PR"` classification |
| `search.rs:162-166` | `LIKE '%.issue' OR LIKE '%.pr'` filter |
| `search.rs:191-197` | `ends_with(".issue")` → `[ISSUE]` display |
| `oxidize/mod.rs:597-632` | Hardcoded forge corpus query |

**events.rs retains:** `create_schema_registry()`,
`populate_schema_registry()` (evolves into capability_registry),
`ProjectionStats` struct (used by capability invocation reporting).

**Verify:** `rg 'forge_issues\|forge_prs\|%.issue\|%.pr\|ends_with.*pr' src/`
returns zero matches. `cargo build --release` succeeds. `patina scrape`
works with github-connector providing materialization.

### Phase 6: Litmus test — add a non-forge connector

Add a minimal slack-connector (or mock connector) with completely
different domain shape (messages, not issues/PRs). The child:
- Declares `materialize` + `contribute-search` capabilities
- Creates `slack_messages` table (its own DDL)
- Contributes FTS5 rows with `event_type = 'slack.message'`

**Verify:** Zero core code changes. `patina scrape` materializes
both github and slack data. `patina assay` searches across both.
`patina scry` enrichment displays correctly via capability registry.

### Table Rename / Legacy Migration

The `forge_issues`/`forge_prs` tables cease to exist. The
github-connector creates `github_issues`/`github_prs`. Migration
is owned by the child (Phase 1):

1. On first `materialize` invocation, child checks for old
   `forge_issues` table
2. If present: `ALTER TABLE forge_issues RENAME TO github_issues`
3. Same for `forge_prs` → `github_prs`
4. Child owns this migration — core doesn't know about it

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

When children contribute FTS5 rows via `contribute-search`, they
choose their own event_type labels. The github-connector might use
`github.issue` and `github.pr`. A slack-connector might use
`slack.message`.

For enrichment (scry vector search), the key ID range check
(`key >= FORGE_ID_OFFSET`) already works generically — it looks up
the event in eventlog and uses whatever `event_type` is stored there.
The only domain-specific part is the `kind` classification ("PR" vs
"Issue"). This moves to contract metadata:

```toml
# In child manifest or schema
[[contracts]]
name = "issues"
event_type = "github.issue"
display_kind = "Issue"

[[contracts]]
name = "pull-requests"
event_type = "github.pr"
display_kind = "PR"
```

Mother stores contract metadata in the capability registry. Enrichment
queries the registry to determine display kind:

```sql
SELECT display_kind FROM capability_registry WHERE event_type = ?
```

If no match, fall back to the event_type string itself. Core never
hardcodes "Issue" or "PR".

### Search Filter

The `include_issues` flag in assay search currently maps to
`LIKE '%.issue' OR LIKE '%.pr'`. With contract-driven search, this
becomes:

```sql
-- All FTS5 rows contributed by children (vs code.* from local scrape)
event_type NOT LIKE 'code.%'
```

Or, if the user wants a specific contract:

```sql
event_type IN (SELECT event_type FROM capability_registry WHERE contract = 'issues')
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

### 1. Children Are Native Binaries, Not SQL Templates

The earlier design had core reading `[[tables]]` column definitions
from schema.toml and generating SQL. This makes core the executor of
connector-specific logic — a weaker boundary. Instead, children are
native binaries that own their SQL directly. Core invokes them.

The child binary already exists (github-connector). It gains two new
entry points (materialize, contribute-search). No new runtime needed.

### 2. Event Log Stays as Canonical Write Side

Children emit events through Mother to events.db. The CQRS audit trail
is preserved. Materialization is a separate read-side concern, invoked
after events are captured. This means:

- Events are never lost (write side is durable)
- Materialization can be re-run (idempotent)
- Multiple read models can coexist (different children, different tables)
- The event log is the source of truth, tables are derived

### 3. Capability Invocation, Not Plugin Architecture

Children are not plugins loaded into the core process. They are
separate binaries invoked via the pipe protocol with different
commands (fetch, materialize, contribute-search). This is consistent
with [[pipes-are-processes-not-wasm]] and means:

- No shared memory or process coupling
- Children can be written in any language
- Children can be updated independently
- Failure in one child doesn't crash core

### 4. schema_registry Evolves Into Capability Registry

The current `schema_registry` table maps event_type → table_name.
This evolves into a `capability_registry` that maps:

- child_name → capabilities (fetch, materialize, contribute-search)
- event_type → contract metadata (display_kind, contract_name)
- contract_name → which children can satisfy it

The registry is populated when children register (on Mother startup
or schema install), not rebuilt on every scrape.

### 5. Consumer Classes Are First-Class

Per [[pipe-architecture]] §Data Layers and [[mother-maturation]],
four consumer scopes coexist. All are first-class — the architecture
does not privilege project projection over other scopes.

| Consumer | Write side | Event source |
|----------|-----------|-------------|
| **Project** | project patina.db | project events.db (direct child fetch) |
| **Lake** | lake storage | lake event store (raw/normalized shared data) |
| **Block** | block storage | lake or project data (shaped downstream artifact) |
| **Transform** | another contract/block | child-A output (composition) |

**Same source, different consumers, different write sides.** A child
that fetches GitHub data may:
- Materialize for project scope → denormalized `github_issues` table
  in patina.db for fast project-local queries
- Materialize for lake scope → normalized rows in lake storage for
  cross-project analysis
- Not support block scope at all (Mother respects declared scopes)

The capability protocol is the same for all scopes. The child
receives `(scope, source_path, destination_path)` and decides what
to do. Mother routes by declaration, not content.

**Contracts are consumer-facing; capabilities are destination-aware.**
A contract like "issues" says what data is available. A capability
like "materialize" says how it gets written somewhere. The
destination (which consumer scope, which path) determines where.
Core connects contract + capability + destination. Core never
interprets what the child writes.

**Alignment with pipe-architecture:**
- Data Layers: Sources → Lakes → Blocks → Projects → Beliefs
- Destination Declarations: projects, data lakes, data blocks
- connector-owns-tables provides the materialization capabilities
  that pipe-architecture routes to these destinations

**Alignment with core-extraction:**
- Core = protocol + stores. Materialization is a child capability,
  not a core verb. Core orchestrates materialization through the
  pipe protocol — it doesn't own or interpret the domain logic.
- schema_registry evolving into capability_registry is consistent
  with core owning discovery (protocol) while children own domain.

## Key Files

| File | Current State | Target State | Migration Phase |
|------|---------------|--------------|-----------------|
| `src/commands/scrape/events.rs` | Domain types, DDL, projection, FTS5 | Generic capability invocation only | P1-P2 (copy), P4 (rewire), P5 (delete) |
| `children/github-connector/` | Fetch mode only | Fetch + materialize + contribute-search | P1 (materialize), P2 (search) |
| `src/commands/scry/internal/enrichment.rs` | `ends_with(".pr")` | Capability registry lookup | P4 (rewire), P5 (delete) |
| `src/commands/assay/internal/search.rs` | `LIKE '%.issue'` convention | FTS5 rows from children (no domain filter) | P4 (rewire), P5 (delete) |
| `src/commands/oxidize/mod.rs` | Hardcoded forge corpus query | Schema `corpus_query` execution | P4 (rewire), P5 (delete) |
| `src/broker/` | Child lifecycle (fetch only) | Extended for materialize/contribute-search invocation | P3 (registry), P4 (invocation) |
| `children/github-connector/child.toml` | Fetch capabilities only | Fetch + materialize + contribute-search with scopes | P1 (manifest update) |

## Scope Boundary: Invocation vs Contract Negotiation

This spec owns the **invocation protocol** — how Mother passes work
to children and how children report results. It does NOT own contract
compatibility, versioning, or type negotiation.

| Concern | Owner | Why |
|---------|-------|-----|
| `materialize(scope, source, dest)` invocation | connector-owns-tables | This spec |
| `contribute-search(dest)` invocation | connector-owns-tables | This spec |
| Capability discovery (child.toml → registry) | connector-owns-tables | This spec |
| Contract type definitions (WIT shapes) | pipe-protocol-types | Contract system |
| Schema versioning (v1 → v2 evolution) | pipe-protocol-types | Contract system |
| Compatibility checking (required/optional fields) | pipe-protocol-types | Contract system |
| Multiple children declaring same contract | pipe-protocol-types | Conflict resolution |

Without this boundary, connector-owns-tables will accidentally invent
a contract system. The invocation protocol should be stable before
the contract type system is designed — they are independent concerns.

### Minimum Viable Contract Matching (v1)

For v1, contract matching is exact string equality:

```
Consumer requests: contract = "issues", scope = "project"
Mother queries:    SELECT child_name FROM capability_registry
                   WHERE contract = 'issues' AND scope = 'project'
Match → invoke child. No match → fail with clear error.
Multiple matches → invoke all (fan-out, same as pipe-architecture).
```

This is deliberately simple. No versioning, no field negotiation, no
compatibility scoring. The contract name is the contract.

**Why this works for v1:** There is one github-connector. It declares
"issues" and "pull-requests" contracts. There are no competing
children declaring the same contract with different schemas. The
contract name is unambiguous.

**When this breaks:** When two children declare "issues" with
different field sets (github has `labels`, jira has `priority`). When
a contract schema evolves (v1 → v2) and a consumer wants v1 data but
the child now emits v2. These are real problems that belong in
[[spec-pipe-protocol-types]], not here.

**Discovery note for pipe-protocol-types:** Contract negotiation
needs to be designed. Key questions: exact contract ID vs versioned
schema? Required vs optional fields? How does a consumer discover
what shape of data a contract provides without domain knowledge in
core? This is the next real design pressure point after
connector-owns-tables lands.

## Cross-References

### [[pipe-architecture]] Alignment

- **Data Layers** (§Data Layers): Sources → Lakes → Blocks → Projects
  → Beliefs. connector-owns-tables provides the materialization
  capabilities that pipe-architecture routes to these layers.
- **Destination Declarations** (§Destination Declarations): Mother
  routes by declaration. connector-owns-tables makes capability
  invocation destination-aware via the `scope` parameter.
- **Child Taxonomy** (§Child Taxonomy): Connector children bridge
  external sources. connector-owns-tables extends their capabilities
  from fetch-only to fetch + materialize + contribute-search.
- **Pipe Protocol** (§Pipe Protocol): `pipe/capabilities` method
  already exists for capability declaration. connector-owns-tables
  adds `scopes` to capability declarations.
- **No conflict.** pipe-architecture defines the routing model;
  connector-owns-tables defines what happens at the destination.

### [[core-extraction]] Alignment

- **Protocol vs Domain** (§The Line): Core = protocol + stores.
  connector-owns-tables moves domain logic (DDL, projection, FTS5
  contribution) out of core into children. This is the materialization
  half of core-extraction's extraction targets.
- **Forge extraction** (§What ISN'T Core): connector-owns-tables
  is the specific mechanism for extracting forge domain logic.
  core-extraction identifies the 2,345 LOC target;
  connector-owns-tables defines how that code moves.
- **schema_registry → capability_registry** evolves the store from
  event-type discovery (schema-driven-projection) to full capability
  and contract registry (connector-owns-tables). Consistent with
  core owning discovery infrastructure while children own domain.

## Open Questions

1. **Database access model.** Children currently don't write to
   patina.db. Materialize requires write access. Options: (a) child
   opens patina.db directly (simple, requires path passing), (b) child
   sends SQL over pipe protocol (more isolation, more complexity).
   Recommendation: (a) for v1 — pass paths via env vars.

2. **Incremental materialization.** Full re-projection on every scrape
   is idempotent but slow for large event logs. Should the capability
   protocol include a "since" parameter? The child can track its own
   high-water mark.

3. **Capability registration timing.** When does Mother learn about
   child capabilities? Options: (a) scan child.toml on startup,
   (b) child self-registers on first invocation, (c) `patina schema install`
   also registers capabilities. Recommendation: (a) — child.toml is
   already parsed by the broker.

4. **forge legacy.** The `forge` schema describes events from the
   deleted ForgeReader (pre-v0.40). Should we keep projecting
   `forge.*` events? They exist in old event logs. Recommendation:
   the forge schema stays, a trivial forge-compat child handles
   materialization, or we accept that old forge events are queryable
   via eventlog but not materialized.

5. **Cross-child search ranking.** When multiple children contribute
   FTS5 rows, the per-table normalization in `assay_search()` needs
   to handle variable numbers of tables. Currently it normalizes
   code, commits, patterns, eventlog. Adding child-contributed tables
   requires the normalization loop to be dynamic. This is a
   mechanical change, not a design question.

6. **Lake materialization format.** Project scope materializes into
   SQLite tables in patina.db. What does lake scope materialize into?
   Options: (a) SQLite tables in a separate lake.db, (b) Parquet
   files, (c) child-defined format (core passes a directory path).
   Recommendation: defer format decision to [[spec-lake-registry]].
   connector-owns-tables provides the capability protocol; lake spec
   defines the storage format.

7. **Block scope definition.** What makes a block different from a
   lake materialization? A block is shaped for a purpose (downstream
   consumer). Options: (a) blocks are just lake materializations
   with a filter/transform, (b) blocks are a separate storage tier
   with their own lifecycle. Recommendation: defer to
   [[spec-mother-maturation]]. connector-owns-tables ensures the
   capability protocol supports blocks without defining them.

8. **Transform child invocation.** Transform children consume other
   children's output. The `source_path` for a transform is another
   child's destination. How does Mother chain these? Options:
   (a) Mother resolves the dependency graph and invokes in order,
   (b) transform children declare their source contracts and Mother
   matches. Recommendation: (a) for v1, consistent with scrape's
   sequential orchestration.
