# Design: Raw Lake Ingestion — V1 Append-Only Parquet Capture

## Why This Work Exists

The pipe architecture data layer diagram says Sources → Lakes →
Projects → Beliefs. The code says Sources → Project eventlog. There
is no lake. raw-lake-ingestion builds the Sources → Lakes arrow.

The infrastructure is surprisingly complete: github-connector emits
records via pipe protocol, Mother's broker routes facts, cursor
tracking works, sandbox works. All of this targets one destination:
project events.db. Adding a second destination type (lake via
lakehouse child) is the minimum change that makes the multi-destination
architecture real.

**Origin:** [[session-20260308-134326]] — V1 Working Lake spec
restructuring. Identified that no existing spec owns the raw lake
path end-to-end. connector-owns-tables conflated v1 and full
architecture. Outside audit agent confirmed: "raw append-only
Parquet capture under Mother-managed registry and cursor control."

**Architectural correction:** Initial design had Mother writing
Parquet inline. Outside alignment agent identified this as role
smearing — Mother should govern, not execute data-plane concerns.
Corrected: lakehouse child is a real child from day one.

## Role Boundaries (normative)

### Connector Child — Source-Boundary Adapter

| Concern | What it means |
|---------|--------------|
| Domain record shape | Fields, types, naming (github.issue has number, title, body, ...) |
| Schema/version declaration | schema.toml declares fact types and versions |
| Identity fields for dedup | Declares which fields constitute record identity (e.g. `number`) |
| API pagination, rate limiting | Source-specific fetch behavior |
| Cursor semantics | What the cursor value means (timestamp, page token, etag) |

A connector owns **one external system boundary**. It may ingest
from that system and may apply changes back, but it never becomes
storage, transform, or coordination.

### Mother — Node-Local Control Plane

| Concern | What it means |
|---------|--------------|
| Destination selection | sources.toml `destination` field determines project vs lake |
| Lake registration | graph.db lake_registry: name, location, persona, metadata |
| Sync cursor truth | graph.db lake_sync: cursor per source-lake pair |
| Routing decision | Broker reads destination, routes to events.db or lakehouse child |
| Lifecycle management | Spawn, monitor, shutdown connector and lakehouse children |
| Persona scoping | Lakes are persona-scoped; Mother enforces boundaries |
| Policy | When to run, what to route where |

Mother governs persona-scoped nodes. She never writes storage,
never decides file format, never owns dedup logic.

### Lakehouse Child — Storage-Boundary Worker

| Concern | What it means |
|---------|--------------|
| Parquet serialization | Converts domain records to Parquet record batches |
| File emission | Writes Parquet files to the lake path |
| Path layout | `raw/<provider>/<owner>/<repo>/<type>/` convention |
| File naming | ISO 8601 timestamp-based file names |
| Dedup enforcement | Reads identity fields from config, checks against existing files |
| Append semantics | New file per ingestion run, never modify existing files |
| Provenance columns | Attaches _ingested_at, _source_id, _content_hash |
| Write result reporting | Returns count and paths to Mother |

The lakehouse child owns the storage boundary. It receives records
via pipe protocol and writes them. It does not know about GitHub,
source APIs, or routing policy.

### Shared Responsibility

| Concern | Connector role | Mother role | Lakehouse role |
|---------|---------------|------------|---------------|
| Dedup | Declares identity fields in schema | Passes schema config to lakehouse | Enforces idempotent append at write time |
| Schema evolution | Bumps version, adds fields | Validates against manifest | Writes whatever is received (raw zone is schema-tolerant) |
| Error recovery | Retries API calls, emits partial results | Advances cursor only after confirmed write | Reports write success/failure |

## Two-Child Pipeline

### Broker Routing Extension

The existing broker flow (`src/broker/mod.rs::run_source`) is:

```
1. Load connection config
2. Decrypt credential
3. Spawn connector child
4. Open destination events.db
5. Get stored cursor
6. Fetch facts from child
7. Validate facts against schema
8. Write facts + cursor transactionally to events.db
```

The lake extension replaces steps 4 and 8:

```
1. Load connection config
2. Decrypt credential
3. Spawn connector child

4. IF destination.type == "project" (or absent):
     Open destination events.db (existing path)
   IF destination.type == "lake":
     Resolve lake from registry
     Spawn lakehouse child with lake config

5. Get stored cursor (project: from events.db; lake: from lake_sync)
6. Fetch facts from connector child
7. Validate facts against schema

8. IF project path:
     Write facts + cursor to events.db (existing)
   IF lake path:
     Send facts to lakehouse child via pipe/ingest
     Lakehouse writes Parquet, reports results
     Mother updates lake_sync cursor after confirmed write
```

The connector is unaware of this branching. It emits facts via pipe
protocol regardless of destination. Mother decides where they go.
The lakehouse child is unaware of where facts came from.

### pipe/ingest Method

New pipe protocol method for Mother → child record delivery:

```json
// Mother sends to lakehouse child:
{
  "jsonrpc": "2.0",
  "method": "pipe/ingest",
  "params": {
    "lake_path": "/Users/foo/.patina/lakes/github-data",
    "provider": "github",
    "source_identity": { "owner": "NicabarNimble", "repo": "patina" },
    "schema": "github",
    "identity_fields": ["number"],
    "records": [
      { "event_type": "github.issue", "data": "{...}", "content_hash": "..." },
      { "event_type": "github.pr", "data": "{...}", "content_hash": "..." }
    ]
  }
}

// Lakehouse child responds:
{
  "jsonrpc": "2.0",
  "result": {
    "written": 42,
    "dedup_skipped": 3,
    "files": [
      "raw/github/NicabarNimble/patina/issues/20260308T134500Z.parquet",
      "raw/github/NicabarNimble/patina/prs/20260308T134500Z.parquet"
    ]
  }
}
```

Mother batches records from the connector and sends them to the
lakehouse child in one call. The lakehouse child partitions by
data type, dedup-checks, writes Parquet files, and reports what it
wrote. Mother then updates the cursor.

### Source Identity Extraction

For the partitioned path layout, Mother extracts source identity
from the source declaration:

```toml
[sources.github-lake]
connection = "github"
params = { owner = "NicabarNimble", repo = "patina" }
destination = { type = "lake", lake = "github-data" }
```

Mother passes `provider` (from connection config) and
`source_identity` (from params) to the lakehouse child. The
lakehouse child uses these for the directory layout. This is not
GitHub-specific — any hierarchical source identity works.

## Lakehouse Child Implementation

### Binary Structure

```
children/lakehouse/
  Cargo.toml          # depends on patina-pipe, arrow, parquet
  child.toml          # type=lakehouse, runtime=native, lifecycle=poll
  src/
    main.rs           # Child trait impl — pipe/ingest handler
    writer.rs         # Parquet serialization (JSON records → Arrow → Parquet)
    dedup.rs          # Identity-based dedup against existing Parquet files
    layout.rs         # Path conventions, directory creation
```

### child.toml

```toml
[child]
name = "lakehouse"
version = "0.1.0"
type = "lakehouse"
runtime = "native"
lifecycle = "poll"
description = "Raw lake storage — append-only Parquet capture"

[capabilities]
methods = ["ingest"]
```

### Parquet Format

Each Parquet file contains all records of one data type from one
ingestion run. Two approaches for JSON-to-Parquet conversion:

**V1 approach (simple):** Store each record as a single JSON string
column plus metadata columns. DuckDB handles JSON columns natively.

```
Columns: _data (JSON string), _event_type, _ingested_at, _source_id, _content_hash
```

**V1+ approach (columnar):** Parse JSON records into typed Parquet
columns using the schema declaration. Better query performance.
More implementation work.

**Recommendation:** Start with JSON-string approach. Gets Parquet
files on disk fast. Column extraction is a follow-on optimization.

### Dedup Strategy

Before writing, the lakehouse child reads existing Parquet files
in the target partition to build an identity index:

```rust
// For github issues: identity_fields = ["number"]
let existing: HashSet<String> = read_identity_values(
    partition_path,
    &identity_fields,
)?;

let new_records: Vec<_> = records
    .into_iter()
    .filter(|r| {
        let key = extract_identity_key(r, &identity_fields);
        !existing.contains(&key)
        // OR: content changed (key exists but hash differs) → append
    })
    .collect();
```

**Changed records:** Raw zone captures history. If a GitHub issue's
title changes, the new version is appended in a new file. Both are
visible to queries. Curated layer (future) handles latest-version
semantics.

## Cursor Management

Cursor truth lives in Mother's `lake_sync` table (graph.db), not in
the lakehouse child. This is because:

- Cursor truth is a control-plane concern (Mother's domain)
- Multiple sources can feed the same lake
- Cursor must advance only after confirmed write

**Flow:**
1. Mother reads cursor from lake_sync
2. Mother passes cursor to connector via pipe/fetch
3. Connector fetches new records since cursor
4. Mother sends records to lakehouse child
5. Lakehouse writes and confirms
6. Mother advances cursor in lake_sync

**Failure mode:** If lakehouse write fails, cursor stays. Next run
re-fetches and dedup handles overlap. Safe because append + dedup
is idempotent.

Note: cursor and Parquet write are not in the same transaction
(different stores). cursor-after-confirmed-write is safe because
worst case is re-fetch + dedup.

## Dependencies

Lakehouse child needs:
- `patina-pipe` crate — Child trait, pipe protocol
- `patina-pipe-types` crate — shared types
- `arrow` crate — Arrow array/schema types
- `parquet` crate — Parquet file writer

Both arrow and parquet are pure Rust. No Python/C dependency.

`cargo tree` should be checked before adding — if already present
transitively, use the same version.

## Key Files

**Extend:**
- `src/broker/mod.rs` — add lake destination branching
- `src/broker/sources.rs` — parse `destination` field
- `src/mother/` — lake_registry and lake_sync tables in graph.db
- `crates/patina-pipe-types/` — pipe/ingest method types

**New:**
- `children/lakehouse/` — entire lakehouse child binary
- `src/commands/lake/mod.rs` — `patina lake create`, `patina lake query`

**Reference (unchanged):**
- `children/github-connector/` — connector code, no changes
- `src/broker/routing.rs` — fact validation, no changes
- `src/broker/lifecycle.rs` — child lifecycle, no changes

## Open Questions

1. **pipe/ingest batch size.** Should Mother send all records in one
   pipe/ingest call, or stream them? For v1, one call is fine (typical
   GitHub fetch is hundreds of records, not millions). Streaming is
   future optimization.

2. **Lakehouse child lifecycle.** Spawn per ingestion run (poll mode)
   or keep alive? Poll mode is simpler and matches the connector
   pattern. Spawn, ingest, shutdown.

3. **Dedup memory budget.** For a lake with 100K issues across 50
   Parquet files, the in-memory identity index is small (100K strings).
   At what scale does this need a persistent index?

4. **Parquet compression.** Snappy (fast, default) vs Zstd (smaller,
   slower)? Snappy is the ecosystem default. Probably fine for v1.

5. **Lake location configuration.** Default is `~/.patina/lakes/`.
   Configurable per lake in future. Fixed for v1.

6. **Schema passthrough to lakehouse.** Mother needs to pass identity
   fields and schema info to the lakehouse child. This goes in the
   pipe/ingest params. Does the lakehouse child also need the full
   schema.toml? Or just identity_fields?
