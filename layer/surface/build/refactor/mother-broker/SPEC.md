---
type: refactor
id: mother-broker
status: active
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
  text: 'Mother spawns native children based on sources.toml config via BrokerChild trait. WASM children are wrapped by BrokerChild but routing stays on legacy host_emit path (see DESIGN.md §5, gated by EC wasm-routing-resolved).'
  checked: true
  verify: '`patina mother run test` spawns test-child (native), routes facts through broker. `patina scrape forge` triggers forge (WASM) via existing path. Both complete successfully. Native facts have content_hash and source_id=child:test-child; WASM facts use source_id=plugin:forge.'
- id: mother-routes-facts
  text: 'Mother routes facts from native children to destination events.db — sources.toml declarations determine where facts go, one spawn per source (no shared-spawn fan-out in v1, see DESIGN.md §4)'
  checked: true
  verify: 'After `patina mother run github`: `SELECT count(*) FROM eventlog WHERE source_id = ''child:github-connector''` returns > 0 in project events.db specified by sources.toml.'
- id: mother-validates-schemas
  text: 'Mother validates emitted facts against declared schemas in child manifest — undeclared schemas are dropped with a warning (see DESIGN.md §6 decision table: undeclared = drop, uninstalled = warn + pass-through)'
  checked: true
  verify: 'Modify test-child to emit a fact with schema "bogus" (not in manifest). Confirm: fact logged as dropped (warning in Mother output), not in events.db. Valid facts from same run ARE present. Separately: remove installed schema file — facts pass through with warning, not dropped.'
- id: mother-run-test-child
  text: '`patina mother run test` spawns test-child (from pipe-native-transport examples/), routes facts to events.db — proves broker works before production connector'
  checked: true
  verify: '`patina mother run test` outputs fact count > 0. `SELECT event_type, count(*) FROM eventlog WHERE source_id = ''child:test-child'' GROUP BY event_type` returns rows.'
- id: mother-run-github
  text: '`patina mother run github` spawns github-connector, routes github.* facts to project events.db with content-hash dedup — tests routing, not child correctness (see [[spec-github-connector]] EC mother-run-works for child protocol verification). Verified after [[spec-github-connector]] is complete.'
  checked: false
  verify: Run `patina mother run github`. Output shows fact count, dedup count, cursor value. `SELECT event_type, count(*) FROM eventlog WHERE event_type LIKE 'github.%' GROUP BY event_type` returns rows. Run again immediately — dedup count should equal fact count (all duplicates).
- id: credentials-via-pipe
  text: Children receive credentials via Mother-proxied delivery (Tier 1 — transparent header injection for pipe/http; Tier 2 — raw token in pipe/initialize for children with auth.requires_in_process_token) — not via environment variables or files
  checked: true
  verify: 'Tier 1: `patina mother run github` — pipe/http audit log shows Bearer header injected for api.github.com. Confirm no GITHUB_TOKEN env var, no temp credential files. Tier 2: test-child with auth.requires_in_process_token=true — inspect pipe/initialize params showing auth.token present.'
- id: sandbox-enforcement
  text: Mother refuses to spawn a native child when the OS cannot enforce sandboxing (Landlock v4 unavailable on Linux, sandbox_init() failure on macOS) unless --no-sandbox is explicitly passed. Sandbox enforcement is a broker responsibility — children do not self-sandbox.
  checked: true
  verify: 'On Linux <6.7: `patina mother run test` fails with explicit Landlock error. `patina mother run test --no-sandbox` succeeds with warning. On macOS: invalid sandbox profile causes spawn refusal with Apple error message.'
- id: wasm-routing-resolved
  text: WASM fact routing unified through broker (forge facts go through broker validation + dedup like native children) OR explicitly decided that forge stays on legacy host_emit path with no new WASM children permitted to bypass broker — decision documented in DESIGN.md
  checked: true
  verify: 'If unified: forge facts in events.db have content_hash values, dedup works across WASM/native. If legacy: DESIGN.md §5 documents decision with rationale, code comment in host_emit marks path as frozen.'
- id: mother-status-works
  text: '`patina mother status` shows source state — last run timestamp (from broker_cursors.updated_at), fact count (from eventlog), and last error. Full health/lifecycle data requires daemon; standalone mode shows historical data only (see DESIGN.md Mother Status Enhancements).'
  checked: true
  verify: 'After `patina mother run test` and `patina mother run github`: `patina mother status` shows both sources with name, last run timestamp, fact count, and status (ok/error). Verify standalone mode works without daemon running (pulls from broker_cursors + eventlog queries).'
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
- Routing engine — read sources.toml, match source→destination,
  content-hash dedup (no shared-spawn fan-out optimization — see
  Design Constraints)
- Child lifecycle management — spawn native children in sandbox,
  wrap WASM children via BrokerChild trait (WASM routing stays on
  legacy host_emit path until EC `wasm-routing-resolved` closes)
- Schema validation — validate emitted facts against child manifest
  declarations (see DESIGN.md §6 decision table)
- Scheduling — manual (`patina mother run`) and on-scrape only.
  Hourly/daily/stream modes are [[spec-continuous-operation]] scope.
- `patina mother run/status/sources` CLI commands

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
  │   ├── Native: fork+exec in sandbox, stdio pipe protocol (BrokerChild)
  │   └── WASM: wrapped via BrokerChild (routing via legacy host_emit
  │         until wasm-routing-resolved EC closes — see DESIGN.md §5)
  ├── Routing engine
  │   ├── pipe/fact received → validate schema → route to destination
  │   └── content-hash dedup (one spawn per source, no shared-spawn)
  ├── Schema validator
  │   └── child manifest declares schemas → Mother checks each fact
  │       (undeclared = drop, uninstalled = warn + pass-through)
  └── Scheduler (v1)
      ├── manual: `patina mother run <name>`
      └── on-scrape: triggered by `patina scrape`
      (hourly/daily/stream: [[spec-continuous-operation]] scope)
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
3. Build BrokerChild trait with NativeChild adapter (WASM wrapped
   but routing stays on legacy path — see DESIGN.md §5)
4. Implement native child spawn: fork+exec in sandbox, connect stdio
5. Implement routing engine: receive pipe/fact, validate schema
   (§6 decision table), write to destination events.db with
   content-hash dedup
6. Add `patina mother run <name>` — manual trigger (spawn child,
   route facts, report results)
7. Add `patina mother status` — show children, last run, fact count,
   errors (daemon: in-memory state; standalone: broker_cursors +
   eventlog queries)
8. Wire on-scrape scheduling: `patina scrape` triggers on-scrape
   sources via `run_source()`
9. Verify: `patina mother run github` produces events in project
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
- `src/broker/mod.rs` — routing engine (run_source, status)
- `src/broker/lifecycle.rs` — BrokerChild trait (NativeChild + WASM stub)
- `src/broker/routing.rs` — fact validation + content-hash dedup
- `src/broker/cursor.rs` — transactional cursor management
- `src/broker/http.rs` — production pipe/http handler
- `src/broker/connection.rs` — connection config reader
- `src/broker/sources.rs` — sources.toml reader
- `src/broker/spawn.rs` — native child spawn with sandbox

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

- **Stream lifecycle** — always-on children with health monitoring,
  restart on crash. Poll mode only in v1. Stream children are
  [[spec-continuous-operation]] scope.
- **Hourly/daily scheduling** — the broker provides `run_source()`.
  Timer-driven invocation is [[spec-continuous-operation]] scope.
- **Fan-out optimization** — no shared-spawn (one child serving
  multiple destinations). One spawn per `run_source()` call.
  Content-hash dedup handles overlap. Optimize when measured.
- P2P sync between Mothers (future, needs persona-federation)
- Encryption at rest (future, needs persona keypair)
- Edge deployment (future, separate spec)
- Building connectors (that's per-connector specs like
  [[spec-github-connector]])
- OAuth/connection management (that's [[spec-patina-connect]])
- Daemon auto-start (that's [[spec-continuous-operation]])
