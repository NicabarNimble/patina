---
type: refactor
id: patina-vision-lock-pre-v1
status: draft
created: 2026-03-22
sessions:
  origin: 20260321-162736-004031000
beliefs:
- patina-is-knowledge-protocol
- agents-are-guests-mother-is-infrastructure
- core-primitives-are-not-children
- core-verbs-standalone-mother-additive
- core-baseline-child-strategy-extensions
blocked_by: []
related:
- layer/surface/epistemic/beliefs/patina-is-knowledge-protocol.md
- layer/surface/epistemic/beliefs/agents-are-guests-mother-is-infrastructure.md
- layer/surface/epistemic/beliefs/core-primitives-are-not-children.md
- layer/surface/epistemic/beliefs/core-verbs-standalone-mother-additive.md
- layer/surface/build/refactor/vocabulary-alignment-child-manifest/SPEC.md
- src/commands/spec/mod.rs
- src/commands/spec/internal/
- src/mother/daemon_client.rs
- src/commands/mother/daemon.rs
- mother/src/daemon.rs
exit_criteria:
- id: VL1
  text: Architecture contract is explicit and frozen — Patina is protocol/product, Mother is local runtime infrastructure, children are strategy extensions (not protocol ownership)
  checked: false
- id: VL2
  text: Canonical command paths contain no scaffold placeholders; no shipped path returns "not yet implemented" on success path
  checked: false
- id: VL3
  text: Runtime requirement policy is explicit and implemented (mother-required for living mode, with defined snapshot/degraded behavior)
  checked: false
- id: VL4
  text: Spec lifecycle hardening exists — human-in-the-loop confirmation required for spec complete/abandon
  checked: false
- id: VL5
  text: Spec lifecycle supports reopen and rename subcommands with deterministic file/tag/reference updates
  checked: false
- id: VL6
  text: Criteria governance is enforced — exit criteria edits after active status require amendment metadata and rationale
  checked: false
- id: VL7
  text: Evidence tiers are enforced per EC (repro command + expected output + artifact location), not narrative-only completion
  checked: false
- id: VL8
  text: Spec-manager child architecture decision is made and documented with a concrete migration plan and bootstrap/recovery policy
  checked: false
- id: VL9
  text: Mother capability map is published (real/partial/deferred) and linked from specs to prevent semantic drift
  checked: false
- id: VL10
  text: Zero-context reproducibility check passes — another agent can determine pass/fail for all gates from spec+evidence alone
  checked: false
---
# refactor: refactor: Patina Vision Lock Pre-v1

> Lock protocol/Mother/child semantics, eliminate scaffold ambiguity, and require evidence-backed completion gates before further build work.

## Problem

Recent pre-v1 execution produced significant architectural progress but also surfaced a trust gap: specs can reach lifecycle completion before semantic alignment is externally reviewed. This creates "ghost completion" risk where criteria are technically checked but the outcome still feels wrong to product vision.

At the same time, Patina's architecture contract drifted repeatedly between protocol-first and daemon-first interpretations. Without a frozen contract + evidence discipline, agents optimize for lifecycle throughput instead of epistemic truth.

## Goal

Establish a single source-of-truth pre-v1 vision lock that:

1. fixes architecture semantics,
2. removes scaffold ambiguity,
3. hardens spec lifecycle trust,
4. and defines evidence-grade completion.

This spec is a governance and alignment lock before further build-stream closure work.

## Status

Draft. This spec should be promoted before any new pre-v1 closure or adjacent architecture specs are completed.

## Non-Goals

- No broad feature expansion unrelated to trust/contract alignment.
- No rewrite of historical commits/spec outcomes.
- No immediate deletion of all compatibility bridges unless tied to a locked gate in this spec.

## Current State

- Major vocabulary and manifest alignment landed (`child.toml`, `kind` canonical + compatibility bridge).
- Mother runtime is split: extracted daemon skeleton plus richer in-process mother runtime surfaces.
- Some command flows still include daemon-first + fallback behavior or scaffold filtering logic.
- Spec lifecycle supports create/promote/complete/abandon/pause/resume/block/set/split but lacks first-class `rename` and `reopen`, and lacks mandatory human confirmation gates.
- Exit criteria checking is not yet bound to evidence tiers.

## Target State

- Architecture doctrine is explicit, frozen, and reflected in command/runtime behavior.
- No canonical success path emits scaffold placeholders.
- Mother-required "living mode" policy (and snapshot/degraded semantics) is explicit in CLI/runtime behavior.
- Spec lifecycle is trustworthy-by-default with HITL finalization and criteria amendment controls.
- Evidence-driven closure is mandatory and reproducible by a zero-context reviewer.

## Solution

Execute a bounded lock sequence:

1. **Contract lock:** encode Patina/Mother/child semantics and runtime policy.
2. **Scaffold purge on canonical paths:** remove/ban placeholder success responses.
3. **Spec lifecycle hardening:** add HITL + reopen/rename + criteria amendment controls.
4. **Evidence governance:** require proof artifacts per EC and enforce tiered evidence.
5. **Spec-manager decision:** choose child migration path (or scoped defer) with explicit bootstrap policy.
6. **Capability transparency:** publish Mother capability map linked from specs.

## Implementation Order

1. Add architecture contract + runtime policy section to this spec and referenced docs.
2. Add scaffold detection checks and inventory all canonical command paths.
3. Implement spec lifecycle hardening (`rename`, `reopen`, HITL complete/abandon).
4. Implement criteria amendment tracking + evidence-tier validation.
5. Produce spec-manager child ADR-style decision + migration plan.
6. Publish and link Mother capability map.
7. Run zero-context pass/fail dry run with independent reviewer.

## Resolved Decisions

- Historical outcomes are preserved; this spec reconciles semantics without history rewrite.
- Completion requires both behavior truth and evidence truth.
- "Done" means reviewer-reproducible, not author-asserted.

## Verification

- `patina spec check patina-vision-lock-pre-v1 --json` reflects objective gate status.
- Grep-based scaffold check for canonical paths returns zero placeholder markers:
  - `not yet implemented` on canonical success routes.
- Lifecycle tests cover:
  - `spec reopen`
  - `spec rename`
  - HITL-required completion/abandon flows.
- Criteria amendment tests show immutable criteria body after active unless amendment metadata is present.
- Independent zero-context review packet yields same gate pass/fail outcomes.

## Exit Criteria

See metadata `exit_criteria` (VL1-VL10).

## Build Readiness

Ready when:

- this spec is promoted to active,
- and new/refined build specs include explicit dependency on this lock or explicitly justify why they do not.
