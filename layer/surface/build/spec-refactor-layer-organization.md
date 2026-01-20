---
type: refactor
id: layer-organization
status: design
created: 2026-01-20
session-origin: 20260120-065409
branch: null
tags: [sessions, specs, organization, skills-pattern]
references: [progressive-disclosure, spec-first]
---

# refactor: Layer Organization with Skill Patterns

**Problem:** Specs are hard to track (lose the thread), sessions lack useful distillation, build.md goes stale.

**Solution:** Apply skill organizational patterns to specs (not sessions). Sessions stay freeform but get better distillation.

**Why:** Skills work because of progressive disclosure and clear routing. Specs need this. Sessions are thinking spaces - constraining them would hurt.

---

## Status

**Phase:** Design
**Blocked By:** None
**Next Action:** Review spec, then Phase 1 (spec reorganization)

---

## Core Principles

1. **Sessions are messy** - That's the feature, not a bug
2. **Specs are commitments** - They need structure and tracking
3. **Git alignment** - Specs use conventional commit types
4. **Progressive disclosure** - Summary first, details on demand
5. **Self-organizing** - Status by folder location, not stale index

---

## What Changes

| Before | After |
|--------|-------|
| `build/spec-*.md` (flat) | `build/{type}/{name}/SPEC.md` (folders) |
| `build.md` (manual index) | `_active.md` + `_queue.md` (focused) |
| Session distillation (minimal) | Structured distillation section |
| Status in frontmatter only | Status by folder + frontmatter |

## What Stays

| Component | Why |
|-----------|-----|
| Session format | Freeform is the feature |
| Beliefs/rules | Working, valuable, searchable |
| Session scripts | Working well |
| Git tagging | Already integrated |

---

## Phase 1: Spec Directory Structure

Create the new spec organization.

### New Structure

```
layer/surface/build/
├── feat/                         # New capabilities
├── fix/                          # Bug fixes
├── refactor/                     # Internal improvements
├── docs/                         # Documentation work
├── chore/                        # Maintenance
│
├── _active.md                    # Currently being worked (1-3 specs)
├── _queue.md                     # Priority-ordered backlog
├── _archive/                     # Completed specs (reference)
│
└── templates/
    └── SPEC.md.tmpl              # Template for new specs
```

### Conventional Commit Types

| Type | Use For |
|------|---------|
| `feat/` | New user-facing capability |
| `fix/` | Bug fix or correction |
| `refactor/` | Internal improvement, no behavior change |
| `docs/` | Documentation only |
| `chore/` | Maintenance, CI, dependencies |
| `perf/` | Performance improvement |
| `test/` | Test additions or fixes |

### Checklist

- [ ] Create directory structure (`feat/`, `fix/`, `refactor/`, etc.)
- [ ] Create `_active.md` template
- [ ] Create `_queue.md` template
- [ ] Create `templates/SPEC.md.tmpl`
- [ ] Create `_archive/` directory

**CHECKPOINT: Review structure before migrating specs**

---

## Phase 2: SPEC.md Template

Define the standard spec format.

### Template: `templates/SPEC.md.tmpl`

```yaml
---
type: {feat|fix|refactor|docs|chore|perf|test}
id: {short-name}
status: design | ready | active | blocked | done
created: {YYYY-MM-DD}
session-origin: {session-id or null}
branch: {git-branch or null}
tags: [{relevant, tags}]
references: [{core-patterns}]
blocked-by: {spec-id or null}
---

# {type}: {Title}

**Problem:** {One sentence}

**Solution:** {One sentence}

**Why:** {Motivation in 1-2 sentences}

---

## Status

**Phase:** {Current phase name}
**Blocked By:** {What's blocking, or None}
**Next Action:** {Concrete next step}

---

## Approach

{Brief description of strategy - 2-5 sentences}

---

## Phases

### Phase 1: {Name}

{Description}

#### Checklist
- [ ] Task 1
- [ ] Task 2

**CHECKPOINT** {if human review needed}

### Phase 2: {Name}

{...}

---

## Testing

{How to verify this worked}

---

## References

- {Links to related specs, docs, code}
```

### Checklist

- [ ] Create template file
- [ ] Document optional sections (Edge Cases, Data Safety, etc.)
- [ ] Test template with one new spec

**CHECKPOINT: Review template before migration**

---

## Phase 3: Index Files

Create the tracking files that replace build.md.

### `_active.md`

```markdown
# Active Work

Specs currently being worked on. Max 3 at a time.

## In Progress

### [[refactor/layer-organization]]
- **Status:** Phase 1
- **Next:** Create directory structure
- **Session:** 20260120-065409

## Paused

{Specs started but temporarily set aside}

---

Updated: {timestamp}
```

### `_queue.md`

```markdown
# Spec Queue

Priority-ordered backlog. Top = next to work.

## High Priority

1. **[[feat/skill-derive]]** - Belief-driven skill generation
   - Blocked by: layer-organization refactor
   - Why: Enables cross-adapter skills

## Medium Priority

2. **[[refactor/scry-decomposition]]** - Break up monolithic scry
   - Why: 2000+ line file violates dependable-rust

## Low Priority / Someday

- [[docs/epistemic-guide]] - User guide for epistemic layer
- [[chore/ci-optimization]] - Speed up CI pipeline

---

Updated: {timestamp}
```

### Checklist

- [ ] Create `_active.md` with current work
- [ ] Create `_queue.md` with prioritized backlog
- [ ] Retire `build.md` (move to `_archive/` as reference)

**CHECKPOINT: Review index files**

---

## Phase 4: Migrate Existing Specs

Move existing specs to new structure.

### Migration Approach

1. Categorize each spec by conventional commit type
2. Create folder: `{type}/{spec-name}/`
3. Move spec to `SPEC.md` in folder
4. Update frontmatter to new format
5. Split large specs: summary in SPEC.md, details in design.md

### Spec Inventory (Current)

High-traffic specs to migrate first:
- [ ] `spec-skill-derive.md` → `feat/skill-derive/`
- [ ] `spec-epistemic-layer.md` → `feat/epistemic-layer/`
- [ ] `spec-repo-org-namespace.md` → `refactor/repo-org-namespace/`

Lower priority:
- [ ] Remaining `spec-*.md` files (batch migration)
- [ ] Move completed specs to `_archive/`

### Checklist

- [ ] Migrate 3 high-traffic specs
- [ ] Verify links still work (or update them)
- [ ] Batch migrate remaining specs
- [ ] Archive completed specs
- [ ] Delete old flat files

**CHECKPOINT: Verify nothing broken after migration**

---

## Phase 5: Session Distillation Enhancement

Improve what comes OUT of sessions (don't change session format).

### Current Session End

```markdown
## Session Classification
- Work Type: exploration
- Files Changed: 0
- Commits: 0
```

### Enhanced Distillation Section

```markdown
## Distillation

### Decisions Made
- {Concrete decision 1}
- {Concrete decision 2}

### Specs Spawned
- [[feat/skill-derive]] - drafted
- [[refactor/layer-organization]] - identified

### Beliefs Touched
- Reinforced: {belief-id}
- Candidate: {new belief idea}

### Key Insights
- {Quotable insight from session}

### Artifacts Created
- `layer/surface/build/spec-skill-derive.md`

### Open Questions
- {Question that needs future exploration}
```

### Implementation

Update `session-end.sh` to:
1. Prompt for decisions made
2. Prompt for specs spawned (with links)
3. Prompt for key insights
4. Auto-detect artifacts (files created this session)

### Checklist

- [ ] Design distillation prompts
- [ ] Update `session-end.sh` script
- [ ] Test with one session
- [ ] Document in session skill

**CHECKPOINT: Test enhanced distillation**

---

## Phase 6: Adapter Skill Update

Update the session/spec skills to understand new structure.

### Changes Needed

1. **session-end skill**: Add distillation guidance
2. **spec skill** (if exists): Update paths to new structure
3. **patina-spec skill**: Create if not exists (like patina-spec in spec-spec-as-skill.md)

### Checklist

- [ ] Update session-end command/skill
- [ ] Create/update spec management skill
- [ ] Test with real workflow

---

## Testing

### Verify Structure
```bash
ls -la layer/surface/build/feat/
ls -la layer/surface/build/refactor/
cat layer/surface/build/_active.md
```

### Verify Workflow
1. Create new spec using template
2. Run session with spec work
3. End session, verify distillation
4. Move spec through statuses

### Verify Nothing Broken
- [ ] Existing spec links still resolve
- [ ] Session workflow unchanged
- [ ] Beliefs/rules unaffected

---

## Success Criteria

1. **Specs findable**: `ls feat/` shows feature specs
2. **Status obvious**: Folder location = status
3. **Queue clear**: `_queue.md` shows priority order
4. **Active focused**: `_active.md` shows current 1-3 items
5. **Sessions valuable**: Distillation captures decisions and spawned specs
6. **build.md retired**: No more stale manual index

---

## Non-Goals

- Change session format (freeform is the feature)
- Automate spec status (human judgment needed)
- Complex tooling (this is file organization)
- Migrate all specs immediately (incremental is fine)

---

## References

- Session: 20260120-065409 (this conversation)
- [[spec-spec-as-skill]] - Related: spec management skill
- [[progressive-disclosure]] - Core belief driving this
- Agent Skills spec - Organizational pattern we're borrowing
- Conventional Commits - Git alignment for spec types
