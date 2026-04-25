---
type: belief
id: control-plane-authority-distributed-execution
status: active
confidence: medium
created: 2026-04-22
evidence:
  - layer/surface/build/feat/child-construction-canon/SPEC.md
  - layer/surface/build/feat/slate-pando-migration/SPEC.md
  - commit-2e3d04b2
---

# Control Plane Authority, Distributed Execution

Centralize discovery, policy, and conflict decisions in control plane; execute work at the owning child.

## Rule

- Control plane owns: registration, grants, policy, routing decisions.
- Data plane owns: command execution in the specific child that implements capability.
- Never force all execution through a single bridge path when direct typed routing is available.

## Why

This avoids central bottlenecks, preserves least-privilege boundaries, and keeps ownership explicit.

## Anchors in this repo

- Mother dispatch routes typed Slate operations to the owning child path in [[commit-2e3d04b2]].
- MCT direction and child role separation tracked in [[child-construction-canon]] and [[slate-pando-migration]].
- Current release lane where this pattern is active: [[tag-v0.64.3]].
