# Design: Raw Lake Ingestion — V1 Append-Only Parquet Capture

## Why This Work Exists

The pipe architecture data layer diagram says Sources → Lakes →
Projects → Beliefs. The code says Sources → Project eventlog. There
is no lake. raw-lake-ingestion builds the Sources → Lakes arrow.

The infrastructure is surprisingly complete: github-connector emits
records via pipe protocol, Mother's broker routes facts, cursor
tracking works, sandbox works. All of this targets one destination:
project events.db. Adding a second destination type (lake / Parquet)
is the minimum change that makes the multi-destination architecture
real.

**Origin:** [[session-20260308-134326]] — V1 Working Lake spec
restructuring. Identified that no existing spec owns the raw lake
path end-to-end. connector-owns-tables conflated v1 and full
architecture. The outside audit agent confirmed: "raw append-only
Parquet capture under Mother-managed registry and cursor control."

## Responsibility Split

This is the normative boundary between connector and Mother for lake
ingestion.

### Connector Owns

| Concern | What it means |
|---------|--------------|
| Domain record shape | Fields, types, naming (github.issue has number, title, body, ...) |
| Schema/version declaration | schema.toml declares fact types and versions |
| Identity fields for dedup | Declares which fields constitute record identity (e.g. `number`) |
| API pagination, rate limiting | Source-specific fetch behavior |
| Cursor semantics | What the cursor value means (timestamp, page token, etag) |

### Mother Owns

| Concern | What it means |
|---------|--------------|
| Destination selection | sources.toml `destination` field determines project vs lake |
| Lake registration | graph.db lake_registry: name, location, metadata |
| Sync cursor storage | graph.db lake_sync: cursor per source-lake pair |
| Routing decision | Broker reads destination, routes to events.db writer or lake writer |
| Parquet serialization | Converts pipe protocol facts to Parquet record batches |
| File emission | Writes Parquet files to the lake's raw storage path |
| Path layout | `<lake>/raw/<provider>/<owner>/<repo>/<type>/` convention |
| File naming | ISO 8601 timestamp-based file names |
| Dedup enforcement | Reads identity fields from schema, checks against existing files |
| Append semantics | New file per ingestion run, never modify existing files |
| Provenance metadata | Attaches source connector, ingestion timestamp, content hash |

### Shared Responsibility

| Concern | Connector role | Mother role |
|---------|---------------|------------|
| Dedup/idempotency | Declares identity fields in schema | Enforces idempotent append at write time |
| Schema evolution | Bumps version, adds fields | Writes whatever the connector sends (raw zone is schema-tolerant) |
| Error recovery | Retries API calls, emits partial results | Transactional cursor: only advances after successful write |

## Broker Routing Extension

The existing broker flow (`src/broker/mod.rs::run_source`) is:

```
1. Load connection config
2. Decrypt credential
3. Spawn child
4. Open destination events.db
5. Get stored cursor
6. Fetch facts from child
7. Validate facts against schema
8. Write facts + cursor transactionally to events.db
```

The lake extension adds a branch at step 4:

```
4a. If destination.type == "project" (or absent):
      Open destination events.db (existing path)
4b. If destination.type == "lake":
      Resolve lake from registry
      Open lake writer for the resolved path

...steps 5-7 unchanged...

8a. Project path: write facts + cursor to events.db (existing)
8b. Lake path: write Parquet file + update lake_sync cursor
```

The connector is unaware of this branching. It emits facts via pipe
protocol regardless of destination. Mother decides where they go.

### Source Identity Extraction

For the partitioned path layout, Mother needs `provider`, `owner`,
`repo` from the source declaration. These come from sources.toml:

```toml
[sources.github-lake]
connection = "github"
params = { owner = "NicabarNimble", repo = "patina" }
destination = { type = "lake", lake = "github-data" }
```

Mother reads `connection` to determine provider (from connection
config), and `params.owner` + `params.repo` for the source path.
This is not GitHub-specific — the convention is:
`<provider>/<param1>/<param2>/.../<data_type>/`

For a future Google Workspace connector:
```
google-workspace/
  my-org/
    drive/
      documents/
        20260310T...parquet
```

The path template is provider-defined, not hardcoded per connector.

## Parquet Format

### Schema

Each Parquet file contains all records of one data type from one
ingestion run. The schema is derived from the connector's emitted
JSON records plus Mother-added metadata columns:

**Connector columns** (from the fact's `data` JSON):
- All fields from the domain record, flattened to Parquet columns
- Types inferred from schema.toml declarations or JSON inspection

**Mother-added columns** (provenance):
- `_ingested_at` — ISO 8601 timestamp of this ingestion run
- `_source_id` — e.g. `child:github-connector`
- `_content_hash` — blake3 hash of the canonical JSON record
- `_fact_seq` — sequence number from the pipe protocol emission

### Parquet Writer

Use the `arrow` and `parquet` crates (already in the Rust ecosystem,
no Python dependency):

```rust
// Pseudocode for lake writer
fn write_parquet(
    records: &[ValidatedFact],
    partition_path: &Path,
    timestamp: &str,
) -> Result<WriteResult> {
    let file_path = partition_path.join(format!("{}.parquet", timestamp));
    let schema = infer_arrow_schema(records)?;
    let batch = facts_to_record_batch(records, &schema)?;
    let file = File::create(&file_path)?;
    let mut writer = ArrowWriter::new(file, schema, None)?;
    writer.write(&batch)?;
    writer.close()?;
    Ok(WriteResult { path: file_path, count: records.len() })
}
```

### JSON-to-Parquet Column Mapping

The connector emits facts as JSON (`data` field is a JSON string).
Mother needs to convert JSON records to Arrow/Parquet columns:

**V1 approach (simple):** Store each record as a single JSON string
column plus metadata columns. This is the simplest path — no schema
inference, no column mapping. DuckDB can query JSON columns with
`json_extract()`.

```
Columns: _data (JSON string), _ingested_at, _source_id, _content_hash
```

**V1+ approach (columnar):** Parse JSON records into typed Parquet
columns using the schema.toml declaration. Better query performance,
native Parquet filtering. More implementation work.

**Recommendation:** Start with the JSON-string approach for v1.
It gets Parquet files on disk fast. Column extraction is a follow-on
optimization that doesn't change the file layout or capture contract.

## Dedup Strategy

### V1: In-Memory Identity Index

Before writing a new Parquet file, Mother reads existing Parquet
files in the same partition to build an identity index:

```rust
// For github issues: identity_fields = ["number"]
// Read all existing files, extract identity field values
let existing: HashSet<String> = read_identity_values(
    partition_path,
    &["number"],
)?;

// Filter incoming records
let new_records: Vec<_> = records
    .into_iter()
    .filter(|r| {
        let key = extract_identity_key(r, &["number"]);
        !existing.contains(&key)
        // OR: content changed (key exists but hash differs)
    })
    .collect();
```

This works for small-to-medium lakes. For large lakes (millions of
records), a persistent dedup index (SQLite or bloom filter) is
needed — future work.

### Changed Records

Raw zone captures history. If a GitHub issue's title changes:
- Old record stays in the earlier Parquet file
- New record (same `number`, different content) is appended in the
  new file
- Both are visible to queries (`SELECT * ORDER BY _ingested_at`)
- Curated layer (future) handles dedup/merge/latest-version semantics

This is intentional: raw zone is an immutable log of observations.

## Cursor Management

Lake cursor lives in `lake_sync` table (graph.db), not in project
events.db `broker_cursors`. This is because:

- Lake is Mother-scoped, not project-scoped
- Multiple projects can declare the same lake source
- Cursor should advance once per lake ingestion, not per project

**Transactional guarantee:** cursor advances only after Parquet
file is successfully written. If the write fails, cursor stays —
next run re-fetches and dedup handles overlap.

```rust
fn write_and_advance_cursor(
    graph_conn: &Connection,
    lake_name: &str,
    source_name: &str,
    records: &[ValidatedFact],
    new_cursor: Option<&str>,
    partition_path: &Path,
) -> Result<WriteResult> {
    // 1. Write Parquet file
    let write_result = write_parquet(records, partition_path)?;

    // 2. Update cursor in same logical operation
    // (graph.db and Parquet file are not in the same transaction,
    //  but cursor-after-write is safe: worst case is re-fetch + dedup)
    update_lake_cursor(graph_conn, lake_name, source_name, new_cursor)?;

    Ok(write_result)
}
```

Note: unlike the project path (events.db cursor is transactional
with fact writes in SQLite), the lake path cannot be fully atomic
(Parquet file + SQLite cursor are different stores). The design is
cursor-after-write: safe because re-fetch + dedup is idempotent.

## Lake Creation and Registration

```bash
# Register a new lake
patina lake create github-data

# This creates:
# ~/.patina/lakes/github-data/raw/   (directory)
# graph.db: INSERT INTO lake_registry (name, location, created_at)
```

Lake creation is explicit. Sources reference lakes by name.
If a source references a lake that doesn't exist, Mother fails
with a clear error.

## Key Files

**Extend:**
- `src/broker/mod.rs` — add destination branching in `run_source()`
- `src/broker/sources.rs` — parse `destination` field
- `src/mother/mod.rs` or `src/mother/graph.rs` — lake_registry and
  lake_sync table creation

**New:**
- `src/lake/mod.rs` — lake module: create, resolve, status
- `src/lake/writer.rs` — Parquet writer (facts → record batch → file)
- `src/lake/dedup.rs` — identity-based dedup from existing Parquet
- `src/lake/layout.rs` — path conventions, partition directory management
- `src/commands/lake/mod.rs` — `patina lake create`, `patina lake query`

**Reference (unchanged):**
- `children/github-connector/` — connector code, no changes
- `src/broker/routing.rs` — fact validation, no changes
- `src/broker/lifecycle.rs` — child lifecycle, no changes

## Dependencies

- `arrow` crate — Arrow array/schema types
- `parquet` crate — Parquet file writer
- Both are pure Rust, no Python/C dependency

`cargo tree` should be checked for existing Arrow/Parquet deps
before adding. If already present transitively, use the same version.

## Open Questions

1. **JSON-string vs columnar Parquet.** V1 recommendation is
   JSON-string column for speed. But DuckDB auto-detects JSON and
   queries it well. Is there a reason to go columnar from day one?
   Measure: how does DuckDB perform on 10K JSON-string rows vs
   10K columnar rows for a simple filter query?

2. **Dedup memory budget.** For a lake with 100K issues across 50
   Parquet files, the in-memory identity index is small (100K strings).
   At what scale does this become a problem? Should we add a SQLite
   dedup index from the start?

3. **Lake location configuration.** Default is `~/.patina/lakes/`.
   Should this be configurable via Mother config? Or is it fixed
   for v1?

4. **Parquet compression.** Snappy (fast, default) vs Zstd (smaller,
   slower)? Snappy is the Parquet ecosystem default. Probably fine
   for v1.

5. **Multi-type batching.** If a connector emits both issues and PRs
   in one fetch, Mother needs to partition them into separate Parquet
   files (one per data type). The broker already receives facts one
   at a time — Mother accumulates per type, then writes. Is there
   a simpler approach?
