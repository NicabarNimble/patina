---
type: feat
id: raw-lake-ingestion
status: draft
created: 2026-03-08
sessions:
  origin: 20260308-134326
related:
- connector-owns-tables
- lake-registry
- pipe-architecture
- github-connector
- mother-maturation
- mother-broker
beliefs:
- connectors-own-tables-schemas-are-contracts
- patina-is-domain-agnostic-knowledge-system
- mother-holds-connections-pipes-transform
- mother-owns-destination-format
- raw-lake-is-capture-contract-first
exit_criteria:
- id: lake-registered
  text: "Mother registers a lake by name and location in graph.db lake_registry table; `patina mother status` shows lake name, location, and sync state"
  checked: false
- id: source-declares-lake-destination
  text: "sources.toml supports a destination field that routes connector output to a named lake instead of project events.db"
  checked: false
- id: mother-writes-parquet
  text: "Mother lake writer receives domain records from connector via pipe protocol and writes append-only Parquet files to the lake's raw storage path"
  checked: false
- id: parquet-layout-partitioned
  text: "Raw Parquet files are written in a provider-partitioned layout: `<lake>/raw/<provider>/<owner>/<repo>/<data_type>/` with time-based file naming"
  checked: false
- id: cursor-sync-tracked
  text: "Mother tracks sync cursor per source-lake pair; re-running ingestion fetches only new/updated records; cursor update is transactional with Parquet write"
  checked: false
- id: dedup-enforced
  text: "Mother enforces idempotent append using connector-provided identity fields (e.g. number + updated_at); duplicate records are not written to Parquet"
  checked: false
- id: provenance-on-lake-records
  text: "Written Parquet records carry provenance metadata: source connector, ingestion timestamp, content hash"
  checked: false
- id: v1-litmus-github
  text: "End-to-end: configure GitHub connection, declare repo as lake source, run `patina mother run github --lake`, raw issue/PR data lands as Parquet, Mother shows metadata, DuckDB can query the files"
  checked: false
---
# feat: Raw Lake Ingestion — V1 Append-Only Parquet Capture

> Connector emits domain records. Mother routes to lake destination.
> Lake writer materializes append-only Parquet under Mother-managed
> registry and cursor control. Raw lake has no catalog, no shared
> tables, no curated semantics.

## Problem

Patina has working connector infrastructure: the github-connector
emits domain records via pipe protocol, Mother's broker routes facts
to project events.db, cursor tracking works. But all connector output
goes to one destination type: project eventlog (SQLite).

The 3-zone lake model (raw → curated → serving) has no implementation.
The raw zone — append-only capture of connector output as Parquet
files — is the foundation everything else builds on. Without it:

- Connector data is trapped in project-scoped SQLite
- No shared data layer across projects
- No path to curated tables (DuckLake, future)
- No multi-source lake (multiple connectors feeding one lake)
- The architecture says "Sources → Lakes → Projects" but the code
  says "Sources → Project eventlog"

The v1 proving slice makes the raw zone real with one connector,
one source, one lake.

## Solution

### Capture Contract (primary concern)

The capture contract defines what a connector writes, where it goes,
and what guarantees hold.

**Connector responsibility:**
- Emit stable domain records with typed fields via pipe protocol
- Declare schema and version in child manifest
- Provide identity fields for dedup (e.g. `number`, `updated_at`)
- Handle API pagination, rate limiting, cursor semantics

**Mother responsibility:**
- Route records to declared destination (project events.db OR lake)
- Register lake: name, location, provider, sync state
- Write append-only Parquet files in partitioned layout
- Enforce idempotent append using connector-provided identity fields
- Track sync cursor per source-lake pair (transactional with write)
- Attach provenance metadata (source, timestamp, content hash)

**The connector does not know about Parquet.** It emits domain
records. Mother decides the destination format. This keeps connectors
reusable across destination types and makes future format changes
(e.g. adding Iceberg metadata) transparent to connectors.

### Lake Destination in sources.toml

```toml
# .patina/sources.toml
[sources.github-lake]
connection = "github"
params = { owner = "NicabarNimble", repo = "patina" }
types = ["issues", "prs"]
destination = { type = "lake", lake = "github-data" }
schedule = "on-demand"
```

When `destination.type = "lake"`, Mother routes connector output to
the lake writer instead of project events.db. The lake is identified
by name, resolved from Mother's lake registry.

When `destination.type` is absent or `"project"`, the existing broker
path writes to events.db (no change to current behavior).

### Raw Lake Storage

```
~/.patina/lakes/github-data/
  raw/
    github/                          # provider
      NicabarNimble/                 # owner
        patina/                      # repo
          issues/                    # data_type
            20260308T134500Z.parquet
            20260309T091200Z.parquet
          prs/
            20260308T134500Z.parquet
```

**Properties:**
- Append-only: new files are added, never modified or deleted
- Time-based file naming: ISO 8601 ingestion timestamp
- One file per ingestion run per data type
- Parquet files contain all records from that ingestion run
  (after dedup against previous files)
- Provider-partitioned layout supports multiple connectors feeding
  the same lake without collision
- Layout is not GitHub-specific: `<provider>/<owner>/<repo>/` is a
  convention that works for any hierarchical source identity

### Lake Registration in Mother

Mother tracks lakes in `graph.db`:

```sql
CREATE TABLE IF NOT EXISTS lake_registry (
    name        TEXT PRIMARY KEY,
    location    TEXT NOT NULL,       -- filesystem path
    created_at  TEXT NOT NULL,
    metadata    TEXT                 -- JSON: provider, description
);

CREATE TABLE IF NOT EXISTS lake_sync (
    lake_name   TEXT NOT NULL,
    source_name TEXT NOT NULL,       -- from sources.toml
    cursor      TEXT,                -- opaque, connector-owned semantics
    last_run    TEXT,                -- ISO 8601
    records_written INTEGER DEFAULT 0,
    status      TEXT DEFAULT 'never_run',
    PRIMARY KEY (lake_name, source_name)
);
```

### Dedup and Idempotency

Mother reads previous Parquet files for the same data type partition
to build a dedup index (identity fields → content hash). New records
whose identity matches an existing record with the same content are
skipped. Changed records (same identity, different content) are
appended — the raw zone captures history, not just current state.

For v1, the dedup index is built in memory from recent Parquet files.
For large lakes, this will need a persistent dedup index (future).

**Identity fields** are declared in the connector's schema:

```toml
# children/github-connector/schema.toml
[[facts]]
name = "issue"
event_type = "github.issue"
identity_fields = ["number"]
```

Mother reads `identity_fields` from the schema to know which fields
constitute record identity. The connector doesn't participate in
dedup — it emits all records, Mother filters.

### Query Path (verification, not primary)

Raw Parquet files can be queried directly with DuckDB:

```sql
SELECT * FROM read_parquet('~/.patina/lakes/github-data/raw/github/NicabarNimble/patina/issues/*.parquet');
```

A minimal `patina lake query` command may wrap this for convenience,
but the query path is a verification tool, not the spec's primary
concern. The value of raw lake is the capture contract.

## Steps

1. Add `destination` field to sources.toml format; Mother parser
   recognizes `type = "lake"` and resolves lake name from registry
2. Add `lake_registry` and `lake_sync` tables to graph.db schema
3. Add `patina lake create <name>` — register a lake with Mother,
   create directory structure
4. Build lake writer module in Mother: receive facts from broker,
   serialize to Parquet, write to partitioned path
5. Add `identity_fields` to schema.toml format; Mother reads them
   for dedup
6. Implement dedup: read recent Parquet for partition, build identity
   index, skip duplicates, append new/changed records
7. Wire broker routing: when source has lake destination, route to
   lake writer instead of events.db writer
8. Track sync cursor in `lake_sync` table (transactional with
   Parquet write)
9. Update `patina mother status` to show lake sync state
10. End-to-end verification: github connection → lake source → run →
    Parquet output → DuckDB query

## Key Files

**Extend:**
- `src/broker/mod.rs` — routing decision (project vs lake)
- `src/broker/sources.rs` — destination field in sources.toml
- `src/mother/` — graph.db schema (lake_registry, lake_sync tables)

**New:**
- `src/lake/mod.rs` — lake writer module (Parquet serialization)
- `src/lake/dedup.rs` — identity-based dedup against existing files
- `src/lake/layout.rs` — path conventions, directory creation
- `src/commands/lake/` — `patina lake create`, `patina lake query`

**Reference (no changes):**
- `children/github-connector/` — emits records, unchanged
- `crates/patina-pipe-types/` — fact types, unchanged

## Non-Goals

- Curated layer (DuckLake, Iceberg, shared tables) — future spec
- Serving projections — future spec (current patina.db continues)
- Block system — future spec
- Transform chaining — future spec
- Multiple lakes per source — v1 is one source → one lake
- Parquet compaction or maintenance — future
- Cross-Mother lake replication — future
- Google Workspace or other connectors — future connector specs
- OAuth device flow — manual PAT sufficient for v1
- Real-time / streaming ingestion — poll mode only
- Project-scope materialization changes — stays in connector-owns-tables

## Relationship to Other Specs

**[[pipe-architecture]]** (active, container): Provides vocabulary.
Data Layers: Sources → Lakes → Projects → Beliefs. raw-lake-ingestion
implements the Sources → Lakes arrow for the first time.

**[[connector-owns-tables]]** (draft): Owns project-scope
materialization (events → SQLite read models). raw-lake-ingestion
owns lake-scope capture (records → Parquet). Together they prove
the destination-aware model works across scopes.

**[[lake-registry]]** (draft): Owns Mother-side lake metadata model.
raw-lake-ingestion implements it for the raw zone.

**[[github-connector]]** (archived, complete): The connector that
proves this spec. All ECs checked — fetch mode works, emits
github.issue/github.pr records via pipe protocol.

**[[mother-broker]]** (archived, complete): Routing engine exists.
raw-lake-ingestion extends it with a lake destination writer.

**[[mother-maturation]]** (draft, container): raw-lake-ingestion
delivers the first concrete lake infrastructure that
mother-maturation has been tracking abstractly.

## 3-Zone Lake Model (context, not scope)

This spec implements the **raw zone** only. The full model:

| Zone | Purpose | V1 status |
|------|---------|-----------|
| **Raw** | Append-only connector output, Parquet files, immutable capture | **This spec** |
| **Curated** | Shared tables/catalog from raw, DuckLake first backend | Deferred — future spec |
| **Serving** | Project projections, indexes, caches, fast query paths | Exists (patina.db) — unchanged |

Raw does not depend on curated or serving. Curated depends on raw.
Serving is independent (fed by project eventlog, not by lake).

## Multi-Source Direction (preserved, not implemented)

The architecture supports multiple sources feeding one lake, multiple
connector types in one lake, and cross-repo/cross-provider queries.
These are visible in the design but not v1 exit criteria:

- Lake layout uses `provider/owner/repo/` — supports multiple sources
- `lake_sync` tracks cursor per source-lake pair — supports multiple
  sources independently
- Different connector types write to different provider prefixes —
  no collision between GitHub and future Google Workspace data
- Curated layer (future) can unify across sources

v1 proves one source → one lake. The layout and metadata model
support N sources → one lake without structural changes.
