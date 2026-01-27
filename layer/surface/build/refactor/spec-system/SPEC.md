---
type: refactor
id: spec-system
status: in_progress
created: 2026-01-22
updated: 2026-01-26
sessions:
  origin: 20260122-083510
  work:
    - 20260126-060540
related:
  - layer/surface/build/spec-epistemic-layer.md
  - layer/surface/build/feat/surface-layer/SPEC.md
  - layer/surface/build/feat/go-public/SPEC.md
---

# refactor: Spec System

> Specs should be trackable, modifiable, and actionable - not aspirational documents that lie.

**Problem:** Current specs are inconsistent. Some have unchecked checklists for work already done. Status fields lie. No provenance to sessions. Hard to know what's real.

**Solution:** One spec format. Folder structure. Machine-parseable frontmatter. Append-only status log. Session links.

---

## Exit Criteria

- [x] Spec format documented (this file)
- [x] Milestone format documented (this file)
- [ ] Existing specs migrated or archived
- [ ] `patina scrape layer` extracts milestones from specs
- [ ] `patina version milestone` reads from scraped index
- [ ] No specs with status that contradicts reality

---

## The Format

### When to Create a Spec

**Create a spec when:**
- Work requires design thinking
- Multiple sessions will touch it
- You need to track progress over time
- Others need to understand the plan

**Don't create a spec when:**
- Simple bug fix (commit message is enough)
- One-liner change
- Already covered by existing spec

### Folder Structure

All specs use folder structure:

```
layer/surface/build/
├── feat/           # New capabilities
├── refactor/       # Restructuring existing code
├── fix/            # Bug fixes needing design (rare)
├── explore/        # Research, uncertain outcome
└── deferred/       # Parked work (can be flat files)
```

Each spec is a folder with at minimum `SPEC.md`:

```
feat/surface-layer/
├── SPEC.md         # Required: what, why, exit criteria
└── design.md       # Optional: how, architecture, details
```

### SPEC.md Template

```yaml
---
type: feat | refactor | fix | explore
id: kebab-case-name
status: design | ready | in_progress | complete | archived
created: YYYY-MM-DD
updated: YYYY-MM-DD
sessions:
  origin: YYYYMMDD-HHMMSS
  work: [session-ids]
related:
  - path/to/related/spec
# Optional: milestones for version-linked work
milestones:
  - version: "0.x.y"
    name: Short description
    status: pending | in_progress | complete
current_milestone: "0.x.y"
---

# type: Title

> One sentence: what problem does this solve?

---

## Exit Criteria

- [ ] Criterion 1
- [ ] Criterion 2
- [ ] Criterion 3

---

## Milestones

Optional section for specs with version-linked outcomes.

| Version | Name | Status |
|---------|------|--------|
| 0.x.y | Milestone description | → in_progress |
| 0.x.z | Next milestone | ○ pending |

---

## Status Log

| Date | Status | Note |
|------|--------|------|
| YYYY-MM-DD | design | Initial spec |

---

## Quick Reference

Optional section for commands, key decisions, anything needed at a glance.

---

## See Also

- [[design.md]] if exists
- Related specs
```

### Status Enum

| Status | Meaning |
|--------|---------|
| `design` | Thinking, not ready to build |
| `ready` | Spec complete, can start anytime |
| `in_progress` | Actively being built |
| `complete` | All exit criteria met |
| `archived` | Superseded, abandoned, or done and tagged |

### Milestones (Optional)

Milestones connect specs to version bumps. Use them when:
- Work spans multiple version releases
- You want `patina version milestone` to link to spec progress
- The spec represents a "phase" of development (like go-public)

**Don't use milestones when:**
- Single version bump completes the spec
- Exploration/research with uncertain outcome
- Quick fix that doesn't warrant version tracking

**Milestone Status Enum:**

| Status | Meaning |
|--------|---------|
| `pending` | Not started |
| `in_progress` | Current focus |
| `complete` | Version bumped, done |

**Milestone Rules:**

1. **One `in_progress` at a time** - Focus. Complete current before starting next.
2. **Version must be sequential** - 0.8.2 → 0.8.3, no skipping
3. **Completing = version bump** - When milestone status → complete, `patina version milestone` bumps version
4. **Exit criteria per milestone** - Each milestone should have clear criteria in the spec body
5. **Scraped into index** - `patina scrape layer` extracts milestones for fast lookup

### Rules

1. **Status must match reality** - If exit criteria are met, status is `complete`
2. **Exit criteria are checkboxes** - Not prose. Checkable.
3. **Status log is append-only** - Every change gets a row
4. **Sessions link to provenance** - Know where thinking happened
5. **design.md is optional** - Only for complex work needing architecture docs

---

## Design Decisions

### One Format, Not Many

Previous discussion considered separate formats for "simple" vs "complex" work. Rejected.

**Rationale:**
- Complexity in deciding which format to use
- Simple work that needs a spec isn't that simple
- If it's truly simple, don't create a spec at all

### Folder Structure for Everything

Even small specs get folders. The folder IS the identity.

**Rationale:**
- Consistent structure
- Room to grow (add design.md later)
- Clear namespace (`feat/surface-layer/` not `spec-surface-layer.md`)

### Status Log Over Git Archaeology

Could rely on git history for status changes. Chose explicit log instead.

**Rationale:**
- Visible in the file itself
- Don't need tooling to see history
- Can add notes explaining why status changed

### Sessions as Provenance

Link to sessions where thinking happened.

**Rationale:**
- Context is recoverable
- Know WHO (session) decided WHAT (spec)
- Supports the epistemic layer (decisions have evidence)

---

## Migration Plan

### Phase 1: New Specs Use New Format

All new specs follow this format. Don't migrate old specs yet.

### Phase 2: Triage Existing Specs

For each existing spec in `layer/surface/build/`:

| Current State | Action |
|---------------|--------|
| Implemented but says "active" | Archive with git tag, delete file |
| Superseded by another spec | Archive, add redirect note |
| Still valid, in progress | Migrate to folder format |
| Design doc, not actionable | Move to `explore/` or `deferred/` |
| Stale, abandoned | Archive or delete |

### Phase 3: Tooling

Add `patina report specs` to parse frontmatter and show:
- Specs by status
- Stale specs (in_progress but not updated in 30 days)
- Specs without exit criteria

---

## Status Log

| Date | Status | Note |
|------|--------|------|
| 2026-01-22 | in_progress | Initial spec created during session |
| 2026-01-26 | in_progress | Added milestone format for version linkage |

---

## See Also

- [[design.md]] - Extended rationale and skills research context
- [[spec-epistemic-layer.md]] - Example of well-structured spec
- [[feat/surface-layer/SPEC.md]] - Uses folder pattern
