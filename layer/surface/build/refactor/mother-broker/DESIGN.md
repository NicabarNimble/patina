# Design: Mother Broker — The Routing Engine

## Why This Work Exists

Mother today manages WASM plugins: spawn wasmtime, call handle(), write
events. But the pipe architecture introduces native children, multiple
runtimes, and explicit source declarations. Mother needs broker
responsibilities — the ability to read "I want GitHub issues for this
repo" and make it happen, regardless of whether the connector is WASM
or native.

The Netflix/Kafka pattern from [[session-20260306-123021]]: Mother is
the broker. She routes facts from sources to destinations based on
declarations. She never transforms data. She manages lifecycle,
resolves credentials, validates schemas, and writes events. Children
produce facts. Mother routes them.

[[mother-holds-connections-pipes-transform]] captures this: "Mother
manages connections, spawns children, routes facts." The broker module
is where this responsibility lives in code.

**Origin:** [[session-20260306-123021]] (Mother as broker, Netflix
pattern, pub/sub declarations), [[session-20260306-174214]] (audit:
transactional cursor+fact writes in same SQLite txn, no fan-out
optimization, WASM fact routing bypass acknowledged).

## What Exists Today

Mother already has substantial infrastructure:

- `src/mother/mod.rs` — project registry, graph, client
- `src/mother/child.rs` — MotherChild trait (WASM children)
- `src/plugin/internal/mother_child.rs` — WasmChild adapter
- `src/plugin/internal/host_support.rs` — emit validation, HTTP proxy
- `src/commands/mother/` — CLI commands, daemon

The broker does NOT replace any of this. It adds a routing layer that
uses existing pieces and extends Mother's CLI with `run` and `sources`.

## Design Decisions

### 1. Unified Child Lifecycle (BrokerChild Trait)

The broker needs to talk to both WASM and native children through one
interface. But the existing `MotherChild` trait is WASM-specific
(handle, health, on_load/on_unload). A new trait bridges both:

```rust
pub trait BrokerChild {
    fn name(&self) -> &str;
    fn fetch(&mut self, params: &FetchParams,
             on_fact: &mut dyn FnMut(Fact) -> Result<()>) -> Result<FetchResult>;
    fn health(&self) -> Result<HealthStatus>;
    fn shutdown(&mut self) -> Result<()>;
}
```

Key difference from Child trait (patina-pipe): BrokerChild is
Mother-side. It wraps the communication channel. NativeChild wraps
a subprocess (stdin/stdout JSON-RPC). WasmBrokerChild wraps an
existing MotherChild (in-process WASM calls).

The `on_fact` callback enables streaming — the broker processes each
fact as it arrives without buffering. For native children, this means
reading interleaved `pipe/fact` notifications from stdout. For WASM
children, facts currently bypass the broker (see decision #5).

### 2. sources.toml — Declarative Source Configuration

Per-project file at `.patina/sources.toml`. Declares what external data
this project wants. Mother reads these across all registered projects.

```toml
[sources.github]
connection = "github"                    # ~/.patina/connections/github.toml
params = { owner = "NicabarNimble", repo = "patina" }
types = ["issues", "prs"]
schedule = "on-scrape"

[sources.github-docs]
connection = "github"                    # same connection, different repo
params = { owner = "NicabarNimble", repo = "docs" }
types = ["issues"]
schedule = "daily"
```

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `connection` | string | yes | Name in `~/.patina/connections/` |
| `params` | table | no | Provider-specific (passed to FetchParams.params) |
| `types` | array | no | Data types to fetch (default: all child capabilities) |
| `schedule` | string | no | "on-scrape", "hourly", "daily", "manual" (default) |

### 3. Transactional Cursor + Fact Writes

The Session 12 audit established this constraint: cursor update and
fact writes MUST be in the same SQLite transaction. Both succeed or
both fail.

**Why this matters:** If facts are written but the cursor isn't
advanced, the next run skips them (thinks they were already fetched).
If the cursor advances but facts aren't written, data is lost. The
only safe model is both in one transaction.

On rollback: cursor stays at the old position, no partial facts
written. Next run re-emits (at-least-once) and content-hash dedup
handles overlap.

```rust
pub fn write_facts_with_cursor(
    conn: &Connection,
    source_name: &str,
    facts: &[ValidatedFact],
    cursor: Option<&str>,
) -> Result<WriteResult> {
    let tx = conn.unchecked_transaction()?;
    // write all facts + update cursor
    tx.commit()?;
}
```

### 4. No Fan-Out Optimization

One child spawn per `run_source()` call. Multiple projects referencing
the same connection get separate spawns. Content-hash dedup handles
data overlap.

This is correct because:
- Each source has its own cursor
- Each source writes to its own project's events.db
- No shared state between runs

The Session 12 audit explicitly decided: no fan-out optimization.
Optimize when measured need exists. Simple is correct until proven
insufficient.

### 5. WASM Children Bypass Broker Routing

WasmBrokerChild wraps existing MotherChild, but WASM facts go directly
to events.db via `host_emit` — the existing `host_support::emit_fact`
path. The broker doesn't intercept them. No content-hash dedup for
WASM children.

This is a conscious trade-off:
- Only forge is WASM. It works. Don't break it.
- Unifying WASM emission through the broker requires changing
  host_emit to route through broker instead of writing directly.
  That's a deeper refactor with risk.
- Accept the asymmetry now. Unify later when the native path proves
  the broker routing works.

[[temporal-layering-causes-drift]] warns about this pattern. Set a
deadline: unify WASM routing through broker when the next WASM child
is built (or explicitly decide not to).

### 6. Fact Validation Extracted from host_support

The validation logic in `host_support::validate_emit()` (schema check,
fact_type check) is reused by the broker's routing engine. The broker
adds content-hash verification on top:

1. `fact.schema` exists in the child's declared schemas
2. `fact.fact_type` exists in that schema
3. `fact.content_hash` matches recomputed hash of `fact.data`
4. Dedup check (data string comparison for now, content_hash index later)

Invalid facts are logged and dropped. Mother never writes unvalidated
data.

### 7. Child Binary Resolution

Search order for finding a child binary:

1. `~/.patina/children/<name>/<name>` — installed children
2. `PATH` — system-installed children
3. `./target/release/<name>` — development builds

This supports both production installs and development workflows.
During development, `cargo build --release` puts the binary in
`target/release/` and `patina mother run github` finds it.

## The Full Flow: run_source()

```
patina mother run github
  |
  +-- Find source "github" in .patina/sources.toml
  |
  +-- Load connection config (~/.patina/connections/github.toml)
  |
  +-- Load child manifest (child.toml)
  |
  +-- Get stored cursor from events.db
  |
  +-- Spawn child (fork+exec in sandbox)
  |     |
  |     +-- pipe/initialize {auth from vault, protocol_version}
  |     |
  |     +-- pipe/fetch {types, since: cursor, params: {owner, repo}}
  |     |     |
  |     |     +-- pipe/fact notifications (streamed, validated, collected)
  |     |
  |     +-- pipe/shutdown
  |
  +-- Write facts + cursor to events.db (single transaction)
  |
  +-- Report: "github: 47 written, 3 dedup, cursor: 2026-03-06T..."
```

## Module Structure

```
src/
  broker/
    mod.rs              # public API: run_source(), status()
    sources.rs          # sources.toml reader
    lifecycle.rs        # BrokerChild trait, NativeChild, WasmBrokerChild
    spawn.rs            # native child spawn (binary resolution, sandbox, init)
    routing.rs          # fact validation + dedup + eventlog write
    cursor.rs           # cursor management (transactional)
```

## CLI Integration

Add to existing `patina mother` subcommands:

```
patina mother run <name>    # run a source (fetch, validate, route)
patina mother sources       # show configured sources with status
```

## On-Scrape Scheduling

After `patina scrape` completes local work, trigger on-scrape sources.
The broker exposes `run_source()` — scrape calls it for each source
with `schedule = "on-scrape"`. Hourly/daily scheduling is
`continuous-operation` scope — the daemon calls the same function on
a timer.

## What's NOT In Scope

- **Scheduling daemon** — `continuous-operation` scope. The broker
  provides `run_source()`. The daemon calls it on schedule.
- **Fan-out optimization** — explicit decision: one spawn per source.
  Optimize when measured.
- **Stream mode lifecycle** — poll mode only. Stream children (always-on,
  health monitoring, restart on crash) is future work.
- **Data architecture changes** — events.db, patina.db, projections
  all stay as-is. The broker changes WHERE code lives, not how data
  flows.
- **Schema auto-installation** — the broker validates against installed
  schemas. Installing schemas from child manifests is future work.

## Belief Anchors

- [[mother-holds-connections-pipes-transform]] — Mother is the broker.
  She routes, she doesn't transform. Children produce facts, Mother
  routes them to destinations.
- [[pipe-protocol-is-transport-agnostic]] — BrokerChild abstracts
  over WASM and native. The broker doesn't care how facts arrived.
- [[host-proxied-io-is-the-security-model]] — credential delivery
  via pipe/initialize. Sandbox prevents direct vault access by
  children.
- [[temporal-layering-causes-drift]] — WASM bypass is acknowledged
  as drift risk. Set a deadline for unification.

## Open Questions

1. **Events.db per project vs shared.** Currently `open_events_db()`
   opens `.patina/local/data/events.db` relative to cwd. The broker
   routes facts to a specific project's events.db. Need a variant
   that accepts a project root path — `open_events_db_at(path)`.

2. **Schema loading scope.** Schemas could be installed globally
   (`~/.patina/schemas/`) or per-project. The design loads from the
   destination project's `.patina/schemas/`. If the schema isn't
   installed, validation fails. Auto-install from child manifest is
   future work.

3. **WASM child fact routing.** WasmBrokerChild bypasses broker
   routing — facts go directly to events.db via host_emit. No
   content-hash dedup for WASM children. Acceptable for now (only
   forge is WASM). When should this be unified?

## Commits

1. `broker: add sources.toml reader` — src/broker/sources.rs with
   SourceEntry, ProjectSources, scan_all_sources(). Parse tests.

2. `broker: add BrokerChild trait with WASM and native adapters` —
   src/broker/lifecycle.rs with trait + NativeChild + WasmBrokerChild.

3. `broker: add native child spawn with sandbox` — src/broker/spawn.rs
   with spawn_native(), resolve_child_binary(), build_init_params(),
   ChildConnection.

4. `broker: add fact routing and schema validation` —
   src/broker/routing.rs with validate_fact(), write_fact_to_eventlog().

5. `broker: add transactional cursor management` —
   src/broker/cursor.rs with write_facts_with_cursor().

6. `broker: add run_source() and status() public API` —
   src/broker/mod.rs orchestrating full flow.

7. `mother: add run and sources CLI commands` — Wire into
   MotherCommands.

8. `scrape: trigger on-scrape sources after local scrape` — Wire
   into scrape command.

## Key Files

- `src/broker/mod.rs` — public API (run_source, status)
- `src/broker/sources.rs` — sources.toml reader
- `src/broker/lifecycle.rs` — BrokerChild trait (WASM + native)
- `src/broker/spawn.rs` — native child spawn with sandbox
- `src/broker/routing.rs` — fact validation + dedup
- `src/broker/cursor.rs` — transactional cursor management
- `src/mother/child.rs` — existing MotherChild trait (WASM)
- `src/plugin/internal/host_support.rs` — validation reference
- `src/commands/mother/mod.rs` — CLI wiring
