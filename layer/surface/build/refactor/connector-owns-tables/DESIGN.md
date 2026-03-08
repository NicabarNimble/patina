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

Mother invokes after events are in events.db. Child receives:
- Path to `events.db` (read)
- Path to `patina.db` (read/write)
- Optional: last materialization timestamp (for incremental)

Child does whatever it needs:
- CREATE TABLE IF NOT EXISTS (its own tables)
- INSERT OR REPLACE from events.db eventlog
- Dedup by its own identity rules
- Schema migrations if table shape changed

Child returns:
- Count of rows materialized
- Table names created/updated (for registry)

**2. `contribute-search`**

Mother invokes after materialization. Child receives:
- Path to `patina.db` (read/write, specifically `code_fts` table)
- Its own table names (from materialize result)

Child does:
- DELETE its own FTS5 rows (by its own label)
- INSERT FTS5 rows from its read model tables
- Choose which fields to index, what labels to use

Child returns:
- Count of documents contributed

### Invocation Modes

Children are native binaries invoked via the pipe protocol. Two
invocation patterns:

**Fetch mode** (existing): Mother spawns child, child fetches from
external API, emits events via pipe protocol to events.db.

**Materialize mode** (new): Mother spawns child with a `materialize`
or `contribute-search` command. Child receives database paths, does
its work, exits.

This is not a new runtime. It's the same native child binary with a
different entry point. The child manifest declares which capabilities
the child supports:

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
description = "Project github.* events into github_issues/github_prs tables"

[[capabilities]]
name = "contribute-search"
description = "Index github issues and PRs for FTS5 search"
```

### What Mother Does at Scrape Time

```
patina scrape:
  1. Local capture (code facts, patterns, commits, etc.) — unchanged
  2. Discover children with "materialize" capability
  3. For each: invoke child materialize(events_db, patina_db)
  4. Discover children with "contribute-search" capability
  5. For each: invoke child contribute_search(patina_db)
  6. Build embedding indices (oxidize) — children contribute via corpus_query or contribute-search
```

Core orchestrates. Children execute. Core never touches
connector-specific tables, columns, or logic.

## Migration: github-connector Gains Capabilities

### Code Movement

The github-connector binary currently has one mode: fetch issues/PRs
from GitHub API, emit events. It gains two more modes:

**From `events.rs` → `github-connector`:**
- `create_materialized_views()` — the issue/PR table DDL
- `project_from_events()` — the issue/PR projection SQL
- `issue_event_exists()` / `pr_event_exists()` — dedup helpers
- `populate_fts5_issues()` / `populate_fts5_prs()` — FTS5 contributions
- Domain types: `Issue`, `PullRequest`, `Comment`, `IssueState`, `PrState`

**From `events.rs` → deleted (no new home):**
- `insert_issues()` / `insert_prs()` — the ForgeReader insert path (dead code, ForgeReader is deleted)

**From `enrichment.rs` → contract metadata or child:**
- `ends_with(".pr")` kind detection — replaced by contract-level metadata ("this contract's fact type is PR")

**From `search.rs` → generic capability:**
- `LIKE '%.issue' OR LIKE '%.pr'` — replaced by querying FTS5 rows contributed by children (no domain filter needed)

### github-connector Binary Changes

```rust
fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().collect();

    match args.get(1).map(|s| s.as_str()) {
        Some("fetch") | None => fetch_mode()?,       // existing
        Some("materialize") => materialize_mode()?,   // new
        Some("contribute-search") => search_mode()?,  // new
        _ => bail!("unknown mode"),
    }

    Ok(())
}

fn materialize_mode() -> Result<()> {
    let events_db = std::env::var("PATINA_EVENTS_DB")?;
    let patina_db = std::env::var("PATINA_DB")?;

    let conn = Connection::open(&patina_db)?;

    // Create tables (child owns DDL)
    conn.execute_batch("
        CREATE TABLE IF NOT EXISTS github_issues (
            number INTEGER PRIMARY KEY,
            title TEXT NOT NULL,
            body TEXT,
            state TEXT NOT NULL,
            labels TEXT,
            author TEXT,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL,
            url TEXT NOT NULL,
            event_seq INTEGER,
            ingested_at TEXT
        );
        CREATE TABLE IF NOT EXISTS github_prs (...);
    ")?;

    // Project (child owns column mappings + dedup)
    conn.execute("ATTACH DATABASE ?1 AS events_db", [&events_db])?;
    conn.execute(
        "INSERT OR REPLACE INTO github_issues (...)
         SELECT json_extract(e.data, '$.number'), ...
         FROM events_db.eventlog e
         WHERE e.event_type IN ('github.issue')
           AND e.seq = (SELECT MAX(e2.seq) ...)",
        [],
    )?;
    // ... same for github_prs ...
    conn.execute("DETACH DATABASE events_db", [])?;

    Ok(())
}
```

This is the same SQL that lives in `events.rs` today — it just moves
to the child binary. The logic is unchanged. The ownership changes.

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

## Table Rename / Migration

The `forge_issues` / `forge_prs` tables cease to exist when projection
moves to the github-connector. The child creates `github_issues` /
`github_prs` (or whatever it chooses). Migration:

1. On first `materialize` invocation, child checks for old
   `forge_issues` table
2. If present: `ALTER TABLE forge_issues RENAME TO github_issues`
3. Child owns this migration — core doesn't know about it

The `forge` schema (describing legacy `forge.*` events) stays in
`.patina/schemas/forge/` for backward compatibility with old events
in events.db. But no new events use `forge.*` — the github-connector
emits `github.*`.

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

### 5. Two Data Modes Coexist

Per [[data-architecture-v3]] and [[mother-maturation]]:

- **Direct:** child → events.db → child.materialize() → patina.db
- **Lake:** child → events.db (lake) → block → project.materialize()

Both use the same materialize capability. The child doesn't care
whether its events came from a direct fetch or a lake extraction. The
capability protocol is the same.

## Key Files

| File | Current State | Target State |
|------|---------------|--------------|
| `src/commands/scrape/events.rs` | Domain types, DDL, projection, FTS5 | Generic capability invocation only |
| `children/github-connector/` | Fetch mode only | Fetch + materialize + contribute-search |
| `src/commands/scry/internal/enrichment.rs` | `ends_with(".pr")` | Capability registry lookup |
| `src/commands/assay/internal/search.rs` | `LIKE '%.issue'` convention | FTS5 rows from children (no domain filter) |
| `src/commands/oxidize/mod.rs` | Hardcoded forge corpus query | Schema `corpus_query` execution |
| `src/broker/` | Child lifecycle (fetch only) | Extended for materialize/contribute-search |

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
