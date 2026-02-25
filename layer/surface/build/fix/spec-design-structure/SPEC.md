---
type: fix
id: spec-design-structure
status: draft
created: 2026-02-25
sessions:
  origin: 20260224-212321
beliefs:
  - spec-carries-progress
  - design-docs-are-implementation-cache
---
# fix: Formalize DESIGN.md as spec companion

> DESIGN.md is convention not structure; when present execution is mechanical, when absent LLM re-derives

## Problem

Session 20260224-202650 showed the difference: spec-create had a
DESIGN.md with 4 planned commits — "4 commits landed almost
mechanically." But DESIGN.md is pure convention. No command creates it,
no tooling validates it exists, and most specs lack one. Without it, the
LLM re-derives the execution plan from scratch each session.

The belief [[design-docs-are-implementation-cache]] captures this: the
DESIGN.md is what turns a spec from a contract into an execution plan.

## Root Cause

`spec create` scaffolds `SPEC.md` in the spec directory but makes no
provision for DESIGN.md. The `/spec` skill's create workflow guides the
LLM to fill in SPEC.md sections but doesn't mention DESIGN.md. There's
no convention for when a spec needs one vs. when it doesn't.

## Fix

1. Add DESIGN.md scaffold to `spec create` for non-trivial types:
   - `feat` and `refactor` — always scaffold DESIGN.md
   - `fix` — scaffold if description suggests multi-file change
   - `explore` — skip (exploratory by nature)

2. DESIGN.md template:
   ```markdown
   # Design: <spec-title>

   ## Approach

   ## Commits
   1. `commit message` — what and why

   ## Key Files
   - `path/to/file.rs` — role

   ## Open Questions
   ```

3. Update `/spec` skill to prompt LLM to fill DESIGN.md after SPEC.md

4. `spec.show` (if spec-show-mcp lands first) includes DESIGN.md content

## Key Files

```
src/commands/spec/internal/create.rs  — scaffold logic, body templates
resources/claude/spec.md              — /spec skill template
```

## Exit Criteria

- [ ] `spec create feat` scaffolds both SPEC.md and DESIGN.md
- [ ] `spec create refactor` scaffolds both SPEC.md and DESIGN.md
- [ ] `spec create fix` scaffolds SPEC.md only (DESIGN.md optional)
- [ ] `spec create explore` scaffolds SPEC.md only
- [ ] `/spec` skill guides LLM to fill DESIGN.md when present
