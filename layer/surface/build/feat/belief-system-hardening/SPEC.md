---
type: feat
id: belief-system-hardening
status: draft
created: 2026-04-01
sessions:
  origin: 20260331-072235-030494000
references:
  - layer/core/patina-identity.md
  - layer/core/spec-driven-design.md
  - layer/core/session-capture.md
  - layer/core/unix-philosophy.md
related:
  - layer/surface/epistemic/beliefs/
  - layer/surface/build/fix/code-audit-remediation/SPEC.md
exit_criteria:
  - id: bsh1-core-contract
    text: "Belief system contract is explicit: beliefs are the product; scrape/oxidize/scry/assay/context/belief are the truth-maintenance machinery."
    checked: false
  - id: bsh2-active-belief-quality-gate
    text: "Active beliefs must satisfy minimum quality gates (evidence, applied-in, anchor links, and non-floating grounding status)."
    checked: false
  - id: bsh3-human-in-loop-proposals
    text: "Introduce first-class proposal flow: 'I noticed X, should this become a belief?' with explicit human accept/reject."
    checked: false
  - id: bsh4-defeat-and-conflict-loop
    text: "Introduce first-class conflict flow: 'changes defeat belief Y; keep change, revise belief, or abort change' with decision capture."
    checked: false
  - id: bsh5-truth-over-assertion
    text: "System can mark beliefs as ungrounded/contested when project truth cannot support them; unsupported beliefs cannot remain active without explicit override scope."
    checked: false
  - id: bsh6-interface-universal-skill
    text: "Belief skill contract is interface-agnostic (Claude/OpenCode/Gemini): same inputs, same evidence schema, same outputs; wrappers are runtime-specific only."
    checked: false
  - id: bsh7-audit-actionability
    text: "`patina belief audit` gains actionable modes: quality failures, conflict queue, stale queue, and suggested fix commands."
    checked: false
  - id: bsh8-zero-context-truth-pack
    text: "Define and generate a compact 'truth pack' for zero-context models containing highest-value grounded beliefs and anchors."
    checked: false
  - id: bsh9-ci-policy
    text: "CI gate enforces belief quality thresholds (no new floating active beliefs; capped contested count unless explicitly allowlisted)."
    checked: false
  - id: bsh10-final-proof
    text: "End-to-end proof: propose new belief, detect conflict from code change, resolve with human decision, rerun audit, and verify policy pass/fail behavior."
    checked: false
---

# feat: Belief System Hardening

## Problem

Beliefs are central to Patina, but current behavior allows too many active beliefs to remain weakly grounded and passively contradictory. This muddies project understanding for both humans and LLMs.

Current gap:

- Belief capture is possible and useful, but quality enforcement is weak.
- Contradictions are visible but not operationally enforced.
- Human-in-loop interaction exists ad hoc, not as a first-class workflow.
- Interface-specific skill behavior can drift.

## Goal

Harden the belief system so it tells a true, current project story:

1. Beliefs remain human-authored/approved.
2. Truth is system-anchored in project evidence.
3. Conflicts trigger explicit decision loops.
4. Interface behavior is consistent across AI runtimes.

## Core Statement

Beliefs are the product. `scrape`, `oxidize`, `scry`, `assay`, `context`, and `belief` are the machinery that keeps that product true.

## Human-in-Loop Rules

- System may propose: "I noticed X; should this become a belief?"
- System must ask on conflict: "This change defeats belief Y. Keep change, revise belief, or abort?"
- Human decision is authoritative and must be captured.
- Unsupported claims can remain only as scoped hypotheses, not active truth.

## Truth Rules

- Active beliefs require grounding anchors.
- Ungrounded/contested beliefs are surfaced as debt and cannot silently persist.
- Reality beats preference: if project truth does not support a claim, demote/revise or implement the missing reality.

## Verification

```bash
cargo check --workspace -q
cargo test -q --lib
patina scrape --rebuild
patina belief audit --warnings-only --grounding
```
