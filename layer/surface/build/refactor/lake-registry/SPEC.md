---
type: refactor
id: lake-registry
status: draft
created: 2026-03-05
sessions:
  origin: 20260305-132827
related:
- data-architecture-v3
- mother-maturation
- continuous-operation
- raw-lake-ingestion
- pipe-architecture
beliefs:
- mother-holds-connections-pipes-transform
- patina-is-domain-agnostic-knowledge-system
exit_criteria:
- id: lake-registry-table
  text: "graph.db contains lake_registry table; Mother creates it on startup; `patina lake create <name>` registers a lake with name, location, and creation timestamp"
  checked: false
- id: lake-sync-table
  text: "graph.db contains lake_sync table tracking cursor, last run, records written, and status per source-lake pair"
  checked: false
- id: lake-status-visible
  text: "`patina mother status` shows registered lakes with sync state (last run, record count, status)"
  checked: false
---
# refactor: Lake Registry — Mother-Managed Lake Metadata

> Mother registers lakes, tracks sync state, and exposes metadata.
> Lakes are Mother-scoped shared resources, not project-local
> artifacts. The registry is the control plane — it knows WHERE
> data lives and WHEN it was last synced, not WHAT the data contains.

## Current State

Mother manages projects (project_registry) and ref repos in graph.db.
There is no lake concept in Mother. data-architecture-v3 designed a
`lake_registry` table but the implementation was split to this spec.

## Target State

### Lake Registry Table

```sql
CREATE TABLE IF NOT EXISTS lake_registry (
    name        TEXT NOT NULL,
    persona     TEXT NOT NULL DEFAULT 'default',  -- persona scope; part of PK
    location    TEXT NOT NULL,       -- filesystem path (e.g. ~/.patina/lakes/github-data)
    created_at  TEXT NOT NULL,       -- ISO 8601
    metadata    TEXT,                -- JSON: provider, description, zone info
    PRIMARY KEY (name, persona)
);
```

**Persona as part of the primary key:** Two personas can safely reuse
the same lake name — they are distinct lakes scoped to different
personas. V1 uses `'default'` for the single implicit persona.
When persona-federation ships, the persona value becomes a real
persona identifier. The key structure doesn't need to change.

### Lake Sync Table

```sql
CREATE TABLE IF NOT EXISTS lake_sync (
    lake_name   TEXT NOT NULL,
    persona     TEXT NOT NULL DEFAULT 'default',  -- matches lake_registry
    source_name TEXT NOT NULL,       -- from sources.toml
    cursor      TEXT,                -- opaque, connector-owned semantics
    last_run    TEXT,                -- ISO 8601
    records_written INTEGER DEFAULT 0,
    status      TEXT DEFAULT 'never_run',  -- never_run, ok, error
    error       TEXT,                -- last error message if status=error
    PRIMARY KEY (lake_name, persona, source_name)
);
```

**Persona in sync key:** A source feeding lake "github-data" under
persona A tracks cursor independently from the same source under
persona B. This prevents cross-persona cursor contamination.

### Commands

- `patina lake create <name>` — register lake, create directory
- `patina lake list` — show registered lakes
- `patina mother status` — includes lake sync state

### Lake Location

Lakes are Mother-scoped: `~/.patina/lakes/<name>/`

Default location is derived from lake name. Custom location may be
supported in future but is not v1.

## Steps

1. Add `lake_registry` and `lake_sync` tables to graph.db schema
   creation (in Mother startup)
2. Implement `patina lake create <name>` — inserts into lake_registry,
   creates `~/.patina/lakes/<name>/raw/` directory
3. Implement `patina lake list` — reads lake_registry
4. Extend `patina mother status` to include lake sync state from
   lake_sync table
5. Lake sync table is written by [[raw-lake-ingestion]]'s lake
   writer — this spec defines the schema, raw-lake-ingestion populates it

## 3-Zone Lake Model

Lakes have three zones. This spec and [[raw-lake-ingestion]] implement
the raw zone only. Curated and serving are direction, not scope.

| Zone | Purpose | V1 status |
|------|---------|-----------|
| **Raw** | Append-only Parquet from connectors, immutable capture, connector-owned record shapes | Implemented by [[raw-lake-ingestion]] |
| **Curated** | Shared tables/catalog from raw, DuckLake first backend, abstract enough for Iceberg later | Future spec — not connector's responsibility |
| **Serving** | Project projections, indexes, caches, consumer-facing fast paths | Exists as patina.db — separate from lake |

Raw does not depend on curated. Curated reads from raw. Serving is
independent (fed by project eventlog and project-scope materialization).

## Non-Goals

- Lake storage format (Parquet details) — that's [[raw-lake-ingestion]]
- Curated layer implementation — future spec
- Serving layer changes — current patina.db continues
- Lake-to-lake replication — future
- Remote lake locations (S3, etc.) — future
- Lake deletion or cleanup — future
- Catalog queries (`patina assay` on lake metadata) — future, after
  curated layer exists

## Exit Criteria

- **lake-registry-table:** graph.db lake_registry table exists, created
  on Mother startup, populated by `patina lake create`
- **lake-sync-table:** graph.db lake_sync table exists, tracks per-source
  cursor and sync state
- **lake-status-visible:** `patina mother status` displays lake
  registration and sync state
