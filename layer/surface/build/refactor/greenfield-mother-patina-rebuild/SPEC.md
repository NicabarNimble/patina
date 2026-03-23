---
type: refactor
id: greenfield-mother-patina-rebuild
status: draft
created: 2026-03-22
related:
  - layer/surface/build/refactor/patina-code-to-vision/SPEC.md
  - layer/surface/build/refactor/patina-code-to-vision/DESIGN.md
  - layer/core/spec-driven-design.md
  - layer/core/dependable-rust.md
  - layer/core/safety-boundaries.md
  - layer/core/unix-philosophy.md
beliefs:
  - core-verbs-standalone-mother-additive
  - agents-are-guests-mother-is-infrastructure
  - mother-is-the-daemon
  - core-primitives-are-not-children
exit_criteria:
  - id: GF1
    text: "A greenfield architecture narrative defines Patina core, Mother runtime, and child boundaries without legacy compatibility constraints"
    checked: false
  - id: GF2
    text: "A command-runtime policy matrix is defined for Mother-available and Mother-unavailable modes, including hard-fail surfaces"
    checked: false
  - id: GF3
    text: "Storage/event model is specified with clear ownership boundaries (events.db, projections, session artifacts, child state)"
    checked: false
  - id: GF4
    text: "Child lifecycle and toy grant model are specified as enforceable contracts (manifest schema, capability checks, failure behavior)"
    checked: false
  - id: GF5
    text: "Interface-runtime contract is explicit: Claude/OpenCode/Gemini are guests with runtime-specific session helpers and no hidden MCP assumptions"
    checked: false
  - id: GF6
    text: "Migration map from current codebase to greenfield target is documented as bounded slices with parity gates"
    checked: false
  - id: GF7
    text: "Risk register and verification plan exist for proving architectural parity before any destructive migration"
    checked: false
---
# refactor: Greenfield Mother + Patina Rebuild

> If we rebuilt Patina and Mother from scratch today, what architecture would we ship first, and why?

## Problem

The current refactor spec aligns code toward the vision, but it is still constrained by
existing module layout, transitional seams, and historical compatibility baggage.

Without a greenfield blueprint:

- we risk treating transitional seams as permanent by inertia,
- we blur "what is ideal" with "what was easiest to migrate",
- future contributors lack a first-principles target to evaluate new changes,
- architecture debates repeat without a canonical decision surface.

## Goal

Produce an authoritative greenfield architecture spec for Patina + Mother that is:

- first-principles and runtime-explicit,
- grounded in current beliefs and core values,
- explicit about boundaries, ownership, and failure modes,
- directly actionable as a migration target after current refactor completion.

## Non-Goals

- No immediate rewrite of current production code in this spec.
- No deletion of working compatibility seams solely because greenfield differs.
- No speculative protocol features without bounded verification plans.
- No runtime-specific lock-in to one interface provider.

## Scope

### In scope

- Core/Mother/child boundary model as if starting from empty repository.
- Runtime lifecycle model (boot, connect, discover, dispatch, observe).
- Data ownership model (event log, projections, session docs, child state).
- Capability and security model for toys and child grants.
- Interface guest contract for Claude/OpenCode/Gemini.
- Migration map from current architecture to target architecture.

### Out of scope

- Implementing all migrations in this spec.
- Re-litigating previously locked beliefs without contradictory evidence.

## Greenfield Questions This Spec Must Answer

1. What code lives in `patina` core vs `mother` crate vs child crates on day one?
2. What are the canonical command behaviors with Mother available/unavailable?
3. Which seams are intentionally permanent contracts vs temporary migration scaffolding?
4. How does Mother load bundled/runtime/project children deterministically?
5. What is the minimal, enforceable toy grant and scope model?
6. What proofs must pass before replacing current paths with greenfield equivalents?

## Deliverables

1. Architecture map with ownership boundaries and rationale.
2. Runtime policy matrix and failure semantics.
3. Data/storage model with ownership and lifecycle.
4. Child and toy contract model with schema examples.
5. Interface-runtime guest contract and session workflow expectations.
6. Migration slices with parity gates and rollback rules.
7. Verification matrix that can be executed by another agent without hidden context.

## Verification

- All GF exit criteria are backed by concrete sections in SPEC + DESIGN.
- Every claim that references current code includes `path:line` or command proof.
- Migration slices include explicit parity gates and rollback triggers.
- No unresolved contradiction remains between this greenfield target and locked beliefs.

## Build Readiness

Ready when promoted to active.
Execution starts only after review against current refactor truth map and beliefs.
