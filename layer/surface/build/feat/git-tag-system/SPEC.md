---
type: feat
id: git-tag-system
status: draft
created: 2026-02-15
sessions:
  origin: 20260215-083121
related:
- layer/core/patina-identity.md
beliefs:
- beliefs-are-the-product
- git-tags-as-knowledge-refs
- git-is-the-knowledge-substrate
- knowledge-diff-is-a-command-not-a-substrate
---

# feat: Git Tag System — Tag-Aware Database and Knowledge Diff

> 1,392 git tags exist. Scrape already indexes them into `git_tags` table.
> Build `patina diff` to compare knowledge between any two tags, and enhance
> tag querying in assay.

## Current State

### Tags already scraped (`src/commands/scrape/git/mod.rs`)

`parse_all_tags()` reads all tags via `git tag --format`. `insert_tags()` writes
to `git_tags` table — full clear + rebuild on every scrape (cheap, idempotent).
Session tag parsing (`parse_session_tags()`) links commits to sessions via tag
boundaries.

```sql
-- Already exists in patina.db
CREATE TABLE IF NOT EXISTS git_tags (
    tag_name TEXT PRIMARY KEY,
    sha TEXT,
    tag_date TEXT,
    tagger_name TEXT,
    message TEXT
);
CREATE INDEX IF NOT EXISTS idx_git_tags_date ON git_tags(tag_date);
```

Also writes `git.tag` events to eventlog for project repos.

### Tag landscape (1,392 total)

| Category | Count | Pattern |
|----------|-------|---------|
| Session | 1,158 | `session-YYYYMMDD-HHMMSS-{adapter}-{start\|end}` |
| Version | 71 | `v0.1.0` through `v0.23.0` |
| Spec archive | 154 | `spec/{spec-id}` |
| Other | 9 | `archive/*`, misc |

### What's NOT indexed

- Tag **category** — no classification of session vs version vs spec vs archive
- Tag **relationships** — no linking between start/end session tag pairs
- **Content at tag** — no snapshot of what beliefs/specs/patterns existed at a tag
- **Diff between tags** — no way to compute what changed between two knowledge states

## What To Build

### Phase A: Tag Classification in Assay

Add a `tag_type` column to `git_tags` and classify during scrape:

```sql
ALTER TABLE git_tags ADD COLUMN tag_type TEXT;
-- Values: 'session-start', 'session-end', 'version', 'spec', 'archive', 'other'
```

Classification rules (in `insert_tags()`):
- `session-*-start` → `session-start`
- `session-*-end` → `session-end`
- `v[0-9]*` → `version`
- `spec/*` → `spec`
- `archive/*` → `archive`
- Everything else → `other`

Add assay query support: `patina assay --query-type tags` to list/filter tags.
`patina assay --query-type tags --pattern "v0.2*"` for glob filtering.

**Code path:** `src/commands/scrape/git/mod.rs` — modify `insert_tags()` to
classify. `src/commands/assay/` — add tags query type.

### Phase B: Knowledge Diff Command

`patina diff <tag1> <tag2>` compares the knowledge state between two git refs.

Implementation using `git2`:

1. **Belief diff** — compare `layer/surface/epistemic/beliefs/` at both refs.
   Parse YAML frontmatter from each. Report: added, removed, modified beliefs.
   Modified = same slug, different content (entrenchment change, evidence added).

2. **Pattern diff** — compare `layer/surface/` and `layer/core/` at both refs.
   Report: added, removed, modified patterns/specs.

3. **Session diff** — count sessions between the two tags by scanning
   `layer/sessions/` tree diff.

Output format:
```
patina diff v0.20.0 v0.23.0

Beliefs: +12 added, -2 removed, ~5 modified
  + git-is-the-knowledge-substrate (high)
  + beliefs-are-the-product (high)
  - old-belief-that-was-archived
  ~ sync-first: entrenchment medium → high

Patterns: +3 added, -1 removed
  + layer/core/session-capture.md
  - layer/surface/old-pattern.md

Sessions: 14 sessions between tags

Commits: 47 commits, 12 authors
```

**Code path:** New `src/commands/diff/mod.rs` command. Uses `git2` crate
(already in `Cargo.toml`) for tree comparison at arbitrary refs.

### Phase C: Tag Pairs and Ranges

Link session start/end tags as pairs in the database:

```sql
CREATE TABLE IF NOT EXISTS tag_pairs (
    pair_id TEXT PRIMARY KEY,       -- session ID or version range
    start_tag TEXT NOT NULL,
    end_tag TEXT,                   -- NULL if session still open
    pair_type TEXT NOT NULL,        -- 'session', 'version-range'
    commits INTEGER DEFAULT 0,     -- commits between tags
    FOREIGN KEY (start_tag) REFERENCES git_tags(tag_name),
    FOREIGN KEY (end_tag) REFERENCES git_tags(tag_name)
);
```

Populate during scrape from existing `parse_session_tags()` logic. Version pairs
are consecutive version tags (v0.22.0 → v0.23.0).

Enable: `patina diff --session 20260215-083121` (diff a session's start to end).

## Exit Criteria

1. `git_tags.tag_type` column populated during scrape with correct classification
2. `patina assay --query-type tags` lists tags with type filter
3. `patina diff v0.20.0 v0.23.0` shows belief/pattern/session diff between tags
4. `patina diff --session <id>` diffs a session's start to end tag

## Non-Goals

- Content-addressed knowledge objects (killed by [[knowledge-protocol]] Outcome C)
- Custom tag creation (git tags are created by session/spec/release commands)
- Tag-based belief history (belief identity is the slug, not a hash per tag)
- Real-time tag watching (scrape is batch)

## Evidence

| Claim | Source |
|-------|--------|
| 1,392 tags exist (1,158 session, 71 version, 154 spec) | `git tag \| wc -l` |
| git_tags table schema exists | `src/commands/scrape/git/mod.rs:358-372` |
| parse_all_tags reads all tags | `src/commands/scrape/git/mod.rs:178-215` |
| insert_tags clears and rebuilds | `src/commands/scrape/git/mod.rs:219-256` |
| parse_session_tags links commits to sessions | `src/commands/scrape/git/mod.rs:39-125` |
| git2 crate already in Cargo.toml | `cargo tree \| grep git2` |
| No tag_type classification exists | `src/commands/scrape/git/mod.rs:359-363` |
