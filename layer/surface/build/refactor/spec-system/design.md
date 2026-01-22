# Spec System - Design Document

Extended rationale and research context for [[SPEC.md]].

---

## Origin: Skills Pattern Analysis

This design emerged from analyzing skills systems across multiple codebases:

### Research Sources (in ref repos)

| Source | Key Learning |
|--------|--------------|
| `anthropics/skills` | Folder structure with SKILL.md + scripts/ + references/ |
| `anthropics/claude-code` | Progressive disclosure: metadata -> main doc -> resources |
| `huggingface/*` | Convention over configuration |

### Sessions That Shaped This

| Session | Contribution |
|---------|--------------|
| 20260116-095954 | Skills research, adapter extensibility |
| 20260117-072948 | Skills testing, progressive disclosure validation |
| 20260121-102727 | Spec review, identified lying specs problem |
| 20260122-083510 | This spec created |

---

## Why Folder Structure

The skills pattern uses:

```
.claude/skills/skill-name/
├── SKILL.md           # Main entry point
├── scripts/           # Executable code
├── references/        # Supporting docs
└── assets/            # Output files
```

Adapted for specs:

```
layer/surface/build/feat/feature-name/
├── SPEC.md            # Main entry point (what, why, exit criteria)
└── design.md          # Supporting details (how, architecture)
```

**Why this works:**
1. **Clear identity** - The folder name IS the spec name
2. **Room to grow** - Start with SPEC.md, add design.md when needed
3. **Isolation** - Each spec is self-contained
4. **Discoverable** - `ls feat/` shows all features in progress

---

## Why Not Flat Files

Previous system used flat files:

```
layer/surface/build/
├── spec-surface-layer.md
├── spec-bootstrap-markers.md
├── spec-mothership.md
└── ... 36 files
```

**Problems found:**
1. **Naming collision** - `spec-` prefix everywhere, hard to scan
2. **No hierarchy** - Can't tell feat from fix from exploration
3. **Grows unbounded** - 36 files in one directory
4. **Checklists lie** - Unchecked boxes for completed work (spec-bootstrap-markers)

---

## Status Field Design

### Considered: Free Text

```yaml
status: "In progress, Phase 2 complete"
```

**Rejected:** Not machine-parseable. Can't aggregate status across specs.

### Considered: Boolean `done: true/false`

```yaml
done: false
```

**Rejected:** Too coarse. Can't distinguish "thinking" from "ready to build" from "actively building."

### Chosen: Enum

```yaml
status: design | ready | in_progress | complete | archived
```

**Why:**
- Machine-parseable
- Covers the lifecycle
- Clear transitions
- Can build tooling (`patina report specs --status=in_progress`)

---

## Exit Criteria Design

### Considered: Prose Description

```markdown
## Done When

The feature is complete when users can capture beliefs and the system validates them.
```

**Rejected:** Subjective. "Complete" means different things to different people.

### Chosen: Checkboxes

```markdown
## Exit Criteria

- [ ] `patina surface capture` command exists
- [ ] Capture precision >70%
- [ ] All tests pass
```

**Why:**
- Binary: done or not done
- Greppable: `grep -c "\- \[ \]"` counts remaining work
- Visible: Progress is obvious
- Accountable: Hard to claim "done" with unchecked boxes

---

## Status Log Design

### Considered: Rely on Git History

Git already tracks changes. Why duplicate?

**Rejected:**
- Requires tooling to see history
- Can't add explanatory notes
- Not visible in the document itself

### Chosen: Append-Only Table

```markdown
## Status Log

| Date | Status | Note |
|------|--------|------|
| 2026-01-08 | design | Initial spec |
| 2026-01-15 | ready | Exit criteria defined |
| 2026-01-20 | in_progress | Started Phase 1 |
```

**Why:**
- Self-documenting
- Visible without tooling
- Notes explain WHY status changed
- Git still provides backup/verification

---

## Session Links Design

### The Problem

Specs reference decisions made in sessions. Without links, context is lost.

Example: "We chose SQLite over Postgres" - but WHY? WHERE was this decided?

### The Solution

```yaml
sessions:
  origin: 20260108-124107      # Where spec started
  work: [20260115-121358]      # Sessions that modified it
```

**Why two fields:**
- `origin` - THE session where this spec was born
- `work` - Sessions that touched it (append as you go)

**Enables:**
- Click through to read original discussion
- Reconstruct thinking when confused
- Supports epistemic layer (decisions have evidence)

---

## Type Categories

| Type | Purpose | Example |
|------|---------|---------|
| `feat` | New capability | surface-layer, federated-query |
| `refactor` | Restructure existing | spec-system, remove-codex |
| `fix` | Bug fix needing design | bootstrap-markers (rare) |
| `explore` | Research, uncertain outcome | agents-and-yolo |

**Why these four:**
- Cover the work types we actually do
- Match git commit conventions loosely
- `explore` is special: outcome unknown, might become feat or get abandoned

---

## Deferred Items

Parked work doesn't need full spec structure. Flat file in `deferred/` is fine:

```
deferred/
├── retrieval-optimization.md
├── persona-fusion.md
└── github-adapter.md
```

**Why:**
- Not actively tracking progress
- Just need to remember the idea
- Can promote to full spec when work resumes

---

## Relationship to Other Systems

### Epistemic Layer (Beliefs)

Specs reference beliefs, beliefs reference specs.

```markdown
# In spec
## See Also
- [[beliefs/spec-first.md]] - Informs this design

# In belief
## Applied-In
- [[feat/surface-layer/SPEC.md]]
```

### Sessions

Specs link to sessions for provenance. Sessions may create specs.

```
Session: "We should refactor the spec system"
         ↓ creates
Spec: refactor/spec-system/SPEC.md
         ↓ references
Sessions: origin: 20260122-083510
```

### Reports

Measurements proving specs worked go in `layer/surface/reports/`:

```markdown
## Exit Criteria

- [ ] Capture precision >70% (see [[reports/surface-layer-eval-2026-01.md]])
```

---

## Future: Tooling

Eventually want:

```bash
# Show all specs by status
patina report specs

# Show stale specs (in_progress but not updated in 30 days)
patina report specs --stale

# Validate spec format
patina surface validate build/feat/*/SPEC.md
```

Not building this now. Manual process first, automate when patterns stabilize.
