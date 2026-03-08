---
type: refactor
id: schema-driven-projection
status: active
created: 2026-03-08
sessions:
  origin: 20260307-234302
related:
- pipe-architecture
- core-extraction
- github-child-owns-forge
exit_criteria:
- id: projection-reads-schemas
  text: project_from_events() discovers event_type → table mappings from installed schemas, not hardcoded strings
  checked: true
- id: fts5-reads-schemas
  text: FTS5 populate discovers event types and field definitions from installed schema index config
  checked: true
- id: search-discovers-event-types
  text: scry enrichment and assay search discover forge-family event types from schemas, not hardcoded matches
  checked: true
- id: oxidize-reads-corpus-query
  text: oxidize builds embedding corpus using corpus_query from installed schemas
  checked: true
- id: litmus-new-connector
  text: Install a new schema.toml (e.g. gitea) with different event types — pipeline handles it with zero core code changes
  checked: true
---
# refactor: Schema-Driven Projection — Pipeline Reads Schemas, Not Hardcoded Event Types

> Projection, FTS5, search, and oxidize hardcode event type strings (forge.issue, github.issue). A new connector (gitea, gitlab) requires modifying core code. The schema system already declares event_type → table mappings — the pipeline should read them.

## Current State

The pipeline has **8+ hardcoded event type strings** across 4 subsystems:

| Subsystem | File | What's hardcoded |
|-----------|------|-----------------|
| Projection | `src/commands/scrape/events.rs` | `IN ('forge.issue', 'github.issue')` in 6 WHERE clauses |
| Scry enrichment | `src/commands/scry/internal/enrichment.rs` | `== "forge.pr" \|\| == "github.pr"` |
| Assay search | `src/commands/assay/internal/search.rs` | `LIKE 'forge.%' OR LIKE 'github.%'` |
| Oxidize | `src/commands/oxidize/mod.rs` | `IN ('forge.issue', 'forge.pr', 'github.issue', 'github.pr')` |

Meanwhile, the **schema system already declares** all the needed metadata:

```toml
# wit/schema/github/schema.toml
[[facts]]
event_type = "github.issue"

[[indexes]]
fact = "issue"
table = "forge_issues"
fts_fields = ["title", "body"]

[embedding]
corpus_query = "SELECT ... WHERE event_type LIKE 'github.%'"
```

The schema metadata exists but the pipeline ignores it. Every new connector requires core code changes.

## Target State

The pipeline reads installed schemas to discover:
1. Which event types exist (e.g. `github.issue`, `gitea.issue`, `forge.pr`)
2. Which materialized view tables they project into (`forge_issues`, `forge_prs`)
3. Which FTS5 fields to index
4. Which corpus queries to use for embeddings

A new connector ships a `schema.toml` + child binary. `patina schema install` makes it known to the pipeline. Zero core code changes.

## Steps

### 1. Schema registry table in patina.db

Create a `schema_registry` table populated from installed schemas:
```sql
CREATE TABLE schema_registry (
    schema_name TEXT NOT NULL,
    event_type  TEXT NOT NULL PRIMARY KEY,
    table_name  TEXT NOT NULL,
    fact_name   TEXT NOT NULL,
    kind        TEXT NOT NULL  -- 'issue' or 'pr' (determines projection shape)
);
```

Populated on `patina scrape` from `.patina/schemas/*/schema.toml`.

### 2. Projection reads registry

`project_from_events()` queries `schema_registry` to build the event type IN-lists dynamically:
```sql
SELECT event_type FROM schema_registry WHERE table_name = 'forge_issues'
-- → ('forge.issue', 'github.issue', 'gitea.issue', ...)
```

### 3. FTS5 reads registry

`populate_fts5_issues()` / `populate_fts5_prs()` use schema_registry to determine which event types to clear and which table to read from.

### 4. Search reads registry

Scry enrichment and assay search discover forge-family event types from the registry instead of hardcoded string matches.

### 5. Oxidize reads corpus_query

Oxidize builds the forge/issue/PR embedding corpus by reading `corpus_query` from each installed schema's embedding config.

## Exit Criteria

- **projection-reads-schemas:** `project_from_events()` discovers event_type → table mappings from installed schemas
- **fts5-reads-schemas:** FTS5 populate discovers event types and field definitions from installed schema index config
- **search-discovers-event-types:** scry enrichment and assay search discover forge-family event types from schemas
- **oxidize-reads-corpus-query:** oxidize builds embedding corpus using corpus_query from installed schemas
- **litmus-new-connector:** Install a new schema.toml with different event types — pipeline handles it with zero core code changes
