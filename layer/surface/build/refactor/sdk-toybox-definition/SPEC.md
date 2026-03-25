---
type: refactor
id: sdk-toybox-definition
status: draft
created: 2026-03-24
sessions:
  origin: 20260324-101606-299953000
related:
- layer/surface/build/refactor/sdk-contract-stabilization/SPEC.md
- layer/core/spec-driven-design.md
- layer/core/dependable-rust.md
- layer/core/safety-boundaries.md
exit_criteria:
- id: toy-definition-locked
  text: Formal toy definition and litmus test are documented in SDK-facing docs and design, with explicit properties and boundaries
  checked: true
- id: host-resources-enumerated
  text: Complete inventory maps every `wit/toys/*.wit` interface to host resource boundary, implementation status, and rationale
  checked: true
- id: toybox-consolidated
  text: Overlap clusters are resolved with explicit keep/merge/defer decisions and justification for each cluster
  checked: true
- id: toybox-locked
  text: Canonical toybox table is locked with direction, host resource, scope knobs, tier, and "why can't the child do this itself" per toy
  checked: false
- id: sdk-tiers-aligned
  text: SDK tier crates/docs align to the canonical toybox with no orphan toy abstractions
  checked: false
- id: migration-gates-defined
  text: Any toy merge/removal path is defined as rollback-safe migration slices with parity gates and explicit non-goals
  checked: false
---
# refactor: Define the toybox framework and enumerate Mother's canonical toy surface

> Establish the formal definition of toys as controlled WASM sandbox boundary openings, enumerate all host resources that require openings, consolidate overlapping toys, and lock the toybox as the finite platform-defined capability surface in the SDK.

## Problem

Patina has 22 WIT toy interface files but no formal framework for what a toy IS, why it exists, or when the set is complete. Toys were added organically as children needed them. Some toys overlap (4 toys hitting the network stack, 3 toys hitting the event system). The pipe abstraction was removed (March 21, commit fa0480db) but the toy vocabulary wasn't reconciled afterward.

Without a formal framework:
- Toy sprawl is inevitable — every new child need becomes a new toy
- Consolidation decisions have no principled basis
- Third-party SDK consumers can't reason about the platform surface
- Security audits can't enumerate the full attack surface

This violates **dependable-rust** (leaking implementation details into public API surface) and **patina-identity** (toys should serve the knowledge protocol, not be a generic capability grab bag).

## Goal

Lock the toybox. Define what a toy is, enumerate the canonical set, and close the surface.

Execution order for this lane is explicit:

1. Lock the canonical toybox contract in SDK/spec docs first.
2. Then clean up existing toys through rollback-safe keep/merge/defer migration slices.

## Status

Draft. A1 inventory, A2 definition lock, and A3 cluster decisions are complete and evidenced in DESIGN/SDK docs. Canonical toybox lock (A4), tier alignment (A4/A5), and migration gates (A5) remain pending.

## Non-Goals

- Implementing new toys — this spec defines the set, not the host-side handlers
- Removing existing toy functionality — consolidation changes WIT surface, not capability
- Domain-specific toys — the toybox is domain-agnostic by design
- Third-party toy extensions — Mother defines toys, period

## Current State

### 22 WIT files in wit/toys/

8 with direct child host adapter modules: connector, events, github, http, ingress, lake, query, session

14 scaffolded only: belief, checkpoint, emit, git, graph, layer-fs, layer, log, measure, peer, schema, state, task, types

### Overlap observed

| Host Resource | Current Toys | Overlap |
|---|---|---|
| Network stack | http, github, connector, ingress | 4 toys, 1 resource |
| Event system | events, emit, measure | 3 toys, 1 resource |
| Layer filesystem | layer, layer-fs | 2 toys, 1 resource |
| Knowledge structures | belief, graph, query | 3 toys potentially related |
| Persistence | lake, state, checkpoint, schema | 4 toys into storage |

### SDK tier allocation exists

- Core: log, state, layer-fs, git, peer
- Data: lake, checkpoint, measure, github, connector
- Agent: query, emit, session

## Target State

A **locked toybox** where:
1. Every toy has a formal justification ("why can't the child do this itself?")
2. Each toy maps to exactly one host resource boundary
3. Scopes configure how the opening is shaped (domains, secrets, resource names)
4. Domain-specific logic lives in children, not toys
5. The SDK documents the complete toybox as the platform contract
6. Adding a new toy requires the same rigor as adding a syscall to a kernel
7. Any merge/removal path is staged with parity + rollback gates and does not silently break child manifests

## Solution

### Phase 1: Lock the definition

A toy is a **controlled opening in the WASM sandbox wall** that Mother defines, owns, and grants to children at init time. It exists because the child cannot provide the capability itself from inside the WASM sandbox.

Properties:
- **Mother-defined** — children cannot create new openings
- **Granted at init** — no runtime privilege escalation
- **Scoped** — pinned to specific resources via `[needs.scopes]`
- **Domain-agnostic** — same toys for any knowledge domain
- **Credential-injected** — secrets flow through scopes on the host side, never cross the boundary
- **Compile-time enforced** — WIT interface IS the capability
- **Indivisible** — related concerns bundle together (per connector-toy-is-indivisible-authority belief)

The litmus test: **"Why can't the child do this itself?"** If the answer is "it can, with pure WASM compute" — it's a library, not a toy.

### Phase 2: Enumerate host resources

List every resource on Mother's side of the wall that a child might need controlled access to. For each resource, determine how many openings are needed and why.

Enumeration is deterministic and must include:
- every `wit/toys/*.wit` file,
- its current host adapter implementation status,
- its owning SDK lane/tier,
- and whether it is canonical, shim, or candidate for merge/defer.

Known host resources:
- Logging/telemetry pipeline
- Child-local persistent store
- Stream progress/cursor store
- Data warehouse (DuckDB)
- Event bus (ordered streams)
- Network stack (HTTP + credential injection)
- Git binary + repository
- Layer filesystem (layer/)
- Session state + artifacts
- Epistemic layer (beliefs)
- Knowledge graph
- Search indexes (scry/assay/context)
- Inter-child event routing
- Task scheduler
- Structured trace buffer (future — trace-as-witness)

### Phase 3: Consolidate

For each host resource, determine if multiple current toys are really the same opening with different shapes. Apply the principle: one host resource, one toy, scopes for configuration.

Consolidation outputs must be explicit per cluster:
- `keep-separate` (with rationale),
- `merge` (with migration sequence), or
- `defer` (with blocker and revisit trigger).

Key consolidation questions:
- Is `github` a toy or a child using `http`?
- Are `emit` and `measure` separate openings or shapes of `events`?
- Are `layer` and `layer-fs` the same opening?
- Is `schema` part of `lake`?
- Is `ingress` part of `events`?
- Is `connector` a toy or a child pattern using `http` + `lake` + `checkpoint`?

Each consolidation must be justified — some apparent overlaps may be legitimately separate openings.

### Phase 4: Lock and document

Write the canonical toybox into the SDK. Each toy entry includes:
- Name
- Direction (in, out, or both)
- Host resource it opens
- Why the child can't do this itself
- SDK tier (core/data/agent)
- Scope parameters
- Stability class (stable/experimental/shim)
- Implementation status (implemented/scaffolded)

## Implementation Order

1. A1: Inventory pass — build toy/resource/status matrix from `wit/toys`, host adapter modules, and SDK exports.
2. A2: Definition lock — publish formal toy definition + litmus test + anti-goals in SDK docs.
3. A3: Cluster decisions — resolve overlap clusters with keep/merge/defer decisions and rationale.
4. A4: Canonical toybox lock — publish canonical table in SDK docs/design with scope knobs and tiers.
5. A5: Migration gates — define rollback-safe merge/removal slices; no behavior changes without parity gates.
6. A6: Cleanup execution — apply approved toy cleanups only after A1-A5 are complete and verified.

## Resolved Decisions

- **Toys are Mother-defined, not child-requested.** Mother owns the toybox. (Session 20260324)
- **Domain specificity lives in children, not toys.** GitHub, Google Workspace, SEC filings all use the same toys. (Session 20260324)
- **Pipes are gone.** The pipe abstraction (removed fa0480db) was replaced by the toy/child model. Toys are the boundary openings; children are the code that flows data through them. (Session 20260324)
- **Secrets are scopes on toys, not a separate toy.** Credentials are injected by Mother on the host side — they never cross the boundary. (Session 20260324)
- **The toybox is finite.** Adding a toy has the same weight as adding a syscall. Requires formal justification. (Session 20260324)

## Verification

- `patina spec check sdk-toybox-definition --json`
- Inventory parity:
  - every `wit/toys/*.wit` appears exactly once in canonical toybox table
  - every canonical toy entry has resource + justification + tier + status fields
- SDK compatibility safety gates remain green before any merge/removal execution:
  - `cargo check -q -p patina-sdk --features knowledge-child,toy-log,toy-state,toy-session,toy-lake,toy-checkpoint,toy-query,toy-emit,toy-measure,toy-github,toy-connector,toy-peer,toy-events,toy-belief,toy-task`
  - `cargo check -q -p patina-sdk --features pipeline`
  - `cargo check -q -p patina-sdk --features task`
  - `cargo check -q -p patina-sdk --features command`
  - `cargo check -q -p patina-sdk --features mother-child`
  - `cargo test -q -p patina-ai scaffold::tests::test_scaffold`
- Before any toy merge/removal lands:
  - first-party child manifests parse/validate,
  - command/runtime parity checks for impacted toys are green,
  - rollback steps are documented in DESIGN.

## Exit Criteria

See frontmatter.

## Build Readiness

Ready for active execution once promoted. No merge/removal code changes should start until A1 inventory and A3 cluster decisions are documented in DESIGN.
