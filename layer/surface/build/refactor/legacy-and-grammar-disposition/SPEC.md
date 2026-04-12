---
type: refactor
id: legacy-and-grammar-disposition
status: draft
created: 2026-04-11
sessions:
  origin: 20260410-220235-028265000
related:
- children/
- grammars/
- sdk/patina-sdk-legacy/
- src/child/internal/pipeline.rs
- src/commands/mother/daemon.rs
- layer/surface/build/feat/child-construction-canon/SPEC.md
- layer/surface/build/refactor/child-typed-composition/SPEC.md
- layer/surface/build/feat/sdk-vision-lock/SPEC.md
beliefs:
- '[[children-have-agency-toys-are-capabilities]]'
- '[[wasi-is-foundation-not-option]]'
exit_criteria:
- id: lgd1-portfolio-inventory
  text: "A committed disposition inventory exists listing: six typed baseline children, legacy service children, and grammar children with current runtime lane and risk notes."
  checked: false
- id: lgd2-service-child-decisions
  text: "Each legacy service child (belief-verifier, session-writer, spec-manager, doctor) has an explicit decision: keep legacy now, migrate by phase, or retire/replace, with rationale."
  checked: false
- id: lgd3-grammar-lane-decision
  text: "Grammar lane has explicit long-term contract: typed-composition integration plan or legacy pipeline containment plan, including constraints and target milestones."
  checked: false
- id: lgd4-owner-deadline
  text: "Every migrate/retire decision has owner + target window (release/spec milestone) and dependency notes."
  checked: false
- id: lgd5-spec-manager-path
  text: "Spec-manager decision path is explicit (remain service-handle lane temporarily vs migrate to wasm child path) and linked to its governing spec(s)."
  checked: false
- id: lgd6-no-implicit-carryover
  text: "No child remains in legacy/pipeline lane without documented disposition status; 'implicit carryover' is eliminated."
  checked: false
---
# refactor: Legacy service children + grammar lane disposition

> Define explicit keep/migrate/retire decisions for legacy handle-based service children and pipeline grammar children, with owners, deadlines, and migration contracts aligned to SDK vision lock.

## Problem

Patina has a strong typed trajectory for the six baseline data-plane children, but
legacy service children and grammar children still span older lanes. Without a
single disposition artifact, these lanes drift by inertia.

## Goal

Produce an explicit, approved child-portfolio disposition that answers:

1. Which children are in the typed baseline set (already true for six baseline children).
2. Which legacy service children stay temporarily, and why.
3. What happens to grammar children long-term (integrate or contain).
4. Who owns each move and when it lands.

## Non-Goals

- Full migration implementation in this spec.
- Rewriting all grammar implementations.
- Runtime engine merge by itself (tracked in engine/conposition specs).

## Current State

- Six baseline children are on typed SDK lane.
- Service children (`belief-verifier`, `session-writer`, `spec-manager`, `doctor`) use
  `sdk/patina-sdk-legacy` handle-based lane.
- Grammar children under `grammars/*` are pipeline-lane plugins.
- Multiple active drafts reference these lanes, but no single disposition matrix
  blocks ambiguity.

## Target State

A committed disposition matrix (policy artifact) with explicit status for every
non-baseline child lane:

- **KEEP (bounded):** stays on legacy lane for now, with review date.
- **MIGRATE:** target lane + acceptance criteria + milestone.
- **RETIRE/REPLACE:** replacement path and cutoff condition.

## Solution

1. Create disposition matrix file (path decided in design phase).
2. Fill per-child decisions for legacy service children.
3. Fill per-grammar decision and lane contract.
4. Link each decision to owning spec(s) and milestones.
5. Treat missing decision as spec failure.

## Implementation Order

1. Inventory current children and lane status.
2. Decide service-child statuses with rationale and dependency links.
3. Decide grammar lane strategy and milestone.
4. Publish matrix with owner/deadline fields.
5. Add drift check so undocumented children fail policy review.

## Resolved Decisions

- Portfolio ambiguity is a blocker.
- Legacy and grammar lanes must be intentional, not accidental.

## Verification

```bash
patina spec check legacy-and-grammar-disposition --json
patina child list
# plus matrix validation/check script added by implementation
```

## Exit Criteria

Frontmatter `lgd1..lgd6` are source of truth.

## Build Readiness

High for policy definition. Medium for follow-on execution across children.
