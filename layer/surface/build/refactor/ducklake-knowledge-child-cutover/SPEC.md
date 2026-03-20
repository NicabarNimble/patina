---
type: refactor
id: ducklake-knowledge-child-cutover
status: complete
created: 2026-03-12
updated: 2026-03-13
sessions:
  origin: 20260312-140904
blocked_by: []
related:
- src/plugin/internal/knowledge_child.rs
- src/toys/lake.rs
- src/broker/mod.rs
- children/ducklake/src/lib.rs
- children/ducklake/plugin.toml
- crates/patina-child-sdk/src/lib.rs
- wit/knowledge-child/deps/patina-host/host.wit
- src/connect/internal/resolve.rs
- src/connect/internal/store.rs
- src/plugin/internal/mod.rs
- src/commands/mother/mod.rs
- src/commands/mother/daemon.rs
- src/commands/connect.rs
exit_criteria:
- id: ducklake-has-single-knowledge-child-identity
  text: DuckLake runs through one authoritative knowledge-child identity (`children/ducklake`) and the legacy native runtime path is removed from runtime + workspace membership
  checked: true
- id: broker-to-knowledge-child-invocation-model-is-explicit
  text: The cutover defines and implements one explicit invocation model for `Destination::Lake` (enqueue + bounded wait via Mother runtime), with no ambiguous dual execution semantics
  checked: true
- id: destination-lake-routes-through-knowledge-child
  text: '`Destination::Lake` cutover routes through the knowledge-child orchestration path, not direct native ducklake spawn in broker'
  checked: true
- id: lake-toy-host-parity-reaches-real-ducklake-semantics
  text: Mother lake host behind granted `lake` toy supports required DuckLake semantics (table lifecycle, append/query behavior, cursor semantics) proven by parity integration tests
  checked: true
- id: connector-or-ingress-capability-supports-repo-scoped-issues-and-prs
  text: Granted child capability model supports repo-scoped GitHub issues and pull requests for any user-selected repository with policy-scoped grants
  checked: true
- id: oauth-vault-auth-is-authoritative-in-knowledge-child-path
  text: OAuth credentials from Patina vault are used through host-granted credential injection in the new DuckLake path (no ambient token assumptions)
  checked: true
- id: source-model-supports-multi-repo-and-incremental-sync
  text: DuckLake source model supports multiple repos, per-type sync (issues/prs), and cursor-driven incremental ingestion with durable checkpoints
  checked: true
- id: migration-preserves-existing-native-cursor-and-checkpoint-continuity
  text: Cutover includes explicit migration/compat behavior for legacy native cursor/checkpoint state with rollback-safe handling and tests proving no silent re-ingest or continuity loss
  checked: true
- id: repo-binding-control-plane-is-implemented
  text: A concrete command/API surface provisions repo bindings, grant-scoped connector capability, and source records for the knowledge-child path
  checked: true
- id: wasm-ducklake-and-old-native-path-pass-parity-suite-before-removal
  text: New DuckLake path matches old path outputs + failure handling on a defined parity suite, and native path removal happens only after that suite passes in CI
  checked: true
- id: sdk-and-wit-contract-minimum-is-stable-for-cutover
  text: WIT + child SDK + toy SDK expose stable typed contracts and one reference flow sufficient to build DuckLake-style apps without internal runtime context
  checked: true
---
# refactor: DuckLake Knowledge-Child Cutover

> Finish DuckLake as the authoritative knowledge-child path with parity for GitHub issues/PR ingestion and OAuth vault auth.

## Historical Note

During execution, the knowledge-child path used `children/ducklake-wasm` and
was later renamed to `children/ducklake` by
`ducklake-child-path-name`. This spec is complete; references below describe the
original cutover framing.

## Problem

DuckLake originally existed in two runtime realities:

- `children/ducklake-wasm` was the doctrine-aligned knowledge-child app shape
  at the time.
- `children/ducklake` carried the richer native ingestion path used by broker
  lake routing.

This split creates architectural debt and user confusion:

- The intended Mother/Child/Toy doctrine says children should be app brains
  using granted capabilities.
- The operational path still depends on the legacy native child runtime for
  key behavior.
- Third-party builders cannot rely on a single canonical DuckLake app model.

If we keep both paths, we keep duplicate identity, duplicate behavior, and
ongoing drift.

## Goal

Make DuckLake fully authoritative on the knowledge-child path.

**Directional doctrine to preserve:**

- Mother owns authority, policy enforcement, identity boundaries, and runtime
  continuity.
- DuckLake child owns ingestion orchestration and workflow policy.
- Toys are granted capability surfaces (lake, connector/ingress, state,
  checkpoint, measure), not ambient host bags.

**Product goal:**

With proper permissions, a user can attach any repository to DuckLake and ingest
issues + pull requests through OAuth credentials stored in vault, using only the
knowledge-child path.

## Status

Partially complete foundation, incomplete cutover (blocked tail):

- Complete: doctrine cleanup, knowledge-child runtime defaults, grant-shaped
  ingress in WIT/runtime/SDK, proof-child alignment.
- Incomplete: full DuckLake parity in host capability layer, broker cutover,
  migration continuity hardening, parity-suite proof, and old native child
  removal.

This spec is blocked behind `ducklake-native-removal-and-verification`, which
is the execution-first slice for native removal safety and wasm-path validation.

## Cutover Tail Contract

The following work is required before this spec can return to `complete`:

1. **Single identity enforcement**
   - Remove legacy native runtime path from workspace/runtime members.
   - Keep only the knowledge-child runtime identity (now `children/ducklake`).

2. **Lake host parity proof**
   - Add integration tests that compare legacy/native expectations vs knowledge-child
     host behavior for table creation, append semantics, query shape, and cursor behavior.
   - Include at least one failure-path assertion (invalid table/data contract).

3. **Migration continuity proof**
   - Add tests that seed legacy cursor/checkpoint state and verify migration to
     knowledge-child state without duplicate ingest on next run.
   - Include partial-failure behavior expectations (idempotent retry / no destructive loss).

4. **Parity gate before native removal**
   - Define a named parity suite command in this repo and run it in CI.
   - Native path removal is allowed only after parity suite is green.

## Non-Goals

- Do not redesign unrelated Mother architecture (persona federation,
  multi-Mother replication, edge deployment) inside this spec.
- Do not add ambient `host_http` back as a shortcut.
- Do not collapse TaskIntent substrate layering into ordinary toy semantics.
- Do not remove the old native path until parity suite passes.
- Do not require app builders to read internal broker code to build apps.

## Current State

- New DuckLake knowledge-child (`children/ducklake`) supports configured sources,
  lake writes, ingress fetch, and checkpoints.
- Current ingress grant is narrow and static (example endpoint) and not a full
  repo-scoped connector model.
- Broker `Destination::Lake` path still uses native child spawn flow.
- Native child path has richer integration semantics that are not yet fully
  represented by current knowledge-child host capabilities.
- OAuth and credential mapping infrastructure exists in host grant/injection
  flow, but the new DuckLake path must be proven end-to-end as authoritative.

## Target State

- DuckLake has one runtime identity: knowledge-child app in
  `children/ducklake`.
- User-selected repository bindings create grant-scoped DuckLake source entries
  and schedule sync through knowledge-child tasks.
- Issues + pull requests ingest through granted connector/ingress capability
  with OAuth credentials from vault injected by Mother policy, not by child
  ambient assumptions.
- Lake writes run through granted lake capability with real DuckLake parity.
- Broker routes lake destination work through the same knowledge-child path.
- Third-party builders can author similar apps with typed WIT/SDK contracts
  without internal context.

## Solution

### 1. Define one canonical DuckLake runtime identity

- Keep `children/ducklake` as authoritative DuckLake app identity.
- Treat native DuckLake runtime as migration reference only until parity is met.
- Remove parallel identity after cutover.

### 2. Lift missing semantics into granted host capabilities

- Expand `lake` host capability shape to support required DuckLake semantics for
  current ingestion parity.
- Expand `ingress` into repo-scoped connector-grade capability (or add a new
  connector interface) for issues + PR ingestion.
- Keep authority bounded by grants (repo, domain, persona/project scope).

### 3. Make OAuth vault auth authoritative in the new path

- Use host credential mappings and vault-backed secret resolution for repo sync
  requests in knowledge-child flow.
- Ensure all fetches needing auth go through host policy injection.

### 4. Bring source model to parity

- Support source records that capture repository, enabled data types
  (issues/prs), and sync policy.
- Preserve incremental cursor/checkpoint behavior per source and type.
- Support multiple repository sources under one DuckLake app identity.

### 5. Cut over broker lake routing

- Freeze invocation model to `enqueue + bounded wait` through Mother runtime:
  broker creates/updates source binding, enqueues sync intent, and waits on
  bounded completion signal from knowledge-child state/checkpoint status.
- Route `Destination::Lake` through that knowledge-child orchestration path.
- Keep old path only as temporary fallback while parity tests run.
- Remove old native path after cutover verification.

### 6. Add binding-management control plane

- Implement command/API surface for repo binding lifecycle:
  create/update/list/remove bindings under persona/project scope.
- Provision grant-scoped connector capability and source records from that
  surface.
- Ensure binding operations are idempotent and auditable.

### 7. Preserve continuity through migration

- Add explicit migration/compat step for existing native cursor/checkpoint
  state.
- Define key mapping and idempotent migration behavior.
- Provide rollback-safe behavior if migration partially fails.

### 8. Ship cutover-minimum builder contracts

- Keep WIT + SDK typed enough that app builders can compose toys with no
  internal runtime assumptions.
- Document expected request/response and error classes for connector/lake toys
  and include one end-to-end reference flow.

## Implementation Order

1. Define parity contract and test matrix against old path behavior.
2. Freeze invocation model for broker-to-knowledge-child execution.
3. Extend WIT interfaces for connector/lake parity semantics.
4. Implement host runtime support for new interfaces and vault-auth path.
5. Update `patina-toy-sdk` / `patina-child-sdk` to expose typed APIs.
6. Build repo-binding command/API surface and grant provisioning flow.
7. Upgrade DuckLake knowledge-child source model for repo-scoped issues/prs ingestion.
8. Add cursor/checkpoint migration/compat path from native state.
9. Cut broker `Destination::Lake` to knowledge-child route.
10. Run parity + failure-path + migration suite.
11. Remove native DuckLake runtime identity.

## Resolved Decisions

- DuckLake final shape is knowledge-child app + granted host capabilities, not
  fully ambient native child and not WASM-embedded storage authority.
- OAuth credentials are resolved and injected by host policy through vault,
  never directly managed by child runtime.
- Legacy path is retained only as migration oracle until parity passes.
- One DuckLake binary/artifact may back multiple scoped source bindings; grants
  provide authority boundaries.
- Broker invocation model is `enqueue + bounded wait` over Mother runtime state,
  not direct in-process child execution.
- Binding management is a first-class control-plane surface in Mother, not an
  ad-hoc child-only concern.

## Verification

- Contract tests: WIT/SDK/runtime agreement for lake + connector interfaces.
- Integration tests: configure repo source, ingest issues/prs, validate writes.
- Auth tests: OAuth credential present/absent/expired behavior through vault
  injection path.
- Incremental tests: repeated sync uses cursors/checkpoints correctly.
- Failure tests: partial type failure, transient HTTP errors, auth denial,
  retry/escalation expectations.
- Migration tests: existing native cursor/checkpoint state is honored/migrated
  without duplicate ingestion.
- Parity tests: new path output and operational behavior match legacy baseline.
- Builder minimum test: one reference app flow compiles and runs using typed
  toy APIs only (full ecosystem docs can follow in a separate spec).
- Tail gate verification to close this spec:
  - `patina spec check ducklake-knowledge-child-cutover --json` reports 11/11 checked
  - parity/migration test suite command(s) added and green in CI
  - no legacy native DuckLake runtime path remains present as active runtime path

## Exit Criteria

Use frontmatter exit_criteria as source of truth.

## Build Readiness

- [ ] Parity suite defined before destructive cutover.
- [ ] WIT changes and SDK changes reviewed together.
- [ ] Invocation model is implemented and covered by timeout/failure tests.
- [ ] Binding command/API surface is implemented and idempotent.
- [ ] Broker cutover guarded behind migration gate until tests pass.
- [ ] Cursor/checkpoint migration plan is implemented and verified.
- [ ] Native path removal planned as explicit final commit.
