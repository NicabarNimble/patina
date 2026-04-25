---
type: belief
id: contracts-outlive-runtimes
status: active
confidence: medium
created: 2026-04-22
evidence:
  - layer/surface/build/feat/slate-pando-migration/SPEC.md
  - layer/surface/build/feat/legacy-typed-bridge-seam/SPEC.md
  - commit-2e3d04b2
  - commit-98ed1113
---

# Contracts Outlive Runtimes

Prioritize stable typed contracts and policy envelopes over runtime-specific wiring.
Runtimes are expected to change; contracts must remain dependable.

## Rule

- Keep domain behavior at typed interface boundaries.
- Allow runtime/adapter replacement behind those boundaries.
- Preserve caller surface compatibility while internals migrate.

## Why

Runtime ecosystems move at different speeds. If runtime details leak into business behavior,
migration cost explodes and safety guarantees drift.

## Anchors in this repo

- Slate migration kept `patina spec` user surface while routing internals to typed operations in [[slate-pando-migration]].
- Typed route parity harness and routing hardening in [[commit-98ed1113]] and [[commit-2e3d04b2]].
- Release continuity while migration progressed: [[tag-v0.64.1]], [[tag-v0.64.2]], [[tag-v0.64.3]].
