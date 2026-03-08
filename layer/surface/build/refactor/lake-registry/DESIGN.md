# Design: Lake Registry — Mother-Managed Lake Metadata

## Approach

Lake registry is the simplest piece of the lake architecture: two
SQLite tables in graph.db plus a directory on disk. The tables are
created by Mother on startup. `patina lake create` inserts a row
and creates the directory. `patina mother status` reads the tables.

The actual Parquet writing, dedup, and cursor management are
[[raw-lake-ingestion]]'s scope. This spec defines the metadata
schema that raw-lake-ingestion populates.

## Key Files

**Extend:**
- `src/mother/mod.rs` or `src/mother/graph.rs` — add lake_registry
  and lake_sync table creation to graph.db initialization

**New:**
- `src/commands/lake/mod.rs` — `patina lake create`, `patina lake list`
- `src/lake/mod.rs` — lake resolution (name → path), registration

**Reference:**
- `src/commands/mother.rs` — extend status output

## Open Questions

1. **Table location: graph.db or separate lake.db?** graph.db is
   the existing Mother database for federation metadata. Adding
   2 small tables is consistent. A separate lake.db would isolate
   concerns but adds file management. Recommendation: graph.db
   for v1, reconsider if lake metadata grows significantly.

2. **Lake name uniqueness scope.** Lake names are unique within a
   Mother instance. No cross-Mother lake naming is needed for v1.
