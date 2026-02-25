---
type: feat
id: git-tag-system
status: draft
created: 2026-02-15
sessions:
  origin: 20260215-083121
related:
- layer/core/patina-identity.md
- src/commands/scrape/git/mod.rs
- src/commands/assay/mod.rs
- src/commands/spec/internal/queries.rs
beliefs:
- beliefs-are-the-product
- git-tags-as-knowledge-refs
- git-is-the-knowledge-substrate
- knowledge-diff-is-a-command-not-a-substrate
- plugins-are-three-prong-bundles
exit_criteria: []
---

# feat: Git Tag System — Tag-Aware Database and Knowledge Diff

> Build `patina diff` to compare knowledge between any two tags, and enhance
> tag classification in scrape + assay.

## Current State (updated 2026-02-25)

### What exists now

- **git_tags table** — `scrape` indexes all tags into `git_tags` (tag_name, sha, tag_date, tagger_name, message). Full clear + rebuild each scrape.
- **Session tag parsing** — `parse_session_tags()` links commits to sessions via tag boundaries.
- **spec-history command** (v0.32.0) — `patina spec history <id>` reconstructs spec lifecycle from tags. Uses `tag_suffix_to_state()` for classification (active, paused, blocked, archived, split). This is in-memory per-query, not persisted in DB.
- **Spec workflow tags stable** — tag conventions settled via [[spec-workflow-rigor]]: `-start`, `-paused-N`, `-resumed-N`, `-blocked-N`, `-vN-complete`, archive (exact match).

### What's NOT indexed

- Tag **category in DB** — `tag_suffix_to_state()` exists for spec tags but isn't applied during scrape. No classification for session/version/archive tags.
- Tag **relationships** — no linking between start/end session tag pairs
- **Diff between tags** — no way to compute what changed between two knowledge states

## What To Build

### Phase A: Tag Classification in Scrape

Add `tag_type` column to `git_tags` and classify during `insert_tags()`:

```sql
ALTER TABLE git_tags ADD COLUMN tag_type TEXT;
```

Classification rules:
- `session-*-start` → `session-start`
- `session-*-end` → `session-end`
- `v[0-9]*` → `version`
- `spec/*-start` → `spec-start`
- `spec/*-paused-*` → `spec-paused`
- `spec/*-resumed-*` → `spec-resumed`
- `spec/*-blocked-*` → `spec-blocked`
- `spec/*-v*-complete` → `spec-split`
- `spec/*` (no suffix) → `spec-archive`
- `archive/*` → `archive`
- Everything else → `other`

**Note:** Reuse the classification pattern from `tag_suffix_to_state()` in `src/commands/spec/internal/queries.rs` for spec tags. Generalize for all tag categories.

Add assay query support: `patina assay --query-type tags` with optional `--pattern` glob filter.

**Code path:** `src/commands/scrape/git/mod.rs` (insert_tags), `src/commands/assay/` (new tags query type).

### Phase B: Knowledge Diff Command

`patina diff <tag1> <tag2>` compares the knowledge state between two git refs.

Uses git subprocess calls (consistent with codebase — no git2 crate):

1. **Belief diff** — compare `layer/surface/epistemic/beliefs/` at both refs.
   Parse YAML frontmatter. Report: added, removed, modified (entrenchment change, evidence added).

2. **Pattern diff** — compare `layer/surface/` and `layer/core/` at both refs.
   Report: added, removed, modified patterns/specs.

3. **Session diff** — count sessions between the two tags.

4. **Commit summary** — commit count and file stats between refs.

Output format:
```
patina diff v0.30.0 v0.32.0

Beliefs: +3 added, -0 removed, ~1 modified
  + git-commits-are-fault-tolerance (medium)
  + multi-expert-convergence-is-signal (medium)
  + adding-type-is-not-migrating-model (medium)
  ~ boundary-string-internal-enum: evidence added

Specs: 4 completed, 1 archived
  ✓ spec-next-typed (v0.31.12)
  ✓ spec-scan-efficiency (v0.31.13)
  ✓ spec-query-filesystem-truth (v0.31.14)
  ✓ spec-history (v0.32.0)

Sessions: 8 sessions between tags

Commits: 40 commits
```

Expose as CLI (`patina diff`), `--json`, and MCP tool.

**Code path:** New `src/commands/diff/mod.rs`.

### Phase C: Tag Pairs (optional, defer if not needed)

Link session start/end tags as pairs in DB. Enable `patina diff --session <id>`.
Session system already tracks this via session metadata — evaluate whether DB pairs add value beyond what `patina session` already provides.

## Exit Criteria

- [ ] `git_tags.tag_type` column populated during scrape with correct classification
- [ ] `patina assay --query-type tags` lists tags with type filter
- [ ] `patina diff <tag1> <tag2>` shows belief/pattern/session diff between tags
- [ ] `--json` output for MCP

## Non-Goals

- Content-addressed knowledge objects
- Custom tag creation (tags created by session/spec/release commands)
- Real-time tag watching (scrape is batch)
- Per-spec lifecycle history (already solved by `patina spec history`)

## Revision Log

- 2026-02-15: Created — original 3-phase design
- 2026-02-23: Alignment audit — blocked on spec-workflow-rigor for tag conventions
- 2026-02-25: Updated — cleared blocker (workflow-rigor shipped), trimmed Phase A to avoid overlap with spec-history's `tag_suffix_to_state()`, made Phase C optional, updated examples to reflect current state (v0.32.0, 159 beliefs)
