---
type: feat
id: mother-repos
status: design
created: 2026-02-09
sessions:
  origin: 20260209-215657
related:
  - layer/surface/build/feat/mother-architecture/SPEC.md
  - layer/surface/build/feat/mother-environment/SPEC.md
beliefs:
  - mother-is-the-daemon
  - mother-owns-ref-repo-indexing
  - four-layer-architecture
  - corpus-composition-over-model
---

# feat: Repos Child — Reference Repository Ownership

> A `MotherChild` that owns the full lifecycle of reference repositories:
> git pull, scrape, index, and serve. Projects declare a dependency on ref
> repo knowledge; Mother handles everything else. Eliminates `oxidize_for_repo()`
> and makes ref repo maintenance a daemon responsibility.

## Problem

### Ref Repos Are Project-Initiated But User-Scoped

25 reference repos (3.9GB) live at `~/.patina/cache/repos/`. They are
user-level resources shared across projects. But today:

- **`patina repo add`** — registers a repo (user-level, correct)
- **`patina repo update`** — git pull + rescrape (user-level, correct)
- **`patina oxidize --repo <name>`** — indexes a repo (project-level, wrong)

The oxidize path (`src/commands/oxidize/mod.rs:124-196`) runs from within a
project context. It changes the working directory to the ref repo, symlinks the
project's model directory, and runs the full oxidize pipeline. This means:

1. Only the project that initiated oxidize has the right model config
2. If two projects use different models, ref repo indexes could be inconsistent
3. No automatic re-indexing when repos get new commits
4. Manual process — user must remember to run `patina oxidize --repo <name>`

### Repos Go Stale Silently

```
$ patina repo show steveyegge/gastown
  Synced: ⚠ 470 commits behind
  Events: 0
```

470 commits behind with zero events indexed. No alert, no automatic action.
The daemon runs 24/7 but doesn't notice.

### Current vs Desired Flow

**Today (project-initiated):**
```
User runs `patina oxidize --repo dojo` from patina project
  → oxidize_for_repo() changes to dojo directory
  → symlinks patina's resources/models/ into dojo
  → runs full oxidize pipeline
  → cleans up symlink
  → restores directory
```

**Target (Mother-owned):**
```
Mother daemon heartbeat detects dojo has new commits
  → Mother pulls latest
  → Mother scrapes using central model from ~/.patina/cache/models/
  → Mother indexes into dojo's .patina/local/data/
  → Projects querying dojo get fresh results
```

## As a MotherChild

```
name()   → "repos"
state    → ~/.patina/cache/repos/ (cache — rebuildable from git)
           ~/.patina/registry.yaml (portable)
```

**`on_load()`**: Read registry, check repo freshness against stored HEADs.

**`handle()`**:
- `index(repo)` — scrape + oxidize a specific repo using models child
- `index_stale()` — index only repos with new commits
- `freshness()` — return per-repo commit delta and last-indexed timestamp

**`health()`**: How many repos are stale? Any repos unindexed? Registry readable?

**`tick()`**: Check each registered repo for new commits (`git rev-parse HEAD`
vs stored). If stale beyond threshold, request toys to pull + re-index.
This is the heartbeat-driven freshness check.

**Toys requested**: `git pull` (shell), scrape pipeline (shell), oxidize
pipeline (shell). Repos child decides *what* to update, Mother runs the work.

## Acceptance Criteria

1. [ ] Repos child implements `MotherChild` trait
2. [ ] `handle()` can index a ref repo using models child (no symlink hack)
3. [ ] `tick()` detects stale repos and requests re-index toys
4. [ ] `oxidize_for_repo()` removed or deprecated in favor of repos child path
5. [ ] `patina mother status` shows repo freshness (via `health()`)
6. [ ] Ref repo indexing uses Mother-owned models (depends on [[mother-environment]])

## Non-Goals

- Belief extraction from ref repos (future extension, not this spec)
- Contributing back to ref repos (contrib mode is separate)
- Cloning repos (`patina repo add` already handles this)
- Real-time watching (inotify/fsevents) — heartbeat polling is sufficient
