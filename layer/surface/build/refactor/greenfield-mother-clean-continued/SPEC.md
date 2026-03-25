---
type: refactor
id: greenfield-mother-clean-continued
status: draft
created: 2026-03-25
beliefs:
  - "[[four-roles-no-overlap]]"
  - "[[children-have-agency-toys-are-capabilities]]"
  - "[[core-verbs-standalone-mother-additive]]"
  - "[[mother-is-connection-and-continuity]]"
  - "[[mother-is-the-daemon]]"
  - "[[agents-are-guests-mother-is-infrastructure]]"
  - "[[initialize-is-capability-grant]]"
sessions:
  origin: 20260325-064204-876122000
  prior_art: greenfield-mother-patina-rebuild (v0.43.11, 2026-03-24)
exit_criteria:
  - id: GFC1
    text: MotherChild trait deleted from codebase — zero hits for `pub trait MotherChild`
    checked: false
  - id: GFC2
    text: StaticChild type deleted — zero hits for `StaticChild`
    checked: false
  - id: GFC3
    text: ChildRegistry holds single `children` vector (KnowledgeChild only), `legacy_children` gone
    checked: false
  - id: GFC4
    text: SecretsCacheChild (`mother/src/secrets.rs`) deleted — logic lives in `services/secrets.rs`
    checked: false
  - id: GFC5
    text: SessionWriterChild (`mother/src/session_writer.rs`) deleted
    checked: false
  - id: GFC6
    text: Legacy heartbeat branch removed — no `legacy_migration` in daemon_heartbeat.rs
    checked: false
  - id: GFC7
    text: daemon.rs protocol v1 handler deleted — Mother speaks HTTP only
    checked: false
  - id: GFC8
    text: MotherServices struct owns secrets, sessions, health — HTTP routes call it directly
    checked: false
  - id: GFC9
    text: builtin_dispatch.rs eliminated — logic absorbed into service-backed routes
    checked: false
  - id: GFC10
    text: "`cargo build` succeeds, `cargo test` passes"
    checked: false
  - id: GFC11
    text: "`patina mother start` launches, `curl -s --unix-socket ~/.patina/run/serve.sock http://localhost/health` returns JSON with `status` and service/child health keys"
    checked: false
  - id: GFC12
    text: "`rg 'MotherChild|StaticChild|legacy_migration|legacy_children' --type rust` returns zero hits"
    checked: false
---
# refactor: Greenfield Mother — separate internal services from child registry

> Mother's code conflates internal services (secrets, sessions, health, specs, lakes) with the child abstraction. Native Rust structs masquerade as children via MotherChild trait and StaticChild markers, while the actual child registry should hold only WASM guests. This refactor draws a clean line: Mother owns internal services directly, ChildRegistry holds only KnowledgeChild WASM instances.

## Problem

Mother's four-role belief (`[[four-roles-no-overlap]]`) says:

- **Patina** = belief layer (knowledge protocol)
- **Mother** = infrastructure (daemon, sandbox, toys, grants)
- **Children** = knowledge workers (WASM, sandboxed, use SDK)
- **Projects** = development zone

But the code doesn't honor this boundary. The `ChildRegistry` in `mother/src/registry.rs` holds two vectors:
- `knowledge_children: Vec<KnowledgeChild>` — intended for WASM guests
- `legacy_children: Vec<MotherChild>` — native Rust structs pretending to be children

Six "children" are registered at daemon startup that are not children at all:
1. `secrets` (SecretsCacheChild) — in-memory TTL cache, native Rust
2. `session-writer` — placeholder that returns "healthy" and nothing else
3. `spec-manager` — StaticChild marker, no-op
4. `doctor` — StaticChild marker, no-op
5. `lake-manager` — StaticChild marker, no-op
6. `secrets-authority` — StaticChild marker, no-op

Meanwhile, `builtin_dispatch.rs` in the CLI doesn't even use the registry for these — it pattern-matches on child name and routes directly to CLI code. They are **already internal services pretending to be children**.

Additionally, `daemon.rs` implements a pre-v1 Unix socket protocol that is mostly stubbed ("not yet implemented") while `http_daemon.rs` is the actual working transport. Two parallel transports exist for no reason.

The heartbeat thread has a `--legacy-migration` branch that spawns `Toy` structs as shell subprocesses — a model superseded by the WASM capability grant design.

## Goal

Draw a clean, permanent line between Mother's internal services and the child registry. After this refactor:
- Mother owns and calls her internal services directly
- ChildRegistry is exclusively for WASM KnowledgeChild guests
- One transport (HTTP over UDS/TCP)
- One heartbeat path (knowledge child cycles only)
- The codebase reflects the four-role architecture from our beliefs

## Core Value Anchors

This spec is anchored to `layer/core` values and must be executed in ways that preserve them:

- `spec-driven-design`: no non-trivial scope outside this spec's gates.
- `dependable-rust`: promote stable service interfaces, keep churn in internals.
- `unix-philosophy`: one responsibility per subsystem (services vs registry vs transport).
- `patina-identity`: Mother is infrastructure; children are knowledge workers.
- `safety-boundaries`: no surprising side effects while deleting legacy paths.
- `session-capture`: record gate proofs and decisions as session updates/notes.

## Status

Draft. Architecture alignment confirmed in session 20260325-064204-876122000. This continues the greenfield Mother work from `greenfield-mother-patina-rebuild` (completed 2026-03-24, released as v0.43.11) and the toybox framework design from session 20260324-101606-299953000.

## Non-Goals

- Wiring actual WASM child loading (unblocked by this work but not part of it)
- Changing the session artifact format or `patina ai` entry point (these already work)
- Redesigning the secrets authority backend (solid, just needs to stop being a "child")
- Implementing the stubbed daemon actions (scry, context, measure, spec, lake)
- Multi-Mother federation
- Token management or context window aware sessions

## Execution Contract

This spec is execution-constrained. Any agent implementing it must follow these rules:

1. **No silent scope changes** — if a gate reveals unexpected entanglement, pause and update the spec before proceeding.
2. **No deferral language** — every commit either does the thing or explicitly explains why not.
3. **Claim discipline** — every state claim backed by `file:line` or command output.
4. **One-gate-at-a-time** — do not start GFC-G2 until GFC-G1 exit proofs pass.
5. **Cargo check between gates** — `cargo build` must succeed after every gate.
6. **Scalpel over shotgun** — change only gate-targeted files; avoid opportunistic rewrites.
7. **Read before write/remove** — inspect current code paths and callsites before edits/deletions.

## Phase Gate Policy

Each gate must have:
- Entry conditions (what must be true before starting)
- Implementation commits (scalpel, not shotgun)
- Exit proofs (commands + expected output)
- GFC truth-map updates

If proofs fail, gate remains open. Do not start next gate.

## Current State

### Mother crate (`mother/src/`)
- `runtime.rs` defines two traits: `KnowledgeChild` (drain/tick/handle) and `MotherChild` (tick→Toys)
- `registry.rs` holds both in parallel vectors with dual-path orchestration
- `secrets.rs` implements `MotherChild` for in-memory cache
- `session_writer.rs` implements `MotherChild` as a no-op placeholder
- `static_child.rs` creates named healthy markers
- `daemon_bootstrap.rs` registers all six fake children before accepting connections
- `daemon_heartbeat.rs` branches on `legacy_migration` flag
- `daemon.rs` implements a second transport (Unix socket, line-based protocol) that is mostly stubbed
- `http_daemon.rs` + `http_routes.rs` + `http_api.rs` implement the actual working HTTP transport

### CLI integration (`src/commands/mother/`)
- `builtin_dispatch.rs` routes "child" requests by pattern-matching names, bypassing the registry entirely
- `daemon.rs` wires up `ServerState` with registry + scry backend
- Internal HTTP client talks to the HTTP transport

### What works today
- `patina ai <interface>` entry point (claude, opencode, gemini)
- Session lifecycle (start/update/note/end) via wrapper scripts
- Interface bundle deployment with tarball-style reconciliation
- Secrets authority backend (vault, keychain, encryption)
- HTTP daemon (UDS + TCP) with health, scry, secrets, child dispatch routes
- Broker source configuration and cursor management
- Session dual-bookkeeping (artifacts in layer/ + state in mother_sessions table)

## Target State

```
Mother (daemon process)
├── MotherServices (internal, native Rust)
│   ├── SecretsService      — wraps secrets_authority_backend (cache + vault)
│   ├── SessionStateService — wraps mother_sessions table operations
│   ├── HealthService       — aggregates Mother health + child health from registry
│   ├── BrokerService       — wraps broker/ (source routing, cursors)
│   ├── LakeService         — wraps lake dispatch
│   └── SpecService         — wraps spec dispatch
│
├── HTTP Transport (single, http_daemon.rs)
│   ├── /health             → HealthService
│   ├── /secrets/*          → SecretsService
│   ├── /api/scry           → ScryBackend
│   ├── /api/spec/*         → SpecService
│   ├── /api/lake/*         → LakeService
│   └── /child/<name>/*     → ChildRegistry (WASM only)
│
└── ChildRegistry (WASM guests only)
    ├── KnowledgeChild instances loaded from ~/.patina/children/
    └── Heartbeat: drain → tick → handle (no legacy branch)
```

## Solution — Milestone Gates

### GFC-G1: Introduce MotherServices with SecretsService
- Create `mother/src/services/mod.rs` with `MotherServices` struct
- Create `mother/src/services/secrets.rs` — migrate TTL cache logic from `SecretsCacheChild`
- Wire secrets HTTP routes to call `SecretsService` directly
- **Exit proof**: `cargo build` succeeds, secrets routes compile against new service

### GFC-G2: Add SessionStateService and HealthService
- Create `mother/src/services/sessions.rs` — wraps `mother_sessions` table ops from `state.rs`
- Create `mother/src/services/health.rs` — aggregates Mother service health + child registry health
- **Entry**: GFC-G1 exit proofs pass
- **Exit proof**: `cargo build` succeeds, health and session types resolve

### GFC-G3: Wire HTTP routes to MotherServices
- Update `http_routes.rs` and `http_api.rs` to route `/health`, `/secrets/*` through `MotherServices`
- Absorb `builtin_dispatch.rs` pattern-matching logic into service-backed routes
- Update `ServerState` to hold `MotherServices` + `ChildRegistry`
- **Entry**: GFC-G2 exit proofs pass
- **Exit proof**: `cargo build` succeeds, HTTP routes compile against services, `builtin_dispatch.rs` logic migrated, and route parity checks pass for `/health`, `/child/spec-manager/*`, `/child/lake-manager/*`, and `/child/doctor/*`

### GFC-G4: Delete legacy child types
- Delete `MotherChild` trait from `runtime.rs`
- Delete `secrets.rs` (SecretsCacheChild)
- Delete `session_writer.rs` (SessionWriterChild)
- Delete `static_child.rs` (StaticChild)
- Delete `Toy` struct from `runtime.rs` (keep `GrantedToys` in `toys.rs` if referenced by KnowledgeChild)
- **Entry**: GFC-G3 exit proofs pass (nothing references deleted types through old paths)
- **Exit proof**: `cargo build` succeeds, `rg "MotherChild|StaticChild|SecretsCacheChild|SessionWriterChild" --type rust` returns zero

### GFC-G4a: Retire mother-child runtime lane

- Retire the last known mother-child plugin lane (`patina-models` legacy artifact path)
- Update daemon loader to reject manifests with `kind = "mother-child"` with explicit migration error
- Keep this as a compatibility stop-gap until G4b removes `MotherChild` trait/types entirely
- **Entry**: GFC-G3 exit proofs pass
- **Exit proof**: `cargo build` succeeds, and daemon loader no longer instantiates `MotherChildEngine` for any `mother-child` manifest

### GFC-G4b: Delete legacy child types

- Execute the original GFC-G4 deletion package after G4a lane retirement is in place
- **Entry**: GFC-G4a exit proofs pass
- **Exit proof**: `cargo build` succeeds, `rg "MotherChild|StaticChild|SecretsCacheChild|SessionWriterChild" --type rust` returns zero

### GFC-G5: Purge ChildRegistry legacy path
- Remove `legacy_children` vector from `ChildRegistry`
- Remove `register_legacy()`, `tick_legacy_all()`
- Simplify to single `children: Vec<KnowledgeChild>`
- Remove `--legacy-migration` flag from `DaemonBootstrapConfig`
- Remove legacy branch from `daemon_heartbeat.rs`
- **Entry**: GFC-G4 exit proofs pass
- **Exit proof**: `cargo build` succeeds, `rg "legacy_children|legacy_migration|tick_legacy" --type rust` returns zero, and daemon smoke (`patina mother start` + `/health`) still passes

### GFC-G6: Delete daemon.rs protocol v1
- Delete `mother/src/daemon.rs` (old Unix socket line-based protocol handler)
- Remove from `lib.rs` exports
- Mother speaks HTTP only (over UDS or TCP)
- **Entry**: GFC-G5 exit proofs pass
- **Exit proof**: `cargo build` succeeds, no `DaemonState` references remain

### GFC-G7: Update bootstrap and verify
- Update `daemon_bootstrap.rs` — create `MotherServices` instead of calling `register_builtin_children()`
- Clean up `lib.rs` module exports (add `services`, remove dead modules)
- Delete `builtin_dispatch.rs` from CLI
- Run full test suite, fix any broken tests
- **Entry**: GFC-G6 exit proofs pass
- **Exit proof**: `cargo build && cargo test` pass, `patina mother start` launches, `/health` returns OK

## Route Parity Checklist (GFC-G3)

Before deleting legacy surfaces, prove HTTP parity for built-in child routes:

- `/health` returns JSON object with `status` and health details
- `/child/spec-manager/dispatch` accepts spec payload and returns JSON object (success/error shape preserved)
- `/child/lake-manager/dispatch` accepts lake payload and returns JSON object (success/error shape preserved)
- `/child/doctor/run` returns JSON object containing doctor run result fields

Parity means response shape and status code class remain equivalent for representative valid and invalid payloads.

## Edge Notes (validated)

- Auth checks are enforced in HTTP routing layer (`mother/src/http_routes.rs`), not in CLI builtin dispatch glue.
- `legacy_migration` wiring exists in Mother/CLI runtime code paths; removal must include bootstrap, heartbeat, and launch option plumbing.

### GFC truth map

Status keys:
- `unverified` — not yet checked
- `verified-false` — code disproves criterion today
- `verified-true` — criterion satisfied with evidence

| GFC | Status | Evidence |
|-----|--------|----------|
| GFC1 | unverified | `MotherChild` trait exists in `mother/src/runtime.rs` (pending G4b) |
| GFC2 | verified-true | `mother/src/static_child.rs` deleted and no `StaticChild` hits remain in Mother/daemon sources |
| GFC3 | unverified | `legacy_children` vector exists in `mother/src/registry.rs` |
| GFC4 | verified-true | `mother/src/secrets.rs` deleted; secrets cache logic is now in `mother/src/services/secrets.rs` |
| GFC5 | verified-true | `mother/src/session_writer.rs` deleted and no `SessionWriterChild` hits remain in Mother/daemon sources |
| GFC6 | unverified | `legacy_migration` flag exists in `daemon_heartbeat.rs` and `daemon_bootstrap_config.rs` |
| GFC7 | unverified | `daemon.rs` protocol v1 handler exists in `mother/src/daemon.rs` |
| GFC8 | verified-true | `MotherServices` now exists in `mother/src/services/mod.rs` and daemon runtime routes `/secrets/*` and `/health` through service-backed methods |
| GFC9 | verified-false | `builtin_dispatch.rs` file still exists, though built-in route logic is now handled in `mother/src/http_api.rs` + `src/commands/mother/daemon.rs` |
| GFC10 | unverified | Build/test not yet run against target state |
| GFC11 | unverified | Daemon not yet tested against target state |
| GFC12 | unverified | Grep not yet run against target state |

## Resolved Decisions

1. **Mother has internal services + external children** (not "everything is a child"). Confirmed in session 20260325-064204-876122000. Aligns with `[[four-roles-no-overlap]]`, `[[children-have-agency-toys-are-capabilities]]`, and `[[core-verbs-standalone-mother-additive]]`.
2. **`KnowledgeChild` lifecycle stays** (drain → tick → handle). This is the right model for WASM guests. Only `MotherChild` (tick → Toys) goes away.
3. **HTTP is the single transport**. The old `daemon.rs` line-based protocol is dead code.
4. **Session dual-bookkeeping is correct** — artifacts in `layer/sessions/` (Patina knowledge) + state in `mother_sessions` (Mother state machine). The session-writer "child" in the middle adds nothing and is removed.
5. **`GrantedToys` survives if referenced** — the `Toy` struct (shell command) dies with `MotherChild`, but `GrantedToys` (WASM capability declarations) may still be needed by `KnowledgeChild` init. Evaluated during GFC-G4.

## Build Readiness

Beliefs aligned. Architecture agreed. Code surfaces mapped. Prior art: `greenfield-mother-patina-rebuild` completed in a single active session (Mar 24) with 12 gates. This spec has 7 gates, smaller scope — primarily deletion and promotion. Same-session executable.
