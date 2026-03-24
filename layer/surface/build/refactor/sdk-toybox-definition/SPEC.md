---
type: refactor
id: sdk-toybox-definition
status: draft
created: 2026-03-24
sessions:
  origin: 20260324-101606-299953000
related:
- layer/surface/build/refactor/sdk-contract-stabilization/SPEC.md
exit_criteria:
- id: toy-definition-locked
  text: Formal toy definition documented in SDK — a toy is a controlled opening in the WASM sandbox wall that Mother defines and grants
  checked: false
- id: host-resources-enumerated
  text: Every host resource that requires a boundary opening is enumerated with justification
  checked: false
- id: toybox-consolidated
  text: Overlapping toys consolidated — each opening maps to one host resource
  checked: false
- id: toybox-locked
  text: Canonical toybox locked in SDK with explicit "why can't the child do this itself" for each toy
  checked: false
- id: sdk-tiers-aligned
  text: SDK tier crates (core/data/agent) updated to reflect consolidated toybox
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

## Status

Draft. Framework established through session 20260324-101606-299953000. Enumeration not yet performed.

## Non-Goals

- Implementing new toys — this spec defines the set, not the host-side handlers
- Removing existing toy functionality — consolidation changes WIT surface, not capability
- Domain-specific toys — the toybox is domain-agnostic by design
- Third-party toy extensions — Mother defines toys, period

## Current State

### 22 WIT files in wit/toys/

8 with host implementations: connector, events, github, http, ingress, lake, query, session

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

## Implementation Order

1. Phase 1 — Write the formal toy definition into SDK docs
2. Phase 2 — Enumerate host resources (audit Mother's capabilities)
3. Phase 3 — Consolidation proposals (each as a scalpel decision)
4. Phase 4 — Update WIT files, SDK tier crates, and lock

## Resolved Decisions

- **Toys are Mother-defined, not child-requested.** Mother owns the toybox. (Session 20260324)
- **Domain specificity lives in children, not toys.** GitHub, Google Workspace, SEC filings all use the same toys. (Session 20260324)
- **Pipes are gone.** The pipe abstraction (removed fa0480db) was replaced by the toy/child model. Toys are the boundary openings; children are the code that flows data through them. (Session 20260324)
- **Secrets are scopes on toys, not a separate toy.** Credentials are injected by Mother on the host side — they never cross the boundary. (Session 20260324)
- **The toybox is finite.** Adding a toy has the same weight as adding a syscall. Requires formal justification. (Session 20260324)

## Verification

- Every WIT file in wit/toys/ maps to exactly one entry in the canonical toybox
- Every toybox entry has a "why can't the child do this itself" justification
- No two toys open the same host resource without explicit rationale
- SDK tier crates match the consolidated toybox
- DuckLake child.toml still compiles and functions after consolidation
- All first-party children's toy declarations remain valid

## Exit Criteria

See frontmatter.

## Build Readiness

Framework established. Definition locked in conversation. Enumeration requires systematic audit of Mother's host resources — needs a focused session with codebase read-through.
