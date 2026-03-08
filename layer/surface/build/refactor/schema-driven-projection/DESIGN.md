# Design: Schema-Driven Projection

## History

The projection layer evolved through three eras:

1. **Era 1 (pre-v0.25):** ForgeReader fetched issues/PRs directly into materialized views. Event types were `forge.issue` / `forge.pr`. One source, hardcoded in core. Worked well.

2. **Era 2 (v0.25-v0.39):** WASM grammar-forge plugin tagged staging files, pipeline routed to insert helpers. Still `forge.*` event types, still hardcoded.

3. **Era 3 (v0.40+):** github-connector native child emits `github.issue` / `github.pr` to events.db. Projection was hardcoded to `forge.*` only — data was invisible. Fixed by adding `IN ('forge.issue', 'github.issue')` to 8+ locations. This works but doesn't scale.

**The pattern:** every new connector means editing core SQL in 4 subsystems. The schema system was designed to prevent this, but the pipeline never reads it.

## Approach

### Schema Registry Table

On `patina scrape`, read all installed schemas from `.patina/schemas/*/schema.toml` and populate a `schema_registry` table:

```sql
CREATE TABLE IF NOT EXISTS schema_registry (
    schema_name TEXT NOT NULL,        -- e.g. 'github', 'forge', 'gitea'
    event_type  TEXT NOT NULL PRIMARY KEY,  -- e.g. 'github.issue'
    fact_name   TEXT NOT NULL,        -- e.g. 'issue', 'pull-request'
    table_name  TEXT NOT NULL,        -- e.g. 'forge_issues'
    kind        TEXT NOT NULL,        -- 'issue' or 'pr'
    fts_fields  TEXT,                 -- JSON array: ["title", "body"]
    corpus_query TEXT,                -- SQL for oxidize embedding corpus
    offset_slot INTEGER               -- embedding ID offset slot
);
```

This is rebuilt every scrape (idempotent, cheap).

### Dynamic Event Type Discovery

Instead of:
```sql
WHERE e.event_type IN ('forge.issue', 'github.issue')
```

The projection engine does:
```sql
WHERE e.event_type IN (
    SELECT event_type FROM schema_registry WHERE kind = 'issue'
)
```

Or builds the IN-list in Rust by querying the registry once at the start of projection.

### Scry / Assay

The enrichment and search code currently matches event types with `==` or `LIKE`. Replace with:
- A set of known issue/PR event types loaded from `schema_registry` at query time
- Or a convention: anything ending in `.issue` is an issue, `.pr` is a PR

Convention-based (`LIKE '%.issue'`) is simpler and requires no registry lookup, but is less explicit. Registry-based is more correct but adds a query.

**Recommendation:** Convention-based for search (pattern match on suffix), registry-based for projection (needs table mapping).

### Oxidize

The schema.toml `[embedding].corpus_query` already contains the SQL needed. Oxidize should:
1. Read all installed schemas
2. For each schema with an `[embedding]` section, run its `corpus_query`
3. Use `offset_slot` to key the results into the right ID space

This replaces the hardcoded forge event query in `oxidize/mod.rs`.

## Key Files

| File | Current | Target |
|------|---------|--------|
| `src/commands/scrape/events.rs` | Hardcoded IN-lists | Registry-driven projection |
| `src/commands/scry/internal/enrichment.rs` | `== "forge.pr"` | Convention or registry lookup |
| `src/commands/assay/internal/search.rs` | `LIKE 'forge.%'` | `LIKE '%.issue' OR LIKE '%.pr'` |
| `src/commands/oxidize/mod.rs` | Hardcoded event types + SQL | Schema corpus_query |
| `src/commands/schema/internal.rs` | Schema install/validate | Also populates registry table |

## Open Questions

1. **Convention vs registry for search:** `LIKE '%.issue'` is simpler but assumes naming convention. Registry lookup is more correct but couples search to scrape having run. Recommendation: convention for v1, registry later.

2. **Table naming:** Currently all issue/PR data goes into `forge_issues` / `forge_prs` regardless of source. Should new connectors get their own tables (e.g. `gitea_issues`) or share? Sharing is simpler for search (one table to query). Recommendation: shared tables, schema declares which table to use.

3. **Materialized view creation:** Currently `create_materialized_views()` hardcodes the table DDL. Should schemas declare their table structure? That's a bigger change. Recommendation: keep shared tables for the issue/PR domain, let schema.toml declare which existing table to project into.
