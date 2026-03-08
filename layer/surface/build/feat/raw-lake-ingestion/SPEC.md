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
  text: "Mother registers a lake by name, persona, and location in graph.db lake_registry table; `patina mother status` shows lake name, location, and sync state"
  checked: false
- id: source-declares-lake-destination
  text: "sources.toml supports a destination field that routes connector output to a named lake instead of project events.db"
  checked: false
- id: lakehouse-child-exists
  text: "A lakehouse child binary exists in children/lakehouse/, speaks pipe protocol, receives records via pipe/ingest, writes append-only Parquet files to a configured lake path"
  checked: false
- id: parquet-layout-partitioned
  text: "Lakehouse child writes Parquet in a provider-partitioned layout: `<lake>/raw/<provider>/<owner>/<repo>/<data_type>/` with time-based file naming"
  checked: false
- id: mother-routes-to-lakehouse
  text: "Mother routes connector output to lakehouse child via pipe protocol — Mother never touches Parquet format, file layout, or storage mechanics"
  checked: false
- id: cursor-sync-tracked
  text: "Mother tracks sync cursor per source-lake pair; cursor update follows successful lakehouse write confirmation; re-running ingestion fetches only new/updated records"
  checked: false
- id: dedup-enforced
  text: "Lakehouse child enforces idempotent append using connector-declared identity fields; duplicate records are not written to Parquet"
  checked: false
- id: provenance-on-lake-records
  text: "Written Parquet records carry provenance metadata: source connector, ingestion timestamp, content hash"
  checked: false
- id: v1-litmus-github
  text: "End-to-end: configure GitHub connection, declare repo as lake source, run ingestion, raw issue/PR data lands as Parquet via lakehouse child, Mother shows metadata, DuckDB can query the files"
  checked: false
---
# feat: Raw Lake Ingestion — V1 Append-Only Parquet Capture

> Connector emits domain records. Mother routes to lakehouse child.
> Lakehouse child materializes append-only Parquet. Mother tracks
> registry and cursor truth. Raw lake has no catalog, no shared
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
one source, one lake, one lakehouse child.

## Solution

### Role Boundaries

Five actors, strict boundaries:

```
Connector child          Mother                  Lakehouse child
source-boundary adapter  node-local control      storage-boundary worker
                         plane

owns:                    owns:                   owns:
  domain record shape      routing policy          Parquet format
  schema/version           lake registry           file layout
  identity fields          cursor truth            append mechanics
  API behavior             persona scoping         storage-local dedup
  source semantics         lifecycle management    provenance columns

does NOT:                does NOT:               does NOT:
  know about Parquet       write storage           know about GitHub
  know about lakes         know Parquet format     know source semantics
  own dedup enforcement    own file layout         own routing policy
  own cursor truth         execute data-plane      own cursor truth
```

**Mother governs. Children execute bounded roles.** Mother never
touches Parquet format, file layout, or storage mechanics. The
lakehouse child is a real child from day one — not an inline module
inside Mother.

### Capture Contract (primary concern)

The capture contract defines what a connector emits, how Mother
routes it, and what the lakehouse child writes.

**Connector responsibility:**
- Emit stable domain records with typed fields via pipe protocol
- Declare schema and version in child manifest
- Provide identity fields for dedup (e.g. `number`, `updated_at`)
- Handle API pagination, rate limiting, cursor semantics
- Source-boundary adapter: may ingest from external system AND may
  apply changes back, but never becomes storage or coordination

**Mother responsibility:**
- Route records to declared destination (project events.db OR lake)
- Register lake: name, location, persona, sync state
- Track sync cursor per source-lake pair (advances after lakehouse
  confirms write)
- Spawn and lifecycle-manage connector and lakehouse children
- Policy: when to run, what to route where

**Lakehouse child responsibility:**
- Receive records from Mother via pipe protocol (`pipe/ingest`)
- Write append-only Parquet files in partitioned layout
- Enforce idempotent append using identity fields from schema
- Own file naming, directory creation, Parquet serialization
- Attach provenance metadata columns to written records
- Report write results (count, paths) back to Mother

**The connector does not know about Parquet.** It emits domain
records. **Mother does not know about Parquet.** She routes records
to the lakehouse child. Only the lakehouse child knows about Parquet.

### Two-Child Pipeline

```
1. Mother spawns connector child (github-connector)
2. Connector fetches from GitHub API, emits records to Mother
3. Mother validates records against declared schema
4. Mother spawns lakehouse child with lake config
5. Mother sends records to lakehouse child (pipe/ingest)
6. Lakehouse child dedup-checks, writes Parquet, reports results
7. Mother updates lake_sync cursor after confirmed write
```

This is the same broker pattern as the existing project path, but
with a lakehouse child at the destination instead of events.db.
Mother routes. Children execute.

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
the lakehouse child instead of project events.db. The lake is
identified by name, resolved from Mother's lake registry.

When `destination.type` is absent or `"project"`, the existing broker
path writes to events.db (no change to current behavior).

### Raw Lake Storage

Owned by the lakehouse child, not by Mother:

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

The lakehouse child owns all layout decisions. Mother passes the
lake root path; the child decides directory structure.

### Lake Registration in Mother

Mother tracks lakes in `graph.db`:

```sql
CREATE TABLE IF NOT EXISTS lake_registry (
    name        TEXT NOT NULL,
    persona     TEXT NOT NULL DEFAULT 'default',
    location    TEXT NOT NULL,       -- filesystem path
    created_at  TEXT NOT NULL,
    metadata    TEXT,                -- JSON: provider, description
    PRIMARY KEY (name, persona)
);

CREATE TABLE IF NOT EXISTS lake_sync (
    lake_name   TEXT NOT NULL,
    persona     TEXT NOT NULL DEFAULT 'default',
    source_name TEXT NOT NULL,       -- from sources.toml
    cursor      TEXT,                -- opaque, connector-owned semantics
    last_run    TEXT,                -- ISO 8601
    records_written INTEGER DEFAULT 0,
    status      TEXT DEFAULT 'never_run',
    PRIMARY KEY (lake_name, persona, source_name)
);
```

Lakes are persona-scoped from day one. Persona is part of the
primary key in both tables, so two personas can safely reuse the
same lake name and source name without collision. V1 uses
`'default'` as the single implicit persona. When persona-federation
ships, the persona value becomes a real identifier — the key
structure doesn't change. This is forward-compatible keying, not
full persona architecture (namespace isolation, sync policy) which
is [[persona-federation]] scope.

### Dedup and Idempotency

The lakehouse child owns dedup. It reads previous Parquet files for
the same data type partition to build an identity index. New records
whose identity matches an existing record with the same content are
skipped. Changed records (same identity, different content) are
appended — the raw zone captures history, not just current state.

**Identity fields** come from the connector's schema declaration.
Mother passes them to the lakehouse child as part of the ingest
configuration. The connector doesn't participate in dedup — it emits
all records. The lakehouse child filters.

```toml
# children/github-connector/schema.toml
[[facts]]
name = "issue"
event_type = "github.issue"
identity_fields = ["number"]
```

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
   (with `persona` column)
3. Add `patina lake create <name>` — register a lake with Mother,
   create root directory
4. Build lakehouse child: `children/lakehouse/` with Child trait
   impl, Parquet writer, dedup, layout management
5. Define `pipe/ingest` method: Mother sends records to lakehouse
   child with lake path and schema config
6. Wire broker routing: when source has lake destination, spawn
   lakehouse child and route records via pipe/ingest
7. Lakehouse child reports write results; Mother updates lake_sync
   cursor
8. Update `patina mother status` to show lake sync state
9. End-to-end verification: github connection → lake source → run →
   connector emits → Mother routes → lakehouse writes Parquet →
   DuckDB query

## Key Files

**Extend:**
- `src/broker/mod.rs` — routing decision (project vs lake)
- `src/broker/sources.rs` — destination field in sources.toml
- `src/mother/` — graph.db schema (lake_registry, lake_sync tables)
- `crates/patina-pipe-types/` — add pipe/ingest method types

**New:**
- `children/lakehouse/Cargo.toml` — lakehouse child crate
- `children/lakehouse/child.toml` — type=lakehouse, runtime=native
- `children/lakehouse/src/main.rs` — Child trait impl, pipe/ingest
- `children/lakehouse/src/writer.rs` — Parquet serialization
- `children/lakehouse/src/dedup.rs` — identity-based dedup
- `children/lakehouse/src/layout.rs` — path conventions
- `src/commands/lake/` — `patina lake create`, `patina lake query`

**Reference (no changes):**
- `children/github-connector/` — emits records, unchanged
- `src/broker/routing.rs` — fact validation, unchanged
- `src/broker/lifecycle.rs` — child lifecycle, unchanged

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
- Full persona architecture — forward-compatible column only
- Inline lakehouse inside Mother — lakehouse is a real child

## Relationship to Other Specs

**[[pipe-architecture]]** (active, container): Provides vocabulary.
Data Layers: Sources → Lakes → Projects → Beliefs. raw-lake-ingestion
implements the Sources → Lakes arrow. Child taxonomy: connector
(source boundary), lakehouse (storage boundary). Both roles proven
here.

**[[connector-owns-tables]]** (draft): Owns project-scope
materialization (events → SQLite read models). raw-lake-ingestion
owns lake-scope capture (records → Parquet via lakehouse child).
Together they prove the destination-aware model works across scopes.

**[[lake-registry]]** (draft): Owns Mother-side lake metadata model.
raw-lake-ingestion implements it for the raw zone.

**[[github-connector]]** (archived, complete): The connector that
proves this spec. All ECs checked — fetch mode works, emits
github.issue/github.pr records via pipe protocol.

**[[mother-broker]]** (archived, complete): Routing engine exists.
raw-lake-ingestion extends it with a lake destination route to
the lakehouse child.

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

## Role Alignment Test

Five questions that must all be "yes":

1. Can the connector be replaced without touching storage? **Yes** —
   connector emits records, lakehouse child writes them. Different
   children, no coupling.
2. Can the lakehouse be replaced without touching connector code?
   **Yes** — swap Parquet for another format by replacing the
   lakehouse child. Connector unchanged.
3. Can Mothers sync beliefs without exposing raw lakes/blocks?
   **Yes** — beliefs are the sync layer. Lakes are node-local.
4. Is persona-aware keying in place before network sharing? **Yes** —
   persona is part of the primary key in lake_registry and lake_sync
   from day one. This is forward-compatible keying, not enforcement —
   real isolation (keypair validation, cross-persona denial) ships
   with persona-federation.
5. Is Mother governing rather than executing data-plane concerns?
   **Yes** — Mother routes to lakehouse child. Mother never touches
   Parquet, file layout, or storage mechanics.
