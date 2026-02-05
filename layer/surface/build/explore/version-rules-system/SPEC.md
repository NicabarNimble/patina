---
type: explore
id: version-rules-system
status: draft
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

Per-spec or batched?

**Option A: Per-spec**
```bash
patina spec status my-fix complete
# → automatically bumps 0.11.0 → 0.11.1
```

**Option B: Batched release**
```bash
patina version release
# → looks at all completed specs since last release
# → determines bump: any feat? → minor. only fixes? → patch
```

Leaning toward **Option B** — allows grouping related work into one release.

### 2. What Happens to `target` Field?

**Option A: Remove entirely**
- Specs don't declare target versions
- Version is emergent

**Option B: Advisory only**
- `target: v0.12-ish` or `target: near-term`
- For rough planning, not enforced

**Option C: Keep for MAJOR milestones only**
- "This feature is required for v1.0"
- Everything else is just fix/feat flow

### 3. Multi-phase Features (Milestones)

Current: milestones with specific versions
```yaml
milestones:
  - version: "0.12.0"
    name: "Phase 1"
  - version: "0.13.0"
    name: "Phase 2"
```

Proposed: phases without versions
```yaml
phases:
  - name: "Phase 1"
    status: complete
  - name: "Phase 2"
    status: in_progress
```

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
```

Proposed:
```bash
patina version release              # auto-determine bump from completed work
patina version release --patch      # force patch (override)
patina version release --minor      # force minor (override)
patina version major "v1.0"         # deliberate major bump
```

## What We'd Remove

- `target` field from specs (or make advisory)
- `milestones` array with version numbers
- `current_milestone` tracking
- Complex milestone state machine

## What We'd Add

- Rules engine: spec type → version impact
- Release command that derives version
- Optional `phases` for multi-step features (no versions)

## Exit Criteria (for this explore)

- [ ] Decide: per-spec vs batched releases
- [ ] Decide: fate of `target` field
- [ ] Decide: fate of `milestones` array
- [ ] Prototype `patina version release` command
- [ ] Test on real workflow for one release cycle
