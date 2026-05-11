---
type: feat
id: slate-intent-truth-loop
status: draft
created: 2026-05-11
sessions:
  origin: 20260508-144836-859149000
beliefs:
- "[[spec-driven-design]]"
- "[[safety-boundaries]]"
- "[[dependable-rust]]"
- "[[control-plane-and-runtime-proof-are-separate-gates]]"
- "[[allium-as-business-backlog]]"
references:
- layer/core/values/spec-driven-design.md
- layer/core/values/safety-boundaries.md
- layer/core/values/dependable-rust.md
- layer/core/values/session-capture.md
- layer/core/beliefs/spec-is-milestone.md
- layer/core/beliefs/explicit-fail-closed-over-hidden-fallbacks.md
- layer/core/beliefs/temporal-layering-causes-drift.md
related:
- slate-pando-migration
- layer/allium/
- layer/surface/epistemic/beliefs/
- layer/core/
- children/slate-manager/
- children/slate-manager/wit-contract/slate.wit
- src/commands/spec/
- src/spec.rs
exit_criteria:
- id: si1-current-spec-actions-reviewed
  text: "Current `patina spec`/Slate lifecycle methods are reviewed and mapped to future Slate responsibilities without assuming SPEC.md remains the authority for intended behavior."
  checked: true
- id: si2-slate-work-item-model
  text: "Slate has an explicit work-item model for build/refactor/fix work that records human complaint, work kind, Allium anchors, user intent alignment, proof plan, relevant existing beliefs, and post-work evidence."
  checked: false
- id: si3-allium-first-intent-dialogue
  text: "Slate creation/activation dialogue uses Allium first to discover, confirm, or update intended behavior before implementation work is treated as ready."
  checked: false
- id: si4-user-alignment-gate
  text: "Slate refuses or blocks work when Allium intent is missing, stale, or disputed until HITL alignment is captured as part of the Slate record."
  checked: false
- id: si5-use-allium-drift-tools
  text: "Slate uses existing Allium check/analyse/plan/model and Allium skill workflows for code/spec drift instead of imposing a competing drift method."
  checked: false
- id: si6-belief-harvest-and-prune
  text: "Slate closure includes a belief harvest/challenge pass that uses existing belief markdown/core doctrine conventions, evidence links, `patina scrape`, and belief audit signals; it does not replace the belief system."
  checked: false
- id: si7-packets-and-wit-surface
  text: "Slate prompt/handoff/packet and WIT-facing result surfaces expose the Allium intent context, proof obligations, and belief harvest/challenge recommendations in structured form."
  checked: false
- id: si8-proof-tests
  text: "Tests cover build/refactor/fix Slate flows, Allium missing/stale intent blocking, belief harvest recommendations, and compatibility with existing `patina spec` routing modes."
  checked: false
---
# feat: Slate Intent Truth Loop

> Finish Slate as the build/refactor/fix todo system that uses Allium as intended truth and uses beliefs/core doctrine as defeasible evidence without replacing either system.

## Problem

The current Slate migration is parity-first around existing `patina spec` command behavior. That remains useful migration infrastructure, but it does not yet lock the product role we want Slate to grow into.

Slate should not become a new specification language, a replacement for Allium, or a replacement for beliefs. Slate is the todo/work system where changes happen: build, refactor, and fix. Its responsibility is to make sure each change starts from intended behavior, records user alignment, executes safely, proves the result, and then harvests or challenges beliefs from evidence.

The risk is two-sided:

1. **Allium can become lies** if implementation and business intent drift while agents keep treating old Allium as unquestionable truth.
2. **Beliefs can become stale doctrine** if their evidence disappears, proof changes, or business logic no longer supports them.

Slate must keep both systems honest while using their existing mechanisms.

## Goal

Implement Slate as the Patina change-work layer for build/refactor/fix tasks:

- Allium is the primary home for intended behavior/truth.
- Slate uses Allium deeply during HITL task creation and readiness.
- Slate blocks or asks when Allium intent is absent, stale, contradicted, or not user-confirmed.
- Slate executes work as a todo/change transaction, not as an authority over behavior.
- Beliefs and `layer/core` doctrine are used as constraints and post-work evidence surfaces.
- Beliefs are harvested, challenged, scoped, defeated, archived, or left alone through the existing belief system.
- Existing Allium and belief workflows are orchestrated, not replaced.

## Status

Draft. This spec follows and reframes `[[slate-pando-migration]]`: the migration work should keep the existing `patina spec` surface safe, but Slate does not need blind 1:1 parity where the intended workflow is changing.

## Non-Goals

- Do not change the Allium language, Allium CLI, or Allium skill semantics.
- Do not replace Allium drift/weed/tend/propagate workflows with Patina-specific substitutes.
- Do not replace the belief markdown schema, belief graph, `patina scrape`, or belief audit machinery.
- Do not remove `patina spec` compatibility as part of this slice.
- Do not make Slate the source of behavioral truth.
- Do not treat beliefs as pre-work guesses to be created before evidence exists.
- Do not include Mother display/buffer/frame work in this scope.

## Target Shape

A Slate is a durable work item for a change. It has enough structure to guide an agent and enough proof hooks to close honestly.

A Slate records:

- human complaint or desired change,
- work kind: build, refactor, or fix,
- affected Allium files and constructs,
- the user-confirmed intended behavior,
- open Allium questions, if any,
- relevant existing beliefs and `layer/core` doctrine,
- implementation plan,
- proof plan derived from Allium obligations where possible,
- execution evidence,
- belief harvest/challenge recommendations after work.

Slate lifecycle remains todo-oriented:

- `draft`: complaint captured, intent not yet aligned,
- `ready`: Allium intent and user alignment are captured,
- `active`: implementation work is underway,
- `blocked`: intent/proof/dependency issue prevents honest progress,
- `complete`: code, Allium, proof, and belief harvest are reconciled.

## Solution

### 1. Map current spec methods to Slate responsibilities

Review existing `patina spec` and `children/slate-manager` methods:

- create/list/ready/blocked/next as todo discovery,
- prompt/handoff/packet as agent work packets,
- check/complete/archive as closure gates,
- pause/block/resume as work-state controls,
- set/rename/reopen/history as management operations.

Keep useful mechanics, but stop treating SPEC.md prose as the long-term source of behavioral truth. Future Slate should represent most existing `spec` lifecycle actions as todo/workflow operations, while adding Allium intent context as the behavioral truth layer. Compatibility matters where behavior is preserved; blind parity is not required where Slate intentionally changes the workflow.

### 2. Add Allium-first intent context

Slate creation/activation should gather an Allium context before work is ready:

- relevant `.allium` files,
- parsed/modelled entities, rules, surfaces, contracts, and invariants where available,
- `allium check` diagnostics,
- `allium analyse` findings when the spec is mature enough,
- `allium plan` obligations where useful,
- open questions or missing Allium coverage.

Slate should classify the relationship between user complaint and Allium:

- Allium already states the desired behavior,
- Allium states old behavior and must be changed,
- Allium is silent and needs elicitation/tending,
- Allium is ambiguous and needs HITL decision,
- implementation-only refactor with no behavior change.

### 3. Require user alignment when truth may change

When behavior changes, Slate should not proceed on inference alone. It should capture HITL alignment that the Allium intent is now accepted business truth.

If the user says the old behavior is no longer desired, Slate should route to Allium update/tending before implementation closure. If Allium and user intent conflict, Slate blocks until the conflict is resolved.

### 4. Use, not replace, Allium drift workflows

Slate should invoke or guide existing Allium mechanisms:

- `allium check` for structural validation,
- `allium analyse` for process gaps,
- `allium model` for domain shape,
- `allium plan` for test/proof obligations,
- Allium `tend` style work for changing intended behavior,
- Allium `weed` style work for code/spec drift analysis.

Slate stores references, summaries, and decisions. It does not define a parallel drift semantics.

### 5. Use beliefs as defeasible evidence, mostly post-work

Existing beliefs and `layer/core` values may constrain a Slate before implementation. New or revised beliefs should usually be produced after contact with reality:

- what happened,
- what we now believe,
- what evidence proves it,
- where it applies,
- what would defeat or scope it.

Slate closure should recommend one of:

- no belief change,
- add evidence to an existing belief,
- create a new belief from proven experience,
- scope an over-broad belief,
- mark a belief contested through attacks/failed verification,
- defeat/archive a belief whose proof disappeared.

The actual belief artifacts remain in `layer/surface/epistemic/beliefs/` and `layer/core/` conventions.

### 6. Make proof closure explicit

A Slate is not complete merely because code changed. It is complete when:

- intended Allium behavior is aligned,
- implementation work is done,
- verification evidence exists,
- Allium/code drift is addressed or explicitly classified,
- belief harvest/challenge pass is recorded,
- no stale doctrine is silently preserved as truth.

## Implementation Order

1. Review current `spec` and Slate child methods against this target model.
2. Define a structured Slate work-item model for build/refactor/fix tasks.
3. Add Allium intent context collection to Slate prompt/packet generation.
4. Add user-alignment/readiness gates for missing, stale, or disputed Allium intent.
5. Add proof obligation extraction using existing Allium CLI outputs where available.
6. Add belief/core doctrine loading as constraints for active work.
7. Add post-work belief harvest/challenge recommendations.
8. Extend WIT and packet surfaces to expose the new structured context.
9. Add tests for build/refactor/fix flows and failure/blocking cases.
10. Reconcile this direction with `[[slate-pando-migration]]` so compatibility work remains migration infrastructure, while intentional Slate workflow changes are documented rather than forced into blind parity.

## Resolved Decisions

- Slate is the todo/change-work system for build/refactor/fix.
- Allium is where intended behavioral truth lives.
- Allium must be user-aligned; stale Allium is not truth.
- Slate uses Allium tools and skills instead of altering Allium.
- Beliefs are defeasible doctrine/evidence, not primary pre-work intent.
- Beliefs are actively challenged and pruned when proof changes or disappears.
- `layer/core` markdown remains reusable doctrine and value grounding.
- Existing `spec` mechanics are migration scaffolding and lifecycle precedent, not the final authority model.
- Most current `spec` actions should be represented in Slate, but parity is required only for compatibility-preserved behavior; Allium is an additive intent layer, not a replacement for todo lifecycle mechanics.

## Verification

```bash
patina spec check slate-intent-truth-loop --json
cargo check -q --workspace
cargo test -q -p patina-ai-child-slate-manager
cargo test -q --lib spec
```

Behavior checks:

- A build Slate cannot become ready without Allium intent context or an explicit HITL reason why no Allium change is needed.
- A refactor Slate can proceed without Allium changes only when behavior-preservation intent and proof are recorded.
- A fix Slate classifies mismatch as code bug, Allium stale, belief stale, or ambiguous intent.
- Slate closure emits belief harvest/challenge recommendations without mutating belief semantics.
- Existing `patina spec` compatibility routing still works in `off`, `observe`, and `execute` modes.

## Exit Criteria

Frontmatter `si1..si8` are the source of truth.

## Build Readiness

Not ready until the initial Slate work-item model is accepted.
