---
type: explore
id: version-rules-system
status: complete
created: 2026-02-05
beliefs:
- milestones-in-specs
---

# explore: Version as Derived Output

## Problem

The current version system hardcodes target versions in specs:
```yaml
target: v0.12.0
milestones:
  - version: "0.12.0"
    name: "Feature X"
    status: in_progress
```

This creates friction because:
1. **Moving targets** — priorities shift, specs get re-targeted
2. **Manual bookkeeping** — updating `target` fields when plans change
3. **Version planning != work planning** — we plan work, version should emerge

## The Simple Model

**A spec is a milestone.**

One spec = one unit of work = one version bump.

```
Complete a spec → bump version based on type → git tag

  fix/refactor → patch bump (0.0.x)
  feat         → minor bump (0.x.0)
  explore      → no bump (research only)
```

History lives in git. Spec type determines version impact.

No `target`. No `released`. No `milestones` array.

## Observation

Semver already encodes work type:
- `0.0.PATCH` — fixes, patches, bug repairs
- `0.MINOR.0` — features, new capabilities
- `MAJOR.0.0` — breaking changes, major releases

We already have spec types: `fix`, `feat`, `refactor`, `explore`

## Proposed Rules

Version is *derived* from completed work, not planned ahead:

| Spec Type | Version Impact |
|-----------|----------------|
| `fix`     | PATCH bump     |
| `feat`    | MINOR bump     |
| `refactor`| PATCH bump (no user-visible change) |
| `explore` | No bump (research only) |

**MAJOR** bumps are outside the rules — deliberate human decision for "this is v1.0".

## Questions to Explore

### 1. Bump Granularity

**Decision: Per-spec**

A spec is a milestone. Complete a spec → version bump.

```bash
patina spec status my-fix complete
# → patch bump (0.11.0 → 0.11.1)
# → git tag v0.11.1

patina spec status new-feature complete
# → minor bump (0.11.1 → 0.12.0)
# → git tag v0.12.0
```

Simple. No batching complexity. Each spec is a release.

### 2. What Happens to `target` Field?

**Decision: Remove `target`. No replacement needed.**

- **`target`** = planning artifact, becomes stale → **remove**
- **`released`** = not needed, git tags are the history

```yaml
# Spec file stays simple:
type: fix
id: my-fix
status: complete
```

When this spec completes → patch bump → `git tag v0.11.1`

Want to know what version it shipped in? `git log --oneline layer/surface/build/fix/my-fix/`

### 3. Multi-phase Features (Milestones)

**Decision: Phases without versions**

Current: milestones with specific versions
```yaml
milestones:
  - version: "0.12.0"
    name: "Phase 1"
  - version: "0.13.0"
    name: "Phase 2"
```

Proposed: phases without versions (or just use separate specs)
```yaml
phases:
  - name: "Phase 1"
    status: complete
  - name: "Phase 2"
    status: in_progress
```

Or even simpler: each phase is its own spec (`feat/thing-phase-1`, `feat/thing-phase-2`).

Version is determined at release time, not planning time.

### 4. v1.0 Planning

How do you express "v1.0 needs these features"?

**Option A: Tag-based**
```yaml
tags: [v1-required]
```

**Option B: Explicit v1 spec**
- A spec that lists exit criteria for v1.0
- Other specs reference it via `blocks: [v1-release]`

**Option C: Don't plan it**
- v1.0 is "when it feels ready"
- Human decision, not system-tracked

### 5. Commands

Current:
```bash
patina version milestone  # complete current milestone, bump version
patina version patch "description"  # manual patch
patina spec status <id> complete  # just marks complete, no version
```

Proposed:
```bash
patina spec status <id> complete  # marks complete + auto-bump + git tag
patina version major "v1.0"       # deliberate major bump (human decision)
```

The version command becomes simpler — only needed for major bumps.
Normal workflow is just `patina spec status <id> complete`.

## What We'd Remove

- `target` field from specs
- `milestones` array with version numbers
- `current_milestone` tracking
- `released` field (git is history)
- Complex milestone state machine
- Batched release logic

## What We'd Add

- Rules engine: spec type → version impact
- Auto-bump on spec completion (integrated into `patina spec status <id> complete`)
- Git tag created automatically

## Exit Criteria (for this explore)

- [x] Decide: per-spec vs batched releases → **per-spec** (spec = milestone)
- [x] Decide: fate of `target` field → **remove** (git is history)
- [x] Decide: fate of `milestones` array → **remove** (spec is the milestone)
- [ ] Prototype: `patina spec status <id> complete` auto-bumps version + tags
- [ ] Test on real workflow for one release cycle
