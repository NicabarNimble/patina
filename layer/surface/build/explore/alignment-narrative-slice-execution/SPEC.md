---
type: explore
id: alignment-narrative-slice-execution
status: active
created: 2026-03-14
sessions:
  origin: 20260313-155708-WKJS
exit_criteria:
  - id: protocol-fields-defined
    text: Spec execution protocol defines required alignment fields and the slice contract sentence
    checked: true
  - id: split-and-scope-rules-defined
    text: Protocol defines mandatory non-goals and explicit split trigger when a slice exceeds one behavior or boundary
    checked: true
  - id: verification-and-handoff-rules-defined
    text: Protocol defines binary verification requirements and handoff packet expectations for continuity
    checked: true
---
# explore: Alignment Narrative for Slice Execution

> Codify a compact, repeatable slice execution protocol so specs stay narrow, verifiable, and aligned with doctrine.

## Intent

This spec formalizes the execution style that already works best in Patina:

- spec broadly,
- execute narrowly,
- split when scope expands,
- verify with binary commands.

It is a workflow protocol spec, not a product feature lane.

## Protocol

Every executable slice spec must include these alignment fields before coding:

1. `vision` — user-visible outcome.
2. `code_truth` — what the repository currently does (facts).
3. `preferences` — user coding preferences and constraints.
4. `constraints` — hard limits (safety/perf/architecture).

Every slice must include this contract sentence:

"Given vision V, code truth T, preferences P, and constraints C, this slice
changes X to achieve Y, verified by Z."

## Required Rules

1. One slice = one behavior or one integration boundary.
2. Every slice must declare explicit `non_goals`.
3. Every slice must include binary verification commands.
4. Dependency policy is a gate: in-tree first; new dependency requires explicit
   gap evidence.
5. If work expands beyond one behavior/boundary, split immediately with
   `patina spec split <id>`.
6. Handoffs must preserve the slice contract and verification status using
   `patina spec handoff <id>` / `patina spec packet <id> --json`.

## Doctrine Fit

This protocol enforces doctrine during execution:

- Mother decides: scope, policy, and acceptance authority in specs.
- Children act: implementation slices execute bounded behavior.
- Toys constrain action surface: capability boundaries stay explicit and
  testable.

## Command Mapping

- Ground truth: `patina context`, `patina scry`, `patina assay`
- Define/adjust scope: `patina spec create`, `patina spec split`
- Execution packet: `patina spec prompt`
- Handoff/continuity: `patina spec handoff`, `patina spec packet --json`
- Closure proof: `patina spec check <id> --json`

## Outcome Target

Alignment narrative becomes a lightweight execution protocol for future slices
after adoption evidence is recorded in active downstream specs/sessions.
