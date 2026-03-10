---
type: feat
id: ducklake
status: draft
created: 2026-03-10
sessions:
  origin: 20260310-074810
related:
- raw-lake-ingestion
- lake-registry
- data-architecture-v3
- patina-connect
- pipe-architecture
beliefs:
- connectors-own-tables-schemas-are-contracts
- mother-holds-connections-pipes-transform
- mother-owns-destination-format
- raw-lake-is-capture-contract-first
- children-have-agency-toys-are-capabilities
- initialize-is-capability-grant
exit_criteria:
- id: ducklake-child-exists
  text: "A DuckLake child binary exists in children/ducklake/, embeds DuckDB+DuckLake extension, uses approved connector toys, manages catalog and Parquet files autonomously"
  checked: false
- id: child-drives-pipeline
  text: "DuckLake child drives the fetch→store cycle: uses its approved connector toy, handles partial failures per data type, advances cursors independently per type"
  checked: false
- id: mother-grants-capabilities
  text: "Mother grants toy approvals via pipe/initialize (lake path, connector binary, credentials, domain allowlist); Mother does not manage the data flow after capability grant"
  checked: false
- id: error-escalation
  text: "Child handles data flow errors (partial fetch, connector failures). Auth/credential errors escalate to Mother. Mother talks to user about infrastructure problems."
  checked: false
- id: ducklake-queryable
  text: "Lake is queryable standalone via DuckDB CLI: `duckdb <lake_path>/lake.ducklake` → `SELECT * FROM issues`"
  checked: false
- id: litmus-claude-code
  text: "End-to-end: Mother grants DuckLake child toy approvals for anthropics/claude-code issues+PRs, child uses connector toy to fetch and store data, queryable via DuckDB"
  checked: false
---
# feat: DuckLake — DuckDB Child with Local Parquet Lake

> Mother grants capabilities. She spins up a DuckLake child and
> approves its toys: a connector, credentials, a lake path, and
> an HTTP domain allowlist. The child uses those toys on its own —
> fetch, store, handle failures. Mother is capability grantor and
> boundary setter, not runtime dispatcher.
> Per [[children-have-agency-toys-are-capabilities]] and
> [[initialize-is-capability-grant]].

## Problem

Patina has working connector infrastructure: github-connector
emits records via pipe protocol, Mother's broker routes facts to
project events.db, cursor tracking works. But all connector output
goes to project-scoped SQLite. There is no shared data lake.

Users need a way to say "I want a datalake with GitHub issues and
PRs from the claude-code repo" and have Patina set it up.

## Solution

### Three Roles: Mother, Child, Toy

```
Mother              DuckLake Child          Connector (toy)
(capability grant)  (agency — decisions)    (capability — does work)
────────────────    ───────────────         ──────────────
grants toys         drives fetch cycle      fetches from GitHub
approves connector  handles partial success rate limits → backoff
approves credentials per-type cursors       pagination → follows
approves domains    saves what it got       network error → retry
escalation target   reports progress        reports failure to child
talks to user       to user
about infra issues  escalates auth to Mother
knows lake exists   uses toys independently
```

**Mother** is the capability grantor. She creates the lake,
approves which toys the child can use (connector binary,
credentials, storage path, domain allowlist), and handles
escalation for things she owns (auth, connections). She does
NOT manage the data flow. Per [[initialize-is-capability-grant]],
the init payload IS the capability token set.

**DuckLake child** has agency. It makes decisions: what to fetch,
how to handle failures, when to advance cursors, what to report.
It uses its approved toys independently — spawns the connector
toy, drives fetch, stores results. Mother is not in the loop.

**Connector** is a toy — a capability that does work when asked.
It fetches data from GitHub APIs, handles rate limits, pagination,
and network errors. It has no agency — it reports results to the
child, which decides what to do. Per
[[children-have-agency-toys-are-capabilities]]: classify by
agency, not by runtime.

### Mother as Capability Grantor

Mother's job is granting capabilities and getting out of the way:

1. **Hear "datalake"** → knows this means DuckDB + DuckLake
2. **Create lake** → `~/.patina/lakes/<name>/`
3. **Resolve connection** → find connector binary + credentials
4. **Spawn DuckLake child** with capability grant via
   `pipe/initialize`:
   - Storage toy: lake path
   - Connector toy: binary path + credential + params + types
   - HTTP toy: approved domain allowlist
5. **(Optional) add ref repo** → `patina repo add` if not present
6. **Get out of the way** → child uses toys independently

After the capability grant, Mother is available for escalation
and status queries only. The child uses its approved toys and
talks to the user during operation.

### DuckLake Child — Autonomous Lake Manager

The child is the lake. It embeds DuckDB + DuckLake and uses
its approved toys:

- **Uses connector toy** — spawns the approved connector
  binary with approved credentials, on its own schedule
- **Drives fetch cycle** — sends pipe/fetch, receives facts
- **Handles partial success** — issues fetched but PRs rate
  limited? Save the issues, advance the issues cursor, report
  PR failure. Not atomic — save what you can.
- **Per-type cursors** — each data type (issues, prs) has its
  own cursor. Partial failure doesn't block successful types.
- **Auto-creates tables** on first encounter
- **Reports to user** — progress, results, failures
- **Escalates to Mother** — only for things the child can't fix
  (auth broken, connector binary missing)

```
children/ducklake/
  Cargo.toml          # depends on patina-pipe, duckdb
  child.toml          # type=lakehouse, runtime=native
  src/
    main.rs           # Child impl, connector management, ingest
```

### Error Handling — Who Handles What

| Error | Who handles | What happens |
|-------|------------|--------------|
| API rate limit | Connector | Backs off, retries, reports if exhausted |
| Network timeout | Connector | Retries, reports failure to child |
| Partial fetch (issues ok, PRs failed) | Child | Saves issues, advances issues cursor, reports PR failure to user |
| Connector crash | Child | Saves what it got, reports to user, suggests re-run |
| Auth expired / credential missing | Child → Mother | Child detects auth error, escalates to Mother, Mother tells user to `patina connect refresh` |
| Lake disk full | Child | Reports to user, does not advance cursors |
| Connector binary not found | Child | Fails when child tries to use approved connector toy; escalates to Mother |

### Cursor Tracking

Per-type cursors inside the DuckLake catalog:

```sql
CREATE TABLE IF NOT EXISTS _sync_cursors (
    source_name VARCHAR,
    data_type   VARCHAR,
    cursor      VARCHAR,
    last_run    TIMESTAMP,
    records_written BIGINT DEFAULT 0,
    status      VARCHAR DEFAULT 'ok',
    last_error  VARCHAR,
    PRIMARY KEY (source_name, data_type)
);
```

Issues and PRs have independent cursors. If issues succeeds but
PRs fails, next run only re-fetches PRs. The lake tracks what
worked and what didn't.

### Table Creation

On first ingest for a new event type:

```sql
CREATE TABLE IF NOT EXISTS issues (
    _ingested_at  TIMESTAMP DEFAULT current_timestamp,
    _source_id    VARCHAR,
    _content_hash VARCHAR,
    data          JSON
);
```

V1 stores records as JSON columns. DuckDB handles JSON natively.

### Query Path

Queryable standalone — no Patina needed:

```bash
duckdb ~/.patina/lakes/github-data/lake.ducklake
```
```sql
SELECT * FROM issues WHERE data->>'state' = 'open';
SELECT data->>'title' FROM prs ORDER BY data->>'number' DESC;
```

### What Mother Knows

Minimal — just enough for orchestration:

- Lake name and location
- What connector feeds it
- Whether the child is running

The DuckLake catalog is the source of truth for everything else.

## The Full Flow

```
User: "I need a datalake with GitHub issues+PRs from claude-code"

Mother (capability grant):
  ├── 1. Create lake "github-data" at ~/.patina/lakes/github-data/
  ├── 2. Resolve github connection + credentials
  ├── 3. Spawn DuckLake child with capability grant (pipe/initialize):
  │      ├── storage toy:   lake_path = ~/.patina/lakes/github-data/
  │      ├── connector toy: binary = github-connector
  │      ├── credential:    decrypted PAT
  │      ├── http toy:      allowed_domains = [api.github.com]
  │      ├── params: { owner: "anthropics", repo: "claude-code" }
  │      └── types: ["issues", "prs"]
  ├── 4. Send pipe/run — child drives from here
  └── 5. Done — Mother out of data path

DuckLake child (uses approved toys):
  ├── 1. Initialize DuckDB + DuckLake at approved lake_path
  ├── 2. Use connector toy: spawn approved binary with credentials
  ├── 3. Load cursors from _sync_cursors
  ├── 4. pipe/fetch issues (cursor: 2026-03-10...)
  │      ├── connector fetches from GitHub API
  │      └── 847 issue records received
  ├── 5. CREATE TABLE IF NOT EXISTS issues
  ├── 6. INSERT 847 records
  ├── 7. Update issues cursor → 2026-03-12T09:30:00Z
  ├── 8. pipe/fetch prs (cursor: none — first run)
  │      ├── connector fetches from GitHub API
  │      └── 312 pr records received
  ├── 9. CREATE TABLE IF NOT EXISTS prs
  ├── 10. INSERT 312 records
  ├── 11. Update prs cursor → 2026-03-12T08:45:00Z
  ├── 12. Report to Mother (pipe/run response):
  │       "github-data: 847 issues ✓, 312 prs ✓"
  └── 13. Shutdown connector toy, done

Query: duckdb ~/.patina/lakes/github-data/lake.ducklake
```

## Steps

1. Build DuckLake child: `children/ducklake/` — embedded DuckDB +
   DuckLake, uses approved connector toy, fetch→store cycle,
   per-type cursors, partial failure handling, reporting
2. Add Mother capability grant — resolve connection, spawn child
   with toy approvals via pipe/initialize, send pipe/run, handle
   escalation
3. Add `patina lake create <name>` command
4. Add Mother lake awareness — `patina mother status` shows lakes
5. End-to-end litmus: anthropics/claude-code issues+PRs

## Key Files

**New:**
- `children/ducklake/Cargo.toml` — patina-pipe, duckdb
- `children/ducklake/child.toml` — type=lakehouse, runtime=native
- `children/ducklake/src/main.rs` — autonomous child impl
- `src/commands/lake.rs` — `patina lake create`, `patina lake list`

**Extend:**
- `src/broker/mod.rs` — lake capability grant (spawn child, grant toys via init)
- `src/commands/mother.rs` — lake state in status

**Extend (patina-pipe + pipe-types):**
- `crates/patina-pipe/src/lib.rs` — add `pipe/run` dispatch + `run()` to Child trait
- `crates/patina-pipe-types/src/config.rs` — `InitializeParams` must accept child-type-specific toy approvals (Option C: child reads extra fields from raw params)
- `crates/patina-pipe-types/src/` — add `RunResult`, `TypeReport`, `Escalation` types

**Unchanged:**
- `children/github-connector/` — connector toy code unchanged

## Non-Goals

- Columnar Parquet extraction — follow-on
- S3 backend — next spec (DuckLake config change)
- Transform layer / data blocks — future spec
- Multiple lakes per source — v1 is one source → one lake
- Persona scoping — future
- Custom Parquet writer / dedup — DuckLake handles this
- Real-time / streaming — poll mode only
- Interactive lake setup UX — hardwired defaults for now

## Relationship to Other Specs

**Supersedes [[raw-lake-ingestion]]** (draft → abandon): DuckLake
replaces the custom lakehouse child. The old design had Mother
driving pipe/ingest to a passive lakehouse. The new design gives
the child agency — it uses approved toys on its own.

**Supersedes [[lake-registry]]** (draft → abandon): DuckLake
catalog is the real registry. Mother tracks names and paths only.

**Builds on [[pipe-architecture]]**: Connector toy speaks pipe
protocol. Child uses pipe protocol to drive the connector toy.

**Depends on [[http-proxy-extraction]]**: Child uses shared HTTP
proxy from patina-pipe to securely proxy its connector toy's HTTP.
The proxy is a capability toy the child builds from its approved
domain allowlist and credentials.

**Builds on [[patina-connect]]**: Mother resolves connections and
credentials, grants them to the child as toy approvals in
pipe/initialize. Per [[initialize-is-capability-grant]].

## Roadmap (context, not scope)

| Phase | What | Status |
|-------|------|--------|
| **1. DuckLake local** | Autonomous DuckLake child, local Parquet | **This spec** |
| **2. DuckLake S3** | Same child, S3 storage backend | Future spec |
| **3. Transform layer** | App reads lake → produces data blocks | Future spec |
