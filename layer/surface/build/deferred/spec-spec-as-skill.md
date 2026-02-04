---
id: spec-spec-as-skill
status: design
created: 2026-01-19
updated: 2026-01-20
tags: [spec, meta, skills, process, architecture]
references: [unix-philosophy, progressive-disclosure]
inspiration: huggingface/skills
---

# Spec: Specs as Skills

**Problem:** Specs are flat files that lose thread, have inconsistent format, and rely on manual build.md curation.

**Solution:** Apply skill architecture to specs - folder-based routing, progressive disclosure (SPEC.md + design.md), conventional commit alignment.

**End goal:** Specs are self-routing, discoverable by structure, with clear entry points and deep details separated.

---

## Key Insight

> Specs ARE skills for development work.

The HuggingFace skills repo (`huggingface/skills`) demonstrates the pattern:

| Skill Component | Spec Equivalent |
|-----------------|-----------------|
| `skills/hf-trainer/` folder | `feat/mothership-graph/` folder |
| `SKILL.md` (entry point) | `SPEC.md` (summary + status) |
| `description` (routing) | Folder path provides routing |
| `references/` (deep docs) | `design.md` (phases, details) |
| `scripts/` (executable) | `scripts/` (migration, tests) |
| `agents/AGENTS.md` (index) | `_active.md`, `_queue.md` |

**The reframe**: Don't create a skill to manage specs. Make specs follow skill structure.

---

## Current Pain

1. **Flat files lose thread** - `spec-*.md` disconnected from related docs
2. **No routing** - Must read file to know what type of work it is
3. **Giant files** - Summary and details mixed, hard to scan
4. **Manual curation** - build.md manually maintained
5. **No conventional commit alignment** - Specs don't map to commit types

---

## Architecture

```
layer/surface/build/
├── feat/                            # New capabilities
│   └── {name}/
│       ├── SPEC.md                  # Entry point (summary + status)
│       ├── design.md                # Deep details (phases, decisions)
│       └── scripts/                 # Optional: migration, test scripts
│
├── fix/                             # Bug fixes
│   └── {name}/
│       └── SPEC.md                  # Often single file (simple fixes)
│
├── refactor/                        # Internal improvements
├── docs/                            # Documentation work
├── chore/                           # Maintenance tasks
│
├── _active.md                       # Currently in progress
├── _queue.md                        # Priority order (next up)
└── _archive/                        # Completed specs (or git tags)
```

**Routing is structural**: Folder path tells you work type. No description-based routing needed.

---

## SPEC.md Format (Entry Point)

Like SKILL.md, this is the scannable summary:

```markdown
---
type: feat                           # Matches folder (feat, fix, refactor, docs, chore)
id: mothership-graph
status: design | ready | active | blocked | done
phase: G1                            # Current phase (if phased work)
created: 2026-01-05
session-origin: 20260105-131238      # Where this came from
---

# feat: Mothership Graph

**Problem:** Cross-project knowledge is siloed.

**Solution:** SQLite graph connecting projects via semantic edges.

**Next:** Implement edge traversal (G2)

---

## Quick Reference

[Key commands, patterns, or decisions - scannable]

---

## Status

- **Phase:** G1 (Schema) complete, G2 (Traversal) next
- **Blocked By:** None
- **Recent:** Completed baseline measurement (G0)

---

See [[design.md]] for phases, checklists, and implementation details.
```

**~50-100 lines max.** Everything else goes in design.md.

---

## design.md Format (Deep Details)

Like `references/` in a skill - the full context:

```markdown
# Design: Mothership Graph

## Approach

[Strategy, key decisions, tradeoffs]

---

## Phase G0: Measurement (Complete)

### Goal
[What this phase achieves]

### Checklist
- [x] Task 1
- [x] Task 2

### Outcome
[What we learned]

---

## Phase G1: Schema (Active)

### Goal
### Checklist
- [ ] Task 1
- [ ] Task 2

**CHECKPOINT: Review before G2**

---

## Phase G2: Traversal (Planned)

[...]

---

## Edge Cases

## Testing

## References
```

---

## Index Files

### _active.md

What's being worked NOW (replaces build.md active section):

```markdown
# Active Specs

## In Progress

| Spec | Phase | Owner | Since |
|------|-------|-------|-------|
| [[feat/mothership-graph]] | G1 | - | 2026-01-05 |
| [[fix/scry-ranking]] | - | - | 2026-01-18 |

## Blocked

| Spec | Blocked By |
|------|------------|
| [[feat/cross-project]] | mothership-graph G2 |
```

### _queue.md

Priority order for what's next:

```markdown
# Spec Queue

1. [[feat/mothership-graph]] - Core infrastructure
2. [[refactor/layer-organization]] - This spec!
3. [[fix/adapter-refresh]] - User-facing bug
4. [[docs/epistemic-guide]] - Onboarding
```

---

## The Skill (Helper, Not Router)

The `patina-spec` skill helps create/execute specs but doesn't route to them:

```yaml
---
name: patina-spec
description: |
  Create and execute specs in Patina. Use when:
  - User says "create a spec", "spec this out"
  - Starting work on an existing spec
  - Reviewing what's active/queued
  Specs follow skill architecture: SPEC.md summary + design.md details.
---

# Spec Management

## Creating a Spec

1. Determine type: feat, fix, refactor, docs, chore
2. Create folder: `layer/surface/build/{type}/{name}/`
3. Create SPEC.md with frontmatter and summary
4. Create design.md if phased work (otherwise SPEC.md is enough)
5. Add to _queue.md or _active.md

## Executing a Spec

1. Read SPEC.md for context and current phase
2. Read design.md for detailed checklist
3. Create todos from checklist
4. Work through phases sequentially
5. STOP at **CHECKPOINT** markers
6. Update SPEC.md status and phase as you progress
7. Move to _archive/ when done (or tag and delete)

## Finding Specs

- Active: `layer/surface/build/_active.md`
- Queue: `layer/surface/build/_queue.md`
- By type: `layer/surface/build/{feat,fix,refactor,docs,chore}/`
- Archive: `layer/surface/build/_archive/` or git tags
```

---

## Migration Strategy

### Phase 1: Create Structure

- [ ] Create `feat/`, `fix/`, `refactor/`, `docs/`, `chore/` folders
- [ ] Create `_active.md` and `_queue.md`
- [ ] Create `_archive/` folder

**CHECKPOINT: Review structure before migrating specs**

### Phase 2: Migrate Active Specs

Convert current `spec-*.md` files to new structure:

| Current | New Location | Notes |
|---------|--------------|-------|
| `spec-mothership-graph.md` | `feat/mothership-graph/` | Split to SPEC.md + design.md |
| `spec-epistemic-layer.md` | `feat/epistemic-layer/` | Large, needs split |
| `spec-repo-org-namespace.md` | `fix/repo-namespace/` | Done, archive |
| `spec-skill-derive.md` | Sunset | Superseded by this spec |

- [ ] Migrate 2-3 specs as test
- [ ] Validate structure works
- [ ] Migrate remaining active specs

**CHECKPOINT: Review migrations before bulk move**

### Phase 3: Update Tooling

- [ ] Create `patina-spec` skill
- [ ] Update build.md to point to new structure
- [ ] Consider: auto-generate _active.md from frontmatter?

### Phase 4: Iterate

- [ ] Use for 2-3 new specs
- [ ] Note friction points
- [ ] Capture learnings as beliefs

---

## What This Changes

| Before | After |
|--------|-------|
| `spec-*.md` flat files | `{type}/{name}/SPEC.md` folders |
| Manual build.md curation | `_active.md`, `_queue.md` |
| Giant spec files | SPEC.md (summary) + design.md (details) |
| No work type signal | Folder = conventional commit type |
| Specs disconnected | Folder groups related files |

## What Stays

| Component | Why |
|-----------|-----|
| CHECKPOINTs | Human-in-loop is valuable |
| Phase structure | Phased work with checklists works |
| Status tracking | Frontmatter status is useful |
| Git as memory | Tags for completed specs |

---

## Success Criteria

1. **Structural routing** - Folder path tells you work type
2. **Progressive disclosure** - SPEC.md scannable, design.md detailed
3. **Discoverable** - _active.md and _queue.md replace manual curation
4. **Conventional commit aligned** - feat/, fix/, refactor/ match commit prefixes
5. **Skill helps, doesn't route** - patina-spec creates structure, doesn't own discovery

---

## References

- `huggingface/skills` - Reference implementation of skill architecture
- `anthropics/skills` - Claude Code skills standard
- Session 20260120-165543 - Discussion capturing this insight
