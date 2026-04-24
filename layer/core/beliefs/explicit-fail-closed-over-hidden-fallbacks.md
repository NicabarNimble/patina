---
type: belief
id: explicit-fail-closed-over-hidden-fallbacks
status: active
confidence: medium
created: 2026-04-22
evidence:
  - layer/surface/build/feat/slate-pando-migration/SPEC.md
  - layer/core/beliefs/temporal-layering-causes-drift.md
  - commit-0cd5833e
  - commit-2e3d04b2
---

# Explicit Fail-Closed Over Hidden Fallbacks

When ownership is changing between systems, unsupported paths should fail explicitly rather than silently tunnel through compatibility backdoors.

## Rule

- If command is not owned/implemented on the active path, return a deterministic error.
- Keep compatibility shims temporary, named, and scheduled for removal.
- Prefer visible contract gaps over silent behavioral drift.

## Why

Hidden fallbacks create long-lived dual systems and make reliability claims unverifiable.
Fail-closed behavior keeps migration debt visible and testable.

## Anchors in this repo

- Execute-path strictness and typed routing hardening in [[slate-pando-migration]] and [[commit-2e3d04b2]].
- Response-shape parity cleanup to reduce hidden drift in [[commit-0cd5833e]].
- This extends and operationalizes [[temporal-layering-causes-drift]].
- Release anchors during this migration window: [[tag-v0.64.2]], [[tag-v0.64.3]].
