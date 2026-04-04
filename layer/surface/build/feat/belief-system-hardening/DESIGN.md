# Design: Belief System Hardening

## Design Intent

Preserve the strongest part of the current model (human-in-loop belief authoring) while making belief truth operationally enforceable.

This design treats belief quality as an engineering system, not just a documentation style.

## Operating Model

### 1) Belief Lifecycle States

- `proposed`: machine or human proposal, not yet accepted.
- `active`: accepted and grounded by project truth.
- `scoped`: valid only under explicit constraints.
- `contested`: currently contradicted by evidence or verification.
- `defeated`: superseded/invalidated.
- `archived`: historical reference.

### 2) Human + Truth Division

- Human owns intent and final decision.
- System owns evidence checks and conflict detection.

Decision matrix:

- proposal without evidence -> keep as proposed/scoped.
- evidence supports -> promote to active.
- change conflicts with active belief -> force explicit decision:
  - keep change + revise belief,
  - keep belief + reject change,
  - scope/defeat belief.

### 3) Core Toolchain as Truth Machinery

- `scrape`: refresh code/git/session/belief projections.
- `oxidize`: improve semantic retrieval quality.
- `scry`/`context`/`assay`: retrieve and verify anchors.
- `belief`: lifecycle, audit, conflict, and status management.

These remain contract-level roles even as implementation moves toward child-backed surfaces.

## Required Product Surfaces

### Belief Proposal Surface

- `patina belief propose` (or equivalent) with mandatory evidence scaffold.
- AI interface wrappers call same contract.

### Belief Conflict Surface

- `patina belief conflicts` lists change-belief conflicts.
- `patina belief resolve <id>` records keep/revise/abort decisions.

### Quality Gate Surface

- `patina belief audit --warnings-only --grounding`
- machine-readable output for CI policy checks.

### Truth Pack Surface

- `patina belief pack` outputs compact, high-signal grounded belief set for zero-context models.

## Interface-Universal Skill Contract

Canonical skill payload must be shared across interfaces:

- Input schema: statement, evidence anchors, support/attack links, scope.
- Output schema: belief file diff + audit delta + unresolved conflicts.
- Runtime wrappers (`.claude`, `.opencode`, `.gemini`) adapt execution only.

No interface may invent belief semantics outside canonical contract.

## Policy and CI

Suggested default policy:

- fail if new active floating beliefs are introduced,
- fail if contested active beliefs increase without resolution note,
- warn on stale active beliefs beyond threshold days,
- allow explicit temporary waivers via tracked allowlist.

## Implementation Sequence

1. Spec/skill contract alignment (remove contradictory confidence guidance).
2. Add proposal + conflict queue data model.
3. Add resolution commands and decision capture.
4. Add quality-gate enforcement in audit and CI.
5. Add zero-context truth pack output.

## Non-Goals

- Replacing human judgment with auto-belief acceptance.
- Treating every assertion as active truth.
- Forcing all beliefs to be code-only (sessions/specs/assay/scry anchors remain valid truth sources).
