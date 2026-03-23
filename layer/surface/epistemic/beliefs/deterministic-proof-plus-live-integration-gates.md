---
type: belief
id: deterministic-proof-plus-live-integration-gates
persona: architect
facets: [workflow, verification, architecture]
entrenchment: medium
status: active
endorsed: true
extracted: 2026-03-22
revised: 2026-03-22
---

# deterministic-proof-plus-live-integration-gates

For risky refactors, require both deterministic isolated proofs and live integration proofs, and gate phase transitions with explicit rollback rules.

## Statement

For risky refactors, require both deterministic isolated proofs and live integration proofs, and gate phase transitions with explicit rollback rules.

## Evidence

- SPEC/DESIGN hardening during [[session-20260322-134406-020610000]] established isolated and integration proof paths plus rollback gates for phase safety (weight: 1.0)

## Supports

- [[specs-require-zero-ambiguity]] — deterministic and integration proofs reduce ambiguous completion claims.
- [[safeguards-from-workflow]] — rollback gates and phase entry checks are workflow safety guards.
- [[spec-driven-design]] — phase transitions depend on explicit proof, not implied progress.

## Attacks

<!-- Add beliefs this defeats -->

## Attacked-By

<!-- Add beliefs that challenge this -->

## Applied-In

- [[patina-code-to-vision]] tightened proof policy and phase gates in `layer/surface/build/refactor/patina-code-to-vision/SPEC.md` and `layer/surface/build/refactor/patina-code-to-vision/DESIGN.md`.
- Added deterministic/integration checker `resources/scripts/check-core-verb-policy.sh` with `--mode off --isolated` and `--mode on` paths.

## Revision Log

- 2026-03-22: Created — metrics computed by `patina scrape`
