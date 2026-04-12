---
type: feat
id: legacy-typed-bridge-seam
status: draft
created: 2026-04-11
related:
- layer/surface/build/feat/spec-atlas-mother-backplane/SPEC.md
- layer/surface/build/feat/child-construction-canon/SPEC.md
- mother/src/
- children/
beliefs:
- '[[children-have-agency-toys-are-capabilities]]'
- '[[core-verbs-standalone-mother-additive]]'
exit_criteria:
- id: ltbs1-seam-contract
  text: "A bridge seam contract exists for legacy-to-typed toy translation with explicit fail-closed behavior."
  checked: false
- id: ltbs2-mother-owned-policy
  text: "Mother-managed internal bridge module enforces allowlist mapping and rejects unknown legacy toys."
  checked: false
- id: ltbs3-typed-bridge-child
  text: "A typed WIT bridge child exists and processes bridge requests through a stable interface."
  checked: false
- id: ltbs4-no-direct-legacy-escalation
  text: "Bridge flow denies direct legacy capability escalation outside mapped seam."
  checked: false
- id: ltbs5-atlas-visibility
  text: "Atlas visibility can report legacy seam exposure via deterministic bridge policy signals."
  checked: false
- id: ltbs6-tests
  text: "Deterministic tests cover mapping success, unknown-toy denial, and bridge request/response shape."
  checked: false
---
# feat: Legacy Typed Bridge Seam

> Introduce a strong transition seam so legacy lanes can be contained while typed SDK lanes remain canonical.

## Problem

Current tree still contains legacy child/toy aliases (`log`, `state`, `store`, `fs`) alongside typed lanes.
That drift is observable, but execution policy is not yet centralized behind one strict seam.

## Goal

Build a bridge seam with two parts:

1. Mother-managed internal policy module (authority + fail-closed mapping).
2. Typed WIT bridge child (execution lane) that processes normalized bridge requests.

This allows migration without rewriting all legacy children immediately, while preventing unconstrained mixed runtime behavior.

## Constraints

- Mother remains additive infrastructure; core CLI verbs stay standalone.
- Legacy compatibility is transitional and must be explicitly time-boxed.
- Unknown toys or partial mappings must fail closed.
- Child terminology remains `child`/`kind`; no `world` vocabulary leakage outside WIT contexts.

## Verification

```bash
cargo check -q
cargo test -q legacy_bridge
patina atlas --json | jq '.summary'
```
