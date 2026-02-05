---
type: fix
id: spec-list-filter
status: ready
created: 2026-02-05
sessions:
  origin: 20260205-163242
related:
  - layer/surface/build/refactor/spec-system/SPEC.md
  - layer/surface/build/feat/spec-as-work-item/SPEC.md
beliefs:
  - spec-is-milestone
---

# fix: Spec List Includes Non-Spec Patterns

> `patina spec list` shows sub-docs, reference docs, and standalone files that aren't specs.

---

## Problem

`patina spec list` queries `WHERE file_path LIKE 'layer/surface/build/%'` — this catches **every** markdown file the layer scraper indexed under `build/`, not just specs.

Current pollution (10 non-spec entries out of 60):

| ID | File | Why it's not a spec |
|----|------|---------------------|
| `core` | `explore/cli-commands/core.md` | Sub-doc of cli-commands |
| `dev` | `explore/cli-commands/dev.md` | Sub-doc of cli-commands |
| `infra` | `explore/cli-commands/infra.md` | Sub-doc of cli-commands |
| `science` | `explore/cli-commands/science.md` | Sub-doc of cli-commands |
| `design` | `refactor/spec-system/design.md` | Sub-doc of spec-system |
| `research` | `explore/clawdbot-patterns/research.md` | Sub-doc of clawdbot-patterns |
| `rust-house-style` | `build/rust-house-style.md` | Standalone style guide |
| `spec-architectural-alignment` | `reference/spec-architectural-alignment.md` | Reference doc |
| `spec-assay` | `reference/spec-assay.md` | Reference doc |
| `spec-pipeline` | `reference/spec-pipeline.md` | Reference doc |

The scraper is correct — it indexes all layer knowledge for `scry` and `context`. The bug is in the spec query logic.

---

## Root Cause

`src/commands/spec/internal.rs` uses `file_path LIKE 'layer/surface/build/%'` in three queries:
- `get_all_specs()` (line ~384)
- `get_ready_specs()` (line ~157)
- `get_blocked_specs()` (line ~265)

This path filter alone cannot distinguish specs from sub-docs that live in the same directory tree.

---

## Fix

Add a second filter: **require the pattern to have a `status` field**. All real specs have status (or should after triage). Sub-docs and reference docs don't have status in their frontmatter.

```sql
-- Before:
WHERE p.file_path LIKE 'layer/surface/build/%'

-- After:
WHERE p.file_path LIKE 'layer/surface/build/%'
  AND p.status IS NOT NULL
```

This works because:
- All spec SPEC.md files have `status:` in YAML frontmatter
- All deferred/ standalone specs now have `status: deferred` (added in this triage)
- Sub-docs (design.md, research.md, core.md) have no frontmatter → status is NULL
- Reference docs have no status field → NULL
- `rust-house-style.md` has no frontmatter → NULL

The `status IS NOT NULL` filter is a minimal, correct fix that leverages the convention already established by the spec-system format.

---

## Exit Criteria

- [ ] `get_all_specs()` excludes patterns without status
- [ ] `get_ready_specs()` excludes patterns without status
- [ ] `get_blocked_specs()` excludes patterns without status
- [ ] `patina spec list` shows only real specs (no sub-docs, no reference docs)
- [ ] Sub-docs and reference docs remain searchable via `patina scry`
- [ ] `patina scrape layer` unchanged (scraper is not the problem)

---

## Files Changed

| File | Change |
|------|--------|
| `src/commands/spec/internal.rs` | Add `AND p.status IS NOT NULL` to 3 queries |

**~3 lines changed.**
