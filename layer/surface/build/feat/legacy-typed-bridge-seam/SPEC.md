---
type: feat
id: legacy-typed-bridge-seam
status: draft
created: 2026-04-11
related:
- layer/surface/build/feat/child-construction-canon/SPEC.md
- layer/allium/mother/mother-view-composer-target.allium
- mother/src/
- children/
beliefs:
- '[[children-have-agency-toys-are-capabilities]]'
- '[[core-verbs-standalone-mother-additive]]'
exit_criteria:
- id: ltbs1-seam-contract
  text: "A bridge seam contract exists for legacy-to-typed toy translation with explicit fail-closed behavior."
  checked: true
- id: ltbs2-mother-owned-policy
  text: "Mother-managed internal bridge module enforces allowlist mapping and rejects unknown legacy toys."
  checked: true
- id: ltbs3-typed-bridge-child
  text: "A typed WIT bridge child exists and processes bridge requests through a stable interface."
  checked: true
- id: ltbs4-no-direct-legacy-escalation
  text: "Bridge flow denies direct legacy capability escalation outside mapped seam."
  checked: true
- id: ltbs5-bridge-policy-observable
  text: "Mother bridge policy exposes deterministic lane signals that future data catalog/view shapes can observe without a hardcoded Atlas surface."
  checked: true
- id: ltbs6-tests
  text: "Deterministic tests cover mapping success, unknown-toy denial, and bridge request/response shape."
  checked: true
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
cargo test -q -p mother bridge::tests
cargo check -q -p patina-ai-child-legacy-typed-bridge
cargo test -q -p mother bridge::tests
cargo check -q
```
