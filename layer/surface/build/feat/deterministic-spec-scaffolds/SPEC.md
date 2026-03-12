---
type: feat
id: deterministic-spec-scaffolds
status: ready
created: 2026-03-12
sessions:
  origin: 20260312-001728
related:
- src/commands/spec/mod.rs
- src/commands/spec/internal/create.rs
- layer/surface/build/refactor/mother-doctrine-cleanup/SPEC.md
- layer/surface/build/feat/knowledge-child-platform/SPEC.md
beliefs:
- specs-describe-current-code-not-aspirations
exit_criteria:
- id: create-scaffold-has-strong-flow
  text: New feat/refactor specs are scaffolded with a stronger deterministic flow including Problem, Goal, Status, Non-Goals, Target Shape, Resolved Decisions, Implementation Order, Verification, and Build Readiness
  checked: false
- id: design-scaffold-has-implementation-contract
  text: New DESIGN.md files scaffold direct code targets, resolved decisions, build target, and build readiness instead of a minimal open-ended outline
  checked: false
- id: readiness-lint-detects-ambiguous-specs
  text: Spec tooling can flag or fail specs that still contain unresolved ambiguous architecture language when promoted to ready
  checked: false
- id: code-targets-supported-first-class
  text: Spec workflow supports direct code target capture so agents can anchor implementation to concrete files and lines without relying only on prose
  checked: false
- id: thin-interface-layer-preserved
  text: Spec tooling improvements focus on deterministic spec structure and agent handoff, not on baking interface-specific skill behavior into the core spec lifecycle
  checked: false
---
# feat: Deterministic Spec Scaffolds For Agents

> Improve Patina's spec tooling so agents get stronger, more deterministic spec scaffolds with thin interface-specific layers and more explicit architecture commitments.

## Problem

Patina's spec system is powerful, but the default scaffold is still too
thin for autonomous agents doing serious architecture work.

Current gaps:

- `patina spec create` produces a minimal body template in
  `src/commands/spec/internal/create.rs:24` that does not force the
  stronger flow seen in successful specs like
  `knowledge-child-platform`.
- the default `DESIGN.md` scaffold in
  `src/commands/spec/internal/create.rs:42` is too open-ended and does
  not demand direct code targets, resolved decisions, or build-readiness
  framing.
- interface-specific skill layers may differ across Claude, OpenCode,
  Gemini, or future runtimes, so the core spec system should not depend
  on agent-side prompt quality to get a good result.
- agents can still create specs that are structurally valid but too
  ambiguous to implement without reopening architecture.

This creates avoidable drift: a strong model with weak scaffolding can
still produce specs that are prose-complete but implementation-unsafe.

## Goal

Improve the deterministic parts of the spec system so agents get better
results even when interface skills differ.

**Target shape:**

- the core spec scaffold encodes a strong implementation flow by default
- interface-specific skill layers stay thin and mostly advisory
- specs capture resolved architecture decisions more explicitly
- specs point directly at code targets, not only abstract concepts
- promoting a spec to `ready` becomes harder if the architecture is still
  underspecified

## Status

Current state:

- `patina spec create` works and creates usable draft specs
- `patina spec promote` / `patina spec check` support lifecycle flow
- strong specs do exist in-tree, but mostly because the author supplied
  them manually rather than because the tool scaffold required them

The best evidence is the gap between:

- the minimal generated scaffold from `src/commands/spec/internal/create.rs`
- higher-quality hand-authored specs like
  `layer/surface/build/feat/knowledge-child-platform/SPEC.md`

The tooling should narrow that gap.

## Non-Goals

- Do not make the spec system dependent on one interface runtime's skill
  format or prompt style.
- Do not push large interface-specific behavior into the core spec
  lifecycle.
- Do not materially replace the current spec workflow shape. The system
  already works; this spec improves the deterministic quality floor
  without turning specs into a different product.
- Do not replace human architectural judgment with rigid templates.
- Do not require every spec to be maximally long; the goal is stronger
  structure, not mandatory verbosity.

## Solution

### 1. Strengthen the default spec scaffold

Update `patina spec create` so new feat/refactor specs default to a
stronger structure more like the successful `knowledge-child-platform`
flow.

For specs, scaffold sections such as:

- Problem
- Goal
- Status
- Non-Goals
- Target Shape or Resolved Decisions
- Solution
- Implementation Order
- Verification
- Build Readiness

### 2. Strengthen the default design scaffold

Update `DESIGN.md` scaffolding so agents start with implementation
contracts rather than a blank outline.

For design docs, scaffold sections such as:

- Why This Design
- Build Target
- Resolved Decisions
- Commits
- Direct Code Targets
- Verification Plan
- Build Readiness
- Open Questions

### 3. Add deterministic readiness checks

Promoting a spec to `ready` should have stronger linting for ambiguous
architecture language.

Examples of things the tool should detect or warn about:

- unresolved “maybe/consider/if needed” phrasing in core architecture
  sections
- mismatch between exit criteria and spec body
- specs with build-critical open questions but no resolved decision
  section
- specs with no direct code/file targets for implementation-heavy work

### 4. Support first-class code targets and handoff views

Spec tooling should let authors and agents capture direct code targets in
structured form, not only as prose.

It should also support a compact agent handoff view that shows:

- resolved decisions
- implementation order
- code targets
- exit criteria
- verification plan
- open questions requiring human input

### 5. Keep interface-specific layers thin

Claude, OpenCode, Gemini, and future interfaces may each have custom
skills or UX patterns, but the core spec system should stay focused on
deterministic project truth.

The core tooling should own:

- scaffold structure
- readiness checks
- lifecycle transitions
- code target support
- handoff views

Interface-specific skills should mainly help gather context and present
the workflow, not define the architecture contract itself.

## Resolved Decisions

- improve the existing spec workflow rather than replacing it
- keep interface-specific layers thin and mostly advisory
- start with prose conventions plus lint for direct code targets rather
  than introducing heavy new structured schema immediately
- readiness lint should fail by default when core architectural
  ambiguity remains, with an override path for exceptional cases
- agent handoff should start as an option on `patina spec show` rather
  than as a separate top-level command

## Exit Criteria

1. New feat/refactor specs are scaffolded with a stronger deterministic
   flow including Problem, Goal, Status, Non-Goals, Target Shape,
   Resolved Decisions, Implementation Order, Verification, and Build
   Readiness.
2. New DESIGN.md files scaffold direct code targets, resolved decisions,
   build target, and build readiness instead of a minimal open-ended
   outline.
3. Spec tooling can flag or fail specs that still contain unresolved
   ambiguous architecture language when promoted to ready.
4. Spec workflow supports direct code target capture so agents can
   anchor implementation to concrete files and lines without relying
   only on prose.
5. Spec tooling improvements focus on deterministic spec structure and
   agent handoff, not on baking interface-specific skill behavior into
   the core spec lifecycle.

## Build Readiness

This spec should result in core spec tooling that helps agents produce
better specs across different runtimes, even when the interface layer is
customized. The goal is to improve the deterministic foundation, not to
standardize every interface prompt surface or replace the current spec
workflow.
