---
type: refactor
id: mother-broker
status: ready
created: 2026-03-06
blocked_by:
- pipe-protocol-types
- pipe-native-transport
sessions:
  origin: 20260306-171859
related:
- pipe-architecture
- continuous-operation
beliefs:
- mother-holds-connections-pipes-transform
exit_criteria:
- id: mother-spawns-children
  text: Mother spawns children (WASM and native) based on sources.toml config — uniform lifecycle management for both runtimes
  checked: false
  verify: '`patina mother run test` spawns test-child (native). `patina scrape forge` triggers forge (WASM). Both complete successfully. Verify via process list or Mother logs.'
- id: mother-routes-facts
  text: Mother routes facts from children to destination events.db — sources.toml declarations determine where facts go (fan-out is config, not child logic)
  checked: false
  verify: 'After `patina mother run github`: `SELECT count(*) FROM eventlog WHERE source_id = ''child:github-connector''` returns > 0 in project events.db specified by sources.toml.'
- id: mother-validates-schemas
  text: Mother validates emitted facts against declared schemas in child manifest — undeclared schemas are dropped with a warning
  checked: false
  verify: 'Modify test-child to emit a fact with schema "bogus". Confirm: fact logged as dropped (warning in Mother output), not in events.db. Valid facts from same run ARE present.'
- id: mother-run-test-child
  text: '`patina mother run test` spawns test-child (from pipe-native-transport examples/), routes facts to events.db — proves broker works before production connector'
  checked: false
  verify: '`patina mother run test` outputs fact count > 0. `SELECT event_type, count(*) FROM eventlog WHERE source_id = ''child:test-child'' GROUP BY event_type` returns rows.'
- id: mother-run-github
  text: '`patina mother run github` spawns github-connector, routes github.* facts to project events.db with content-hash dedup — tests routing, not child correctness (see [[spec-github-connector]] EC mother-run-works for child protocol verification). Verified after [[spec-github-connector]] is complete.'
  checked: false
  verify: Run `patina mother run github`. Output shows fact count, dedup count, cursor value. `SELECT event_type, count(*) FROM eventlog WHERE event_type LIKE 'github.%' GROUP BY event_type` returns rows. Run again immediately — dedup count should equal fact count (all duplicates).
- id: credentials-via-pipe
  text: Children receive credentials via pipe/initialize config delivery (Mother reads connection config, decrypts from vault, passes via stdin) — not via environment variables or files
  checked: false
  verify: '`PATINA_SANDBOX_DEBUG=1 patina mother run github` — inspect logged pipe/initialize params showing auth.token present. Confirm: no GITHUB_TOKEN env var set, no temp credential files created.'
- id: sandbox-enforcement
  text: Mother refuses to spawn a native child when the OS cannot enforce sandboxing (Landlock v4 unavailable on Linux, sandbox_init() failure on macOS) unless --no-sandbox is explicitly passed. Sandbox enforcement is a broker responsibility — children do not self-sandbox.
  checked: false
  verify: 'On Linux <6.7: `patina mother run test` fails with explicit Landlock error. `patina mother run test --no-sandbox` succeeds with warning. On macOS: invalid sandbox profile causes spawn refusal with Apple error message.'
- id: wasm-routing-resolved
  text: WASM fact routing unified through broker (forge facts go through broker validation + dedup like native children) OR explicitly decided that forge stays on legacy host_emit path with no new WASM children permitted to bypass broker — decision documented in DESIGN.md
  checked: false
  verify: 'If unified: forge facts in events.db have content_hash values, dedup works across WASM/native. If legacy: DESIGN.md §5 documents decision with rationale, code comment in host_emit marks path as frozen.'
- id: mother-status-works
  text: '`patina mother status` shows running children and health — lifecycle state, last run, fact count, errors'
  checked: false
  verify: 'After running github-connector and test-child: `patina mother status` shows both with name, lifecycle state, last run timestamp, fact count, and error count.'
---
# refactor: Mother Broker — Routing Engine + Child Lifecycle

> Mother becomes the broker. Routes facts from children to
> destinations based on pub/sub declarations. Manages child lifecycle
> (spawn, health, restart, shutdown) for both WASM and native
> children. Netflix/Kafka pattern.

## Context

[[spec-pipe-architecture]] defines Mother as the broker — the central
node that manages children and routes facts. This spec builds that
broker capability.

**What Mother does today:**
- Project registry (`mother.db → project_registry`)
- Ref repo registry (cross-project git references)
- Mother-child WASM plugin world (spawn, heartbeat, tick, health)
- Cross-project belief search (`patina mother search`)
- Graph.db for federated FTS5 search

**What this spec adds:**
- Routing engine — read sources.toml, match source→destination, fan-out
- Child lifecycle management — spawn (WASM or native), health,
  restart, shutdown via uniform interface
- Schema validation — validate emitted facts against child manifest
- Scheduling — on-scrape, hourly, daily, stream, manual modes
- `patina mother run/status/health/logs` CLI commands

## Current State

Mother manages WASM plugins via the mother-child world but has no
concept of:
- Destination declarations (sources.toml)
- Fact routing (child → events.db per destination)
- Native child spawning (only WASM)
- Schema validation at the broker level

## Target State

```
MOTHER (broker)
  ├── sources.toml reader
  │   └── for each source: connection → child → params → schedule
  ├── Child lifecycle manager
  │   ├── WASM: existing mother-child world (wasmtime)
  │   └── Native: fork+exec in sandbox, stdio pipe protocol
  ├── Routing engine
  │   ├── pipe/fact received → validate schema → route to destination
  │   └── fan-out: one child → multiple destinations (content-hash dedup)
  ├── Schema validator
  │   └── child manifest declares schemas → Mother checks each fact
  └── Scheduler
      ├── manual: `patina mother run <name>`
      ├── on-scrape: triggered by `patina scrape`
      └── scheduled: hourly/daily (via continuous-operation daemon)
```

### Destination Declarations (sources.toml)

```toml
# .patina/sources.toml (project-level)
[sources.github]
connection = "github"
params = { owner = "NicabarNimble", repo = "patina" }
types = ["issues", "prs"]
schedule = "on-scrape"
```

Mother reads sources.toml from all registered projects. For each
source entry: resolve connection → determine child → build fetch
params → spawn child → route emitted facts to project events.db.

## Steps

1. Define sources.toml format specification
2. Implement sources.toml reader in Mother (scan registered projects)
3. Build unified child lifecycle interface (trait over WASM and native)
4. Implement native child spawn: fork+exec in sandbox, connect stdio
5. Implement routing engine: receive pipe/fact, validate schema,
   write to destination events.db
6. Implement fan-out: multiple sources.toml entries for same connection
   → shared spawn or separate spawns with dedup
7. Add `patina mother run <name>` — manual trigger (spawn child,
   route facts, report results)
8. Add `patina mother status` — show children, last run, health
9. Wire scheduling modes (manual first, on-scrape second, daemon
   scheduled via [[spec-continuous-operation]])
10. Verify: `patina mother run github` produces events in project
    events.db, facts are schema-validated

## Key Files

**Build on:**
- `src/plugin/internal/mod.rs` — existing WASM child lifecycle
- `src/plugin/internal/host_support.rs` — emit_fact validation,
  schema checking (extract to shared broker logic)
- `src/commands/mother.rs` — existing mother CLI commands
- [[spec-pipe-architecture]] DESIGN.md §4 (Mother as Broker),
  §2.3 (Child Lifecycle), §4.4 (Schema Validation)

**New:**
- `src/broker/mod.rs` — routing engine
- `src/broker/lifecycle.rs` — unified child lifecycle (WASM + native)
- `src/broker/routing.rs` — fact routing + fan-out
- `src/broker/validation.rs` — schema validation

## Design Constraints (from architecture review, session 20260306-174214)

- **Cursor update must be transactional with fact writes.** When
  Mother writes facts to destination events.db and updates the `since`
  cursor, both must happen in the same SQLite transaction. Otherwise:
  facts written + cursor not advanced = harmless re-fetch on next run
  (dedup handles it), but cursor advanced + facts not written = data
  loss. Same transaction eliminates both failure modes. (Helland lens:
  at-least-once requires the acknowledgment and the write to be
  atomic.)

- **No fan-out optimization.** Separate child spawns per destination
  only. No shared-spawn optimization (one child serving multiple
  destinations) until there is a measured performance need. Simplicity
  first. (Kelley lens: solve problems you have, not problems you
  imagine.)

## Non-Goals

- P2P sync between Mothers (future, needs persona-federation)
- Encryption at rest (future, needs persona keypair)
- Edge deployment (future, separate spec)
- Building connectors (that's per-connector specs like
  [[spec-github-connector]])
- OAuth/connection management (that's [[spec-patina-connect]])
- Daemon auto-start (that's [[spec-continuous-operation]])
