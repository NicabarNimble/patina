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
| Path layout | `raw/<provider>/<source_path>/<type>/` — receives `source_path` opaque from Mother |
| File naming | ISO 8601 timestamp-based file names |
| Dedup enforcement | Reads identity fields from config, checks against existing files |
| Append semantics | New file per ingestion run, never modify existing files |
| Provenance columns | Attaches _ingested_at, _source_id, _content_hash |
| Write result reporting | Returns count and paths to Mother |

The lakehouse child owns the storage boundary. It receives records
via pipe protocol and writes them. It does not know about GitHub,
source APIs, or routing policy. Layout ownership is split: Mother
provides `source_path` (routing decision — which source goes where),
lakehouse owns everything below that (file naming, Parquet format,
data type partitioning, append semantics).

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

**events.db bypass:** Lake-bound records never touch events.db. Each
consumer scope has its own write side and audit trail:
- Project scope: events.db (content_hash dedup, transactional cursor)
- Lake scope: Parquet files + lake_sync (identity field dedup,
  append-only immutable files, provenance columns)

To route the same connector output to both project and lake, configure
two source entries in sources.toml pointing to the same connection
with different destinations. This is explicit fan-out via config.

### pipe/ingest Method (Normative)

New pipe protocol method for Mother → lakehouse child record delivery.
Follows the JSON-RPC 2.0 pattern from [[pipe-architecture]] §1.2.
This specification is normative — implementation must match.

**Direction:** Mother → lakehouse child (Mother sends, child responds).
This is the reverse of pipe/fetch (child sends facts to Mother).

**When called:** After Mother receives facts from a connector child
via pipe/fetch, validates them, and determines the destination is a
lake (from `sources.toml` destination field).

**Batching:** Mother sends records in bounded batches, not as a single
unbounded payload. The maximum batch size matches `DEFAULT_MAX_BATCH_SIZE`
(currently 10,000 records, from [[pipe-architecture]] §13 safety net).
For fetches exceeding this limit, Mother calls pipe/ingest multiple
times. The lakehouse child maintains its dedup identity index across
calls within the same ingestion run.

This is consistent with pipe-architecture §1.3 Streaming Fact Delivery:
the pipe/fact transport layer delivers facts as individual notifications
(O(1) per fact). The broker accumulates facts into a bounded batch per
fetch (O(batch), bounded by limit). pipe/ingest accepts bounded batches
matching this same bound. Neither the transport nor the broker
accumulates unbounded data.

#### Request

```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "method": "pipe/ingest",
  "params": {
    "lake_path": "/Users/foo/.patina/lakes/github-data",
    "persona": "default",
    "provider": "github",
    "source_path": "NicabarNimble/patina",
    "schema": "github",
    "schema_version": "1.0.0",
    "identity_fields": {
      "github.issue": ["number"],
      "github.pr": ["number"]
    },
    "records": [
      {
        "event_type": "github.issue",
        "data": "{\"number\":42,\"title\":\"Fix auth\",\"body\":\"...\",\"state\":\"open\"}",
        "content_hash": "blake3:abc123def456..."
      },
      {
        "event_type": "github.pr",
        "data": "{\"number\":17,\"title\":\"Add caching\",\"body\":\"...\",\"state\":\"merged\"}",
        "content_hash": "blake3:789xyz012..."
      }
    ]
  }
}
```

**Required fields:**

| Field | Type | Description |
|-------|------|-------------|
| `lake_path` | string | Absolute path to lake root directory |
| `persona` | string | Persona scope for this lake (Mother-provided) |
| `provider` | string | Provider identifier (from connection config) |
| `source_path` | string | Opaque path segment for directory layout (Mother-constructed) |
| `schema` | string | Schema package name (matches schema.toml) |
| `schema_version` | string | Schema version (for provenance) |
| `identity_fields` | object | Map of event_type → list of identity field names (for dedup) |
| `records` | array | Bounded batch of records to ingest (max `DEFAULT_MAX_BATCH_SIZE`) |

**Record fields:**

| Field | Type | Description |
|-------|------|-------------|
| `event_type` | string | Fact type (e.g., `github.issue`) |
| `data` | string | JSON payload (canonical serialization from connector) |
| `content_hash` | string | blake3 hash of canonical data (from pipe protocol) |

**Notes:**
- `identity_fields` is keyed by event_type because different fact types
  within the same schema may have different identity keys (e.g., issues
  dedup by `number`, comments might dedup by `id`).
- `data` is a JSON string, not a parsed object. The lakehouse child
  parses it for Parquet column extraction but the raw string is the
  source of truth for content hashing.
- `persona` is provided by Mother, not by the connector. Persona
  scoping is Mother's domain.
- `source_path` is an opaque string constructed by Mother from the
  source params. Lakehouse uses it for directory layout:
  `raw/<provider>/<source_path>/<data_type>/`. Mother owns path
  construction (a routing concern); lakehouse receives it opaque.
  This replaces the earlier `source_identity: HashMap<String, String>`
  which was nondeterministic (HashMap iteration order) and could not
  represent multi-value params like `channels = ["#dev", "#incidents"]`.
- `records` is a bounded batch. Mother MUST NOT send more than
  `DEFAULT_MAX_BATCH_SIZE` records per call. For larger fetches,
  Mother calls pipe/ingest multiple times.

#### Response (success)

```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "result": {
    "written": 42,
    "dedup_skipped": 3,
    "files": [
      "raw/github/NicabarNimble/patina/issues/20260308T134500Z.parquet",
      "raw/github/NicabarNimble/patina/prs/20260308T134500Z.parquet"
    ],
    "provenance": {
      "ingested_at": "2026-03-08T13:45:00Z",
      "source_connector": "github-connector",
      "schema_version": "1.0.0"
    }
  }
}
```

**Result fields:**

| Field | Type | Description |
|-------|------|-------------|
| `written` | integer | Total records written to Parquet |
| `dedup_skipped` | integer | Records skipped (identity match, same content) |
| `files` | array of strings | Relative paths (from lake root) of written Parquet files |
| `provenance` | object | Metadata attached to all written records |

**Provenance fields:**

| Field | Type | Description |
|-------|------|-------------|
| `ingested_at` | string | ISO 8601 timestamp of this ingestion run |
| `source_connector` | string | Child name that produced the records |
| `schema_version` | string | Schema version at ingestion time |

#### Response (error)

```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "error": {
    "code": -32001,
    "message": "Parquet write failed: disk full",
    "data": { "records_before_failure": 37 }
  }
}
```

Error codes follow pipe protocol conventions ([[pipe-architecture]] §1.5):
- `-32001` (Transient): disk full, permission error, temporary I/O failure
- `-32002` (Fatal): invalid schema, corrupted lake directory, unsupported format

#### Post-Response: Mother Cursor Advance

After receiving a successful result, Mother:
1. Updates `lake_sync.cursor` in graph.db for this source-lake pair
2. Updates `lake_sync.records_written` (cumulative)
3. Updates `lake_sync.last_run` to current timestamp
4. Updates `lake_sync.status` to `'ok'`

If the response is an error, Mother:
1. Does NOT advance cursor (next run re-fetches)
2. Updates `lake_sync.status` to `'error'`
3. Stores error message in `lake_sync.error`

**Failure mode:** Cursor and Parquet write are not in the same
transaction (different stores: graph.db vs filesystem). cursor-after-
confirmed-write is safe because worst case is re-fetch + dedup.

#### Implementation Note

This method should be defined as Rust types in `patina-pipe-types`
when implemented:

```rust
// crates/patina-pipe-types/src/ingest.rs
pub struct IngestParams {
    pub lake_path: PathBuf,
    pub persona: String,
    pub provider: String,
    pub source_path: String,
    pub schema: String,
    pub schema_version: String,
    pub identity_fields: HashMap<String, Vec<String>>,
    pub records: Vec<IngestRecord>,
}

pub struct IngestRecord {
    pub event_type: String,
    pub data: String,
    pub content_hash: String,
}

pub struct IngestResult {
    pub written: u64,
    pub dedup_skipped: u64,
    pub files: Vec<String>,
    pub provenance: IngestProvenance,
}

pub struct IngestProvenance {
    pub ingested_at: String,
    pub source_connector: String,
    pub schema_version: String,
}
```

Mother sends records from the connector to the lakehouse child in
bounded `pipe/ingest` calls (max `DEFAULT_MAX_BATCH_SIZE` records
per call). The lakehouse child partitions by data type, dedup-checks,
writes Parquet files, and reports what it wrote. Mother advances the
cursor after the final successful ingest call. For fetches within
the batch limit (typical for v1), this is a single call.

### Source Path Construction

Mother constructs `source_path` from source params and passes it
as an opaque string to the lakehouse child. The lakehouse child
uses it for directory layout without parsing it.

```toml
[sources.github-lake]
connection = "github"
params = { owner = "NicabarNimble", repo = "patina" }
destination = { type = "lake", lake = "github-data" }
```

Mother constructs `source_path` by joining hierarchical params:
`"NicabarNimble/patina"` (from `owner` + `repo`). The lakehouse
child receives this opaque and constructs:
`raw/<provider>/<source_path>/<data_type>/` →
`raw/github/NicabarNimble/patina/issues/`.

**Why Mother owns path construction:**
- Path construction is a **routing concern** (where does data go).
  Mother owns routing per role-boundary doctrine.
- Source params vary by connector (GitHub has owner/repo, Slack has
  workspace/channel, RSS has feed URL). Mother normalizes these
  into a deterministic path string.
- `HashMap` iteration order is nondeterministic — the current
  `sources.rs` stores params as `HashMap<String, toml::Value>`,
  so iteration order cannot produce deterministic paths.
- Multi-value params (e.g., `channels = ["#dev", "#incidents"]`)
  require decisions about path representation that belong to routing
  policy, not storage mechanics.

**Resolution:** The connector's schema.toml declares a `path_template`
that defines the deterministic path construction:

```toml
# children/github-connector/schema.toml
[lake]
path_template = "{owner}/{repo}"
```

Mother reads the template, substitutes params by name (not by
iteration order), and produces the `source_path` string. This is
deterministic because the template explicitly names the params and
their order. Connectors that don't declare a `path_template` get
a fallback: the source entry name from sources.toml (e.g.,
`"github-lake"`).

The lakehouse child treats `source_path` as opaque regardless of
how it was constructed.

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

### Sandbox Profile

The lakehouse child uses the **storage child sandbox profile** from
[[pipe-architecture]] §8.3: scoped filesystem access to the lake
root path, deny-all network.

Mother configures the sandbox at spawn time:
1. Read `child.toml` type = "lakehouse"
2. Resolve the lake root path from the destination config
3. Generate OS sandbox profile with scoped filesystem access to
   that path only (macOS: `(allow file-read* file-write* (subpath
   "<lake_path>"))`; Linux: Landlock `PathBeneath` rule)
4. Deny all outbound network — lakehouse communicates only via
   stdio (pipe protocol)

This resolves the conflict between pipe-architecture §8.3 (which
originally said "deny all filesystem") and the lakehouse's need to
create directories and write Parquet files. The sandbox is
parameterized, not weakened — connectors still get deny-all
filesystem.

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

## SDK/WIT/Code Alignment Audit (session 20260308-164629)

The existing code surface encodes assumptions from before the
role-boundary doctrine was established. This section documents
what needs to change when raw-lake-ingestion is implemented, so
protocol debt is tracked — not left as undiscovered surprises.

### host.wit — emit interface assumes events.db

**File:** `wit/deps/patina-host/host.wit` lines 136-155

**Current:** The `emit` interface says "writes to events.db with
provenance=external." The `emit-fact` function takes schema,
fact-type, and data — returns a sequence number (from events.db).

**Problem:** This hardcodes the destination as events.db. Under
raw-lake-ingestion, connector output may route to a lake (via
lakehouse child) instead of events.db. The WASM host's emit
implementation is the routing point.

**Required change:** The emit interface itself can stay unchanged
(it's the child's API — children emit facts, Mother routes them).
But the HOST IMPLEMENTATION must gain destination awareness: when
the source has `destination.type = "lake"`, the host routes to
the lakehouse child instead of writing to events.db. The WIT
contract doesn't need to change — routing is Mother's concern.

**Tracking:** This change belongs to [[raw-lake-ingestion]] step 6
("Wire broker routing"). The WIT file itself doesn't change; the
host implementation in `src/plugin/internal/host_support.rs` does.

### mother-child.wit — broad daemon world, not role-specific

**File:** `plugins/sdk/wit/mother-child/mother-child.wit` (also
`wit/mother-child/mother-child.wit` — identical copies)

**Current:** Defines a "mother-child" world with init, on-load,
on-unload, health, handle (string dispatch), and tick (toy requests).
This is a WASM-era design for long-lived daemon children.

**Problem:** Under pipe-architecture, native children speak pipe
protocol (JSON-RPC over stdio). WASM children use this WIT world.
The two interfaces have different shapes:
- Pipe protocol: initialize, fetch, health, shutdown (role-specific)
- mother-child.wit: init, load, unload, handle, tick (generic daemon)

The handle(action, payload) function can dispatch pipe protocol
methods — and the existing forge plugin does exactly this. So
the WIT world is NOT broken, just unaligned with the naming.

**Required change (future, not this spec):** Two options:
1. mother-child.wit evolves to mirror pipe protocol methods
   (initialize, fetch, ingest, health, shutdown exports)
2. mother-child.wit is deprecated for new children; native pipe
   protocol is the primary path; WASM children use a new
   `pipe-child.wit` world that matches pipe protocol

**Tracking:** This is [[pipe-architecture]] scope, not
raw-lake-ingestion. No change needed for lake v1 since the
lakehouse child is native (speaks pipe protocol directly).
Document as discovery note in pipe-architecture.

### sources.rs — no destination or persona model

**File:** `src/broker/sources.rs`

**Current:** `SourceEntry` has: name, connection, params, types,
schedule. No `destination` field. No `persona` field.

**Problem:** raw-lake-ingestion requires `destination` to route
connector output to a lake vs project events.db. The current
`RawSourceEntry` struct doesn't parse `destination`.

**Required change:** Add `destination` to both `RawSourceEntry`
(deserialization) and `SourceEntry` (public API):

```rust
#[derive(Debug, Clone)]
pub struct SourceEntry {
    pub name: String,
    pub connection: String,
    pub params: HashMap<String, toml::Value>,
    pub types: Vec<String>,
    pub schedule: String,
    pub destination: Option<SourceDestination>,  // NEW
}

#[derive(Debug, Clone, Deserialize)]
pub struct SourceDestination {
    #[serde(rename = "type")]
    pub dest_type: String,  // "project" or "lake"
    pub lake: Option<String>,  // lake name, if type=lake
}
```

**Tracking:** This is [[raw-lake-ingestion]] step 1 ("Add
destination field to sources.toml format").

### broker/mod.rs — terminates at events.db, no lake branching

**File:** `src/broker/mod.rs`

**Current:** `run_source()` follows a linear flow: spawn child →
fetch → validate → write to events.db. Step 4 opens events.db
unconditionally. Step 9 writes facts + cursor to events.db.

**Problem:** When `destination.type = "lake"`, the flow must branch:
step 4 resolves the lake from registry and spawns a lakehouse child
instead of opening events.db. Step 9 sends records to the lakehouse
child via pipe/ingest instead of writing to events.db.

**Required change:** Add destination branching:

```
Step 4: IF destination is project (or absent) → open events.db
        IF destination is lake → resolve lake, spawn lakehouse child

Step 9: IF project → write_facts_with_cursor (existing)
        IF lake → pipe/ingest to lakehouse → update lake_sync cursor
```

The connector is unaware of this branching. It emits facts via pipe
protocol regardless. Mother decides routing.

**Tracking:** This is [[raw-lake-ingestion]] step 6 ("Wire broker
routing") and step 7 ("Lakehouse reports results, Mother updates
lake_sync cursor").

### github-connector — healthy, no changes needed

**File:** `children/github-connector/src/main.rs`
**File:** `children/github-connector/child.toml`

**Status:** Healthy. The github-connector is a clean source-boundary
adapter. It implements `Child` trait with `capabilities()` and
`fetch()`. It emits facts via pipe protocol. It knows nothing about
destinations, lakes, or materialization. This is exactly right.

No changes needed for raw-lake-ingestion or connector-owns-tables.

## Open Questions

1. **pipe/ingest batch size.** ~~Should Mother send all records in one
   pipe/ingest call, or stream them?~~ **Resolved:** Mother sends
   bounded batches (max `DEFAULT_MAX_BATCH_SIZE = 10,000` records per
   call), matching the existing broker safety net from lifecycle.rs.
   For v1, typical GitHub fetches are hundreds of records — single
   call. For larger datasets, Mother calls pipe/ingest multiple times.
   True per-record streaming (pipe/record notifications) is future
   optimization. See pipe/ingest §Batching above.

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

6. **Schema passthrough to lakehouse.** Resolved by pipe/ingest spec:
   Mother passes `identity_fields` per event_type and `schema_version`
   in the ingest params. Lakehouse does not need the full schema.toml.
