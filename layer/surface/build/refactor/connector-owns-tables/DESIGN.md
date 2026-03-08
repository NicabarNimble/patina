# Design: Connector-Owns-Tables — Schema-Driven DDL and Domain-Specific Materialized Views

## Why This Work Exists

[[schema-driven-projection]] removed hardcoded event type strings from
the pipeline. Projection now discovers event types from installed
schemas via the `schema_registry` table. But three things remain
hardcoded in `src/commands/scrape/events.rs`:

1. **Table DDL.** `create_materialized_views()` hardcodes two CREATE
   TABLE statements with fixed column sets — issue shape and PR shape.
   A Slack connector can't project `slack.message` events because
   there's no `slack_messages` table and no way to declare one.

2. **Projection shape.** `project_from_events()` hardcodes which JSON
   fields map to which columns: `json_extract(e.data, '$.number')` →
   `number`, `json_extract(e.data, '$.title')` → `title`, etc. A
   different data shape requires different mappings.

3. **Table naming.** Everything is called `forge_*` — a name from the
   WASM plugin era when there was only one source. GitHub issues sit
   in `forge_issues`. This is confusing and wrong.

[[connectors-own-tables-schemas-are-contracts]] captures the principle:
each connector declares its own materialized tables via schema.toml.
Schemas are contracts between producer (connector) and consumer
(project), not shared infrastructure.

**Origin:** [[session-20260308-070818]] — during schema-driven-projection
work, user identified that `forge_issues`/`forge_prs` shared tables
can't support non-forge connectors. A Google Workspace or Slack
connector would need its own table shapes. The discussion established
Option B (connector-owns-tables) over Option A (shared domain tables).

## What Exists Today

### Table DDL (hardcoded)

`create_materialized_views()` in `events.rs:176` creates two tables:

```sql
CREATE TABLE IF NOT EXISTS forge_issues (
    number INTEGER PRIMARY KEY,
    title TEXT NOT NULL,
    body TEXT,
    state TEXT NOT NULL,
    labels TEXT,           -- JSON array
    author TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    url TEXT NOT NULL,
    event_seq INTEGER,
    ingested_at TEXT
);

CREATE TABLE IF NOT EXISTS forge_prs (
    number INTEGER PRIMARY KEY,
    title TEXT NOT NULL,
    body TEXT,
    state TEXT NOT NULL,
    labels TEXT,           -- JSON array
    author TEXT,
    created_at TEXT NOT NULL,
    merged_at TEXT,
    url TEXT NOT NULL,
    linked_issues TEXT,    -- JSON array
    approvals INTEGER DEFAULT 0,
    event_seq INTEGER,
    ingested_at TEXT
);
```

These are the only two materialized view shapes. Adding a third
requires editing this function.

### Projection (hardcoded column mappings)

`project_from_events()` in `events.rs:258` has two SQL statements,
each with hardcoded `json_extract` → column mappings:

```sql
-- Issue projection (13 columns, 13 json_extract calls)
INSERT OR REPLACE INTO forge_issues
    (number, title, body, state, labels, author, created_at, updated_at,
     url, event_seq, ingested_at)
    SELECT
        json_extract(e.data, '$.number'),
        json_extract(e.data, '$.title'),
        ...
```

A connector with different JSON field names (e.g., Slack's `$.ts`,
`$.channel`, `$.text`) can't use this projection.

### Schema Registry (already dynamic)

The `schema_registry` table maps `event_type` → `table_name`:

```
forge  | forge.issue  | forge_issues
forge  | forge.pr     | forge_prs
github | github.issue | forge_issues
github | github.pr    | forge_prs
```

This tells the projection *which* table to use, but not *how* to
build the table or *how* to extract columns from the JSON.

### FTS5 (hardcoded label, partially coupled)

`populate_fts5_issues()` and `populate_fts5_prs()` use fixed labels
(`'forge.issue'`, `'forge.pr'`) and read from fixed tables
(`forge_issues`, `forge_prs`). The label is a display tag — DELETE
and INSERT match on it consistently (fixed in P1 bugfix). But FTS5
still assumes two tables with known column names.

## Target State

### Extended schema.toml Format

Each schema declares its tables with column definitions and JSON
source paths:

```toml
# .patina/schemas/github/schema.toml

[schema]
name = "github"
version = "2.0.0"
package = "patina:schema/github@2.0.0"

[[facts]]
name = "issue"
event_type = "github.issue"
record = "issue"

[[facts]]
name = "pull-request"
event_type = "github.pr"
record = "pull-request"

[[tables]]
fact = "issue"
name = "github_issues"
columns = [
    { name = "number",     source = "$.number",     type = "INTEGER PRIMARY KEY" },
    { name = "title",      source = "$.title",      type = "TEXT NOT NULL" },
    { name = "body",       source = "$.body",        type = "TEXT" },
    { name = "state",      source = "$.state",       type = "TEXT NOT NULL" },
    { name = "labels",     source = "$.labels",      type = "TEXT" },
    { name = "author",     source = "$.author",      type = "TEXT" },
    { name = "created_at", source = "$.created_at",  type = "TEXT NOT NULL" },
    { name = "updated_at", source = "$.updated_at",  type = "TEXT NOT NULL" },
    { name = "url",        source = "$.url",          type = "TEXT NOT NULL" },
]

[[tables]]
fact = "pull-request"
name = "github_prs"
columns = [
    { name = "number",        source = "$.number",        type = "INTEGER PRIMARY KEY" },
    { name = "title",         source = "$.title",         type = "TEXT NOT NULL" },
    { name = "body",          source = "$.body",           type = "TEXT" },
    { name = "state",         source = "$.state",          type = "TEXT NOT NULL" },
    { name = "labels",        source = "$.labels",         type = "TEXT" },
    { name = "author",        source = "$.author",         type = "TEXT" },
    { name = "created_at",    source = "$.created_at",     type = "TEXT NOT NULL" },
    { name = "merged_at",     source = "$.merged_at",      type = "TEXT" },
    { name = "url",           source = "$.url",             type = "TEXT NOT NULL" },
    { name = "linked_issues", source = "$.linked_issues",  type = "TEXT" },
    { name = "approvals",     source = "$.approvals",      type = "INTEGER DEFAULT 0" },
]

[[indexes]]
fact = "issue"
table = "github_issues"
fts_fields = ["title", "body"]

[[indexes]]
fact = "pull-request"
table = "github_prs"
fts_fields = ["title", "body"]

[embedding]
offset_slot = 5
corpus_query = """
SELECT seq,
       json_extract(data, '$.title') || ' ' ||
       COALESCE(json_extract(data, '$.body'), '')
       as content
FROM eventlog
WHERE event_type LIKE 'github.%'
"""
```

A completely different domain works without core changes:

```toml
# .patina/schemas/slack/schema.toml

[schema]
name = "slack"
version = "1.0.0"
package = "patina:schema/slack@1.0.0"

[[facts]]
name = "message"
event_type = "slack.message"
record = "message"

[[tables]]
fact = "message"
name = "slack_messages"
columns = [
    { name = "ts",        source = "$.ts",        type = "TEXT PRIMARY KEY" },
    { name = "channel",   source = "$.channel",   type = "TEXT NOT NULL" },
    { name = "user_name", source = "$.user",      type = "TEXT" },
    { name = "text",      source = "$.text",       type = "TEXT" },
    { name = "thread_ts", source = "$.thread_ts", type = "TEXT" },
]

[[indexes]]
fact = "message"
table = "slack_messages"
fts_fields = ["text"]

[embedding]
offset_slot = 7
corpus_query = """
SELECT seq,
       COALESCE(json_extract(data, '$.text'), '')
       as content
FROM eventlog
WHERE event_type LIKE 'slack.%'
"""
```

### Dynamic Table Creation

Replace `create_materialized_views()` with a schema-driven function:

```rust
fn create_tables_from_schemas(conn: &Connection) -> Result<()> {
    let schemas = crate::commands::schema::load_all_installed()?;

    for schema in &schemas {
        for table_def in &schema.tables {
            // Build DDL from column definitions
            let columns: Vec<String> = table_def.columns.iter()
                .map(|c| format!("    {} {}", c.name, c.col_type))
                .collect();

            // Always add event_seq and ingested_at tracking columns
            let ddl = format!(
                "CREATE TABLE IF NOT EXISTS {} (\n{},\n    event_seq INTEGER,\n    ingested_at TEXT\n)",
                table_def.name,
                columns.join(",\n")
            );

            conn.execute_batch(&ddl)?;
        }
    }
    Ok(())
}
```

The existing `create_materialized_views()` remains as a fallback for
projects with no installed schemas (backward compatibility), or is
removed entirely if we commit to schema-driven.

### Dynamic Projection

Replace hardcoded INSERT/SELECT with schema-driven column mappings:

```rust
fn project_table(
    conn: &Connection,
    table_def: &TableDef,
    event_types: &[String],
) -> Result<usize> {
    // Build column list and json_extract list from schema
    let col_names: Vec<&str> = table_def.columns.iter()
        .map(|c| c.name.as_str())
        .collect();
    let extracts: Vec<String> = table_def.columns.iter()
        .map(|c| format!("json_extract(e.data, '{}')", c.source))
        .collect();

    // Build the event_type IN-list from registry
    let placeholders: String = event_types.iter()
        .map(|_| "?").collect::<Vec<_>>().join(",");

    // Find the primary key column for dedup
    let pk_col = table_def.columns.iter()
        .find(|c| c.col_type.contains("PRIMARY KEY"))
        .map(|c| &c.source)
        .unwrap_or(&table_def.columns[0].source);

    let sql = format!(
        "INSERT OR REPLACE INTO {} ({}, event_seq, ingested_at)
         SELECT {}, e.seq, e.timestamp
         FROM events_db.eventlog e
         WHERE e.event_type IN ({})
           AND e.seq = (
             SELECT MAX(e2.seq) FROM events_db.eventlog e2
             WHERE e2.event_type IN ({})
               AND json_extract(e2.data, '{}') = json_extract(e.data, '{}')
           )",
        table_def.name,
        col_names.join(", "),
        extracts.join(", "),
        placeholders,
        placeholders,
        pk_col,
        pk_col,
    );

    let params: Vec<&dyn rusqlite::types::ToSql> = event_types.iter()
        .map(|s| s as &dyn rusqlite::types::ToSql)
        .collect();

    // Duplicate params for both IN-lists
    let mut all_params = params.clone();
    all_params.extend(params);

    Ok(conn.execute(&sql, all_params.as_slice())?)
}
```

### Dynamic FTS5

FTS5 indexing reads from the schema-declared tables instead of
hardcoded `forge_issues`/`forge_prs`:

```rust
fn populate_fts5_from_schemas(conn: &Connection) -> Result<usize> {
    let schemas = crate::commands::schema::load_all_installed()?;
    let mut total = 0;

    for schema in &schemas {
        for index_cfg in &schema.indexes {
            let table_def = schema.tables.iter()
                .find(|t| t.fact == index_cfg.fact);
            let table_name = &index_cfg.table;

            // Use first fts_field as symbol_name, rest as content
            let symbol_field = &index_cfg.fts_fields[0];
            let content_fields: Vec<String> = index_cfg.fts_fields[1..].iter()
                .map(|f| format!("COALESCE({}, '')", f))
                .collect();
            let content_expr = if content_fields.is_empty() {
                "''".to_string()
            } else {
                content_fields.join(" || ' ' || ")
            };

            // Use table name as the FTS5 event_type label
            let label = table_name;

            conn.execute(
                &format!("DELETE FROM code_fts WHERE event_type = '{}'", label),
                [],
            )?;

            let count = conn.execute(
                &format!(
                    "INSERT INTO code_fts (symbol_name, file_path, content, event_type)
                     SELECT {symbol_field}, COALESCE(url, ''), {content_expr}, '{label}'
                     FROM {table_name}",
                ),
                [],
            )?;

            total += count;
        }
    }

    Ok(total)
}
```

### Enrichment and Search (Registry-Driven)

The convention-based approach (`ends_with(".pr")`, `LIKE '%.issue'`)
is replaced with registry lookups. The `schema_registry` table already
has `fact_name` which distinguishes fact types:

```rust
// Load known fact types from registry at query time
let issue_types: HashSet<String> = conn.prepare(
    "SELECT event_type FROM schema_registry WHERE fact_name = 'issue'"
)?.query_map([], |r| r.get(0))?.filter_map(|r| r.ok()).collect();

let kind = if issue_types.contains(&event_type) { "Issue" } else { "PR" };
```

For assay search FTS5 filter, the event_type filter becomes:

```sql
event_type LIKE 'code.%'
OR event_type IN (SELECT table_name FROM schema_registry)
```

This replaces the `LIKE '%.issue' OR LIKE '%.pr'` convention that
only works for forge-family naming.

## Migration

### Table Rename

Existing `forge_issues`/`forge_prs` tables must be renamed. Two options:

| Option | DDL | Risk |
|--------|-----|------|
| ALTER TABLE RENAME | `ALTER TABLE forge_issues RENAME TO github_issues` | Low — SQLite supports this natively since 3.25.0 |
| Copy + drop | `CREATE TABLE github_issues AS SELECT * FROM forge_issues; DROP TABLE forge_issues` | Loses indexes, constraints |

**Recommendation:** ALTER TABLE RENAME. Run as a one-time migration
during scrape if old table exists and new doesn't.

```rust
fn migrate_forge_tables(conn: &Connection) -> Result<()> {
    // Only migrate if old tables exist and new ones don't
    let has_old: bool = conn.query_row(
        "SELECT COUNT(*) FROM sqlite_master WHERE name = 'forge_issues'",
        [], |r| r.get::<_, i64>(0),
    ).map(|c| c > 0).unwrap_or(false);

    let has_new: bool = conn.query_row(
        "SELECT COUNT(*) FROM sqlite_master WHERE name = 'github_issues'",
        [], |r| r.get::<_, i64>(0),
    ).map(|c| c > 0).unwrap_or(false);

    if has_old && !has_new {
        conn.execute_batch("
            ALTER TABLE forge_issues RENAME TO github_issues;
            ALTER TABLE forge_prs RENAME TO github_prs;
        ")?;
        eprintln!("Migrated forge_issues → github_issues, forge_prs → github_prs");
    }
    Ok(())
}
```

### Schema Update

The `forge` schema stays as-is (describes legacy `forge.*` events).
The `github` schema updates its `[[indexes]]` table names:

```toml
# Before
[[indexes]]
fact = "issue"
table = "forge_issues"   # shared legacy name

# After
[[indexes]]
fact = "issue"
table = "github_issues"  # connector-specific
```

### forge_refs Table

`forge_refs` (incremental sync backlog) also lives in
`create_materialized_views()`. This table is GitHub-specific (repo +
ref_number + ref_kind) and should move into the github schema's
`[[tables]]` section or into the github-connector's own setup.

## Design Decisions

### 1. Schemas Declare DDL, Not Shapes

The `[[tables]]` section is explicit DDL — column names, SQL types,
JSON source paths. This is more verbose than a "shape" abstraction
but avoids inventing a type system. SQLite already has one. The schema
just declares how to use it.

The `source` field is a `json_extract` path. This is a direct mapping
— no transformation, no computed columns. If a connector needs
computed values (e.g., `CASE ... WHEN`), it should compute them
before emitting the event, not in the schema.

### 2. event_seq and ingested_at Are Implicit

Every materialized table gets `event_seq INTEGER` and
`ingested_at TEXT` columns automatically. These are not declared in
the schema because they're infrastructure — the cross-db reference
to events.db and the insertion timestamp. The projection engine adds
them to every table.

### 3. Primary Key Drives Dedup

The projection's dedup logic (`AND e.seq = (SELECT MAX ...)`) needs
to know which field uniquely identifies a record. The primary key
column from the DDL serves this purpose. `json_extract(e2.data, '$.number')`
becomes `json_extract(e2.data, columns[pk].source)`.

If a table has a composite primary key, the dedup query must match
on all PK columns. This is an open question — single PK covers all
current use cases.

### 4. FTS5 Labels Use Table Names

The FTS5 event_type label switches from connector-specific strings
(`'forge.issue'`) to table names (`'github_issues'`). This is more
stable — the table name is the canonical identifier for a materialized
view. The label only needs to be consistent between DELETE and INSERT.

### 5. Backward Compatibility via Fallback

Projects without any installed schemas still work. If
`load_all_installed()` returns empty, the pipeline falls back to the
hardcoded `create_materialized_views()`. This allows gradual migration
— existing projects keep working, new projects use schema-driven
tables from the start.

This fallback is temporary. Once all projects have installed schemas,
the hardcoded DDL can be deleted.

## Key Files

| File | Current State | Target State |
|------|---------------|--------------|
| `src/commands/schema/internal.rs` | Parses schema.toml (facts, indexes, embedding) | Also parses `[[tables]]` with column defs |
| `src/commands/scrape/events.rs` | Hardcoded CREATE TABLE + INSERT/SELECT | Schema-driven table creation + projection |
| `src/commands/assay/internal/search.rs` | Convention: `LIKE '%.issue'` | Registry: `IN (SELECT table_name ...)` |
| `src/commands/scry/internal/enrichment.rs` | Convention: `ends_with(".pr")` | Registry: lookup from `schema_registry` |
| `src/commands/oxidize/mod.rs` | Convention: `LIKE '%.issue'` | Already uses `corpus_query` from schema; convention remains for kind detection |
| `wit/schema/github/schema.toml` | Declares facts + indexes + embedding | Also declares `[[tables]]` with column defs |
| `wit/schema/forge/schema.toml` | Declares facts + indexes + embedding | Unchanged (legacy events) |

## Open Questions

1. **Composite primary keys.** Current dedup assumes a single PK
   column. Slack messages use `ts` (unique within channel) but need
   `(channel, ts)` for global uniqueness. Should the schema declare
   explicit dedup keys separate from the DDL primary key?

2. **COALESCE and defaults.** The current projection uses `COALESCE`
   for several columns (e.g., `COALESCE(json_extract(...), '[]')`).
   Should the schema declare default values per column, or should the
   connector guarantee well-formed JSON?

3. **Table versioning.** If a schema updates its table columns
   (v1 → v2), how does the pipeline handle the migration? Options:
   drop + recreate (lossy), ALTER TABLE ADD COLUMN (additive only),
   or versioned table names (`github_issues_v2`).

4. **Multi-project isolation testing.** EC5 requires proving two
   projects with different schemas materialize different tables. This
   needs the `patina repo` infrastructure to be functional. May need
   to defer or simplify this EC.

5. **forge_refs ownership.** The `forge_refs` table is used by the
   incremental sync system (`forge_refs_pending`). Does it belong in
   the github schema, in a separate sync infrastructure, or stay
   hardcoded as pipeline infrastructure?
