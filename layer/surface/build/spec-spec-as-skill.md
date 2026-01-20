---
id: spec-spec-as-skill
status: design
created: 2026-01-19
tags: [spec, meta, skills, process]
references: [unix-philosophy, spec-skills-universal]
---

# Spec: Spec as Skill

**Problem:** Specs are inconsistent, manually managed, and not easily executable by LLMs.

**Solution:** Define spec format as an evolvable template, use a skill to guide spec creation and execution.

**End goal:** Uniform specs that LLMs can reliably create and execute with human checkpoints.

---

## Current Pain

1. **Inconsistent format** - Each spec structured differently
2. **Manual curation** - build.md manually lists specs
3. **No creation guidance** - LLM guesses at structure each time
4. **Execution ambiguity** - Unclear when LLM should pause for human
5. **Status tracking** - Ad-hoc, often stale

## Design Principles

1. **Evolvable** - Format lives in editable document, not code
2. **LLM-executable** - Clear phases, checkpoints, tasks
3. **Human-in-loop** - Explicit pause points for review
4. **Minimal** - Start simple, add structure when pain demands

---

## Architecture

```
layer/core/spec-template.md          # Defines current spec format (evolvable)
                 │
                 ▼
.claude/skills/patina-spec/SKILL.md  # Reads template, guides creation/execution
                 │
                 ▼
layer/surface/build/spec-*.md        # Individual specs (uniform format)
```

**Key insight:** The skill doesn't hardcode format - it reads the template. Update template, skill adapts.

---

## Phase 1: Spec Template

Create `layer/core/spec-template.md` that defines:

### Required Frontmatter

```yaml
---
id: spec-{name}              # Lowercase, hyphenated
status: design | ready | in-progress | complete | blocked
created: YYYY-MM-DD
tags: [relevant, tags]
references: [core-patterns]  # Links to layer/core/*.md
---
```

### Required Sections

```markdown
# Spec: {Title}

**Problem:** One sentence.

**Solution:** One sentence.

---

## Approach

Brief description of strategy. Design decisions.

---

## Phase N: {Phase Name}

### Changes/Steps

Description of what to do.

### Checklist

- [ ] Task 1
- [ ] Task 2

**CHECKPOINT** (if human review needed before next phase)

---

## Testing

How to verify the spec worked.
```

### Optional Sections

- `## Edge Cases` - Known edge cases to handle
- `## Data Safety` - For migrations, what's preserved
- `## References` - Links to code, docs, external resources

**CHECKPOINT: Review template before building skill**

---

## Phase 2: Spec Skill

Create `.claude/skills/patina-spec/SKILL.md`:

```yaml
---
name: patina-spec
description: |
  Guide for creating and executing specs in Patina. Use when:
  - User says "create a spec", "let's spec this", "make a spec for X"
  - Starting work on an existing spec
  - Reviewing spec status
  Specs capture phased work with checkpoints for human review.
---

# Spec Management

## Creating a Spec

1. Read `layer/core/spec-template.md` for current format
2. Gather from user:
   - Problem (one sentence)
   - Solution approach (one sentence)
   - Phases needed
   - Where checkpoints belong
3. Create `layer/surface/build/spec-{name}.md`
4. Add to build.md active specs list (if significant)

## Executing a Spec

1. Read the spec file
2. Create todos from checklist
3. Work through phases sequentially
4. STOP at **CHECKPOINT** markers - ask user to review
5. Mark tasks complete as you go
6. Update spec status when done

## Spec Statuses

| Status | Meaning |
|--------|---------|
| design | Still being shaped, not ready to execute |
| ready | Approved, ready to implement |
| in-progress | Currently being worked on |
| blocked | Waiting on external dependency |
| complete | Done, may archive |

## Finding Specs

- Active: `layer/surface/build/spec-*.md`
- Deferred: `layer/surface/build/deferred/spec-*.md`
- Overview: `layer/core/build.md`
```

**CHECKPOINT: Review skill before using**

---

## Phase 3: Migrate Existing Specs

Update existing specs to match template format:

### High Priority (Active)
- [ ] `spec-repo-org-namespace.md` - Already close to format
- [ ] `spec-skills-focused-adapter.md` - Needs restructure
- [ ] `spec-surface-layer.md` - Needs restructure

### Lower Priority
- Defer migration of specs in `deferred/` folder
- Archive specs remain as-is (historical)

**CHECKPOINT: Confirm migration approach before bulk changes**

---

## Phase 4: Iterate

As we use the system:

1. **Notice friction** - What's hard? What's unclear?
2. **Update template** - Add/remove/clarify sections
3. **Skill adapts** - Reads fresh template each time
4. **Capture learnings** - Create beliefs about what works

No code changes needed to evolve the format.

---

## What This Is NOT

- **Not a spec registry CLI** - Discovery stays manual (build.md)
- **Not automated status tracking** - Human updates status
- **Not rigid** - Template is guidance, not enforcement
- **Not permanent** - Format will evolve as we learn

Start minimal. Add structure only when pain demands.

---

## Checklist

### Phase 1: Template
- [ ] Create `layer/core/spec-template.md`
- [ ] Include required frontmatter definition
- [ ] Include required sections with examples
- [ ] List optional sections

**CHECKPOINT: Review template**

### Phase 2: Skill
- [ ] Create `.claude/skills/patina-spec/SKILL.md`
- [ ] Skill reads template (not hardcoded format)
- [ ] Include creation guidance
- [ ] Include execution guidance
- [ ] Include checkpoint behavior

**CHECKPOINT: Test skill with one spec creation**

### Phase 3: Migration
- [ ] Update `spec-repo-org-namespace.md` to match
- [ ] Update other active specs as touched
- [ ] Leave deferred/archived as-is

### Phase 4: Iterate
- [ ] Use for 2-3 specs
- [ ] Note friction points
- [ ] Update template as needed
- [ ] Capture beliefs about spec design

---

## Success Criteria

1. New specs follow consistent format
2. LLM can create specs without guessing structure
3. LLM knows when to pause for human review
4. Format can evolve without code changes
5. Existing workflow (manual curation in build.md) unchanged
