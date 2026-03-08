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
beliefs:
- connectors-own-tables-schemas-are-contracts
- patina-is-domain-agnostic-knowledge-system
exit_criteria:
- id: schema-declares-ddl
  text: "schema.toml includes a [[tables]] section with CREATE TABLE DDL; pipeline creates tables from schema, not hardcoded in events.rs"
  checked: false
- id: forge-tables-renamed
  text: "forge_issues/forge_prs renamed to github_issues/github_prs (or generic issues/prs); forge schema updated; migration handles existing data"
  checked: false
- id: projection-shape-from-schema
  text: "project_from_events() reads column mappings from schema, not hardcoded SELECT/INSERT column lists"
  checked: false
- id: non-forge-connector-works
  text: "A non-forge connector (e.g. slack) with a different data shape (messages, not issues) installs and projects with zero core code changes"
  checked: false
- id: multi-project-isolation
  text: "Two projects with different installed schemas materialize different tables from the same event stream"
  checked: false
---
# refactor: Connector-Owns-Tables — Schema-Driven DDL and Domain-Specific Materialized Views

> Replace shared forge_issues/forge_prs tables with connector-declared tables. Each schema.toml declares its own DDL, table names, and projection shape. Non-forge connectors (Slack, Google Workspace) get first-class support without fitting into issue/PR shapes.

## Current State

`schema-driven-projection` removed hardcoded event type strings — the pipeline now discovers event types from installed schemas via the `schema_registry` table. But three things remain hardcoded:

1. **Table DDL** — `create_materialized_views()` in `events.rs` hardcodes `CREATE TABLE forge_issues (...)` and `CREATE TABLE forge_prs (...)` with fixed column sets
2. **Projection shape** — `project_from_events()` hardcodes which JSON fields map to which columns (issue shape vs PR shape)
3. **Table names** — everything is called `forge_*`, a legacy name from when there was only one source

The schema system declares `table = "forge_issues"` but doesn't declare the table's structure or column mappings. Adding a Slack connector would require core code changes because Slack messages have different fields than GitHub issues.

## Target State

Each connector's schema.toml is a complete contract:

```toml
# .patina/schemas/slack/schema.toml
[schema]
name = "slack"

[[facts]]
name = "message"
event_type = "slack.message"
record = "message"

[[tables]]
fact = "message"
name = "slack_messages"
columns = [
    { name = "ts", source = "$.ts", type = "TEXT PRIMARY KEY" },
    { name = "channel", source = "$.channel", type = "TEXT NOT NULL" },
    { name = "user", source = "$.user", type = "TEXT" },
    { name = "text", source = "$.text", type = "TEXT" },
    { name = "thread_ts", source = "$.thread_ts", type = "TEXT" },
]

[[indexes]]
fact = "message"
table = "slack_messages"
fts_fields = ["text"]
```

The pipeline reads the schema to:
1. **Create tables** — generate DDL from `[[tables]]` columns
2. **Project events** — build INSERT/SELECT from `columns[].source` (JSON path) → `columns[].name`
3. **Index for search** — FTS5 fields from `[[indexes]]`

A new connector ships schema.toml + child binary. Zero core code changes.

## Steps

### 1. Extend schema.toml format with [[tables]] section

Add `[[tables]]` to the schema format with column definitions including JSON source paths and SQL types. Update `SchemaMetadata` parsing.

### 2. Schema-driven table creation

Replace `create_materialized_views()` hardcoded DDL with dynamic DDL generated from `[[tables]]` in installed schemas.

### 3. Schema-driven projection

Replace hardcoded column mappings in `project_from_events()` with dynamic INSERT/SELECT generated from `columns[].source` → `columns[].name` mappings.

### 4. Rename forge tables

Rename `forge_issues` → `github_issues`, `forge_prs` → `github_prs` (or keep as `issues`/`prs` if shared). Update github and forge schemas. Add migration for existing data.

### 5. Litmus test with non-forge connector

Create a mock Slack schema with a completely different data shape. Install it. Verify tables are created and events project with zero core changes.

## Exit Criteria

- **schema-declares-ddl:** schema.toml includes [[tables]] with DDL; pipeline creates tables from schema
- **forge-tables-renamed:** forge_issues/forge_prs renamed; migration handles existing data
- **projection-shape-from-schema:** column mappings read from schema, not hardcoded
- **non-forge-connector-works:** non-forge connector with different shape projects with zero core changes
- **multi-project-isolation:** two projects with different schemas materialize different tables
