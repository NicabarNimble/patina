---
type: belief
id: spec-is-milestone
status: active
confidence: high
created: 2026-02-05
evidence:
  - layer/surface/build/explore/version-rules-system/SPEC.md
  - commit-e6b1aa96
---

# Spec is the Milestone

A spec is not a container for milestones — **a spec IS a milestone**. One spec, one unit of work, one version bump.

## The Rule

```
Complete a spec → bump version based on type → git tag

  fix/refactor → patch (0.0.x)
  feat         → minor (0.x.0)
  explore      → no bump
```

## What This Replaces

- No `target: v0.12.0` in specs (aspirational, becomes stale)
- No `milestones` array with planned versions
- No `released` field (git tags are history)
- No batched releases (each spec is its own release)

## Why It Works

1. **Version is derived, not planned** — spec type determines impact
2. **Git is history** — want to know when something shipped? check tags
3. **Simple mental model** — complete spec = release
4. **No coordination overhead** — don't need to decide "what goes in v0.12.0"

## Anti-pattern

Planning versions ahead: "v0.12.0 will have X, Y, Z" — this creates:
- Stale `target` fields when priorities shift
- Re-targeting busywork
- Pressure to batch unrelated work into releases
