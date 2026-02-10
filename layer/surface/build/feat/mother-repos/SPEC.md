---
type: feat
id: mother-repos
status: design
created: 2026-02-09
sessions:
  origin: 20260209-215657
related:
  - layer/surface/build/feat/mother/SPEC.md
  - layer/surface/build/feat/mother-environment/SPEC.md
  - layer/surface/build/feat/mother-beliefs/SPEC.md
beliefs:
  - mother-is-the-daemon
  - mother-owns-ref-repo-indexing
  - four-layer-architecture
  - corpus-composition-over-model
---

# feat: Mother Repos — Reference Repository Ownership

> Mother owns the full lifecycle of reference repositories: git pull, scrape,
> index, and serve. Projects declare a dependency on ref repo knowledge;
> Mother handles everything else. Eliminates `oxidize_for_repo()` and makes
> ref repo maintenance a daemon responsibility, not a manual project command.

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
The daemon runs 24/7 but doesn't notice. Gastown's deacon patrol pattern
solves exactly this: heartbeat → check freshness → act.

### No Knowledge Flow From Ref Repos

Ref repos are indexed for code search (scry), but no beliefs are extracted.
A ref repo like `anthropics/claude-code` contains architectural patterns,
but those patterns don't flow into the belief layer. Ref repos are treated
as code dumps, not knowledge sources.

## Current State

```
~/.patina/
├── cache/repos/              # 25 repos, 3.9GB
│   ├── anthropics/claude-code/
│   ├── dojoengine/dojo/
│   ├── steveyegge/gastown/
│   └── ... (22 more)
├── registry.yaml             # repo metadata (name, path, domains, contrib)
└── mother/
    └── graph.db              # 31 nodes, 3 edges
```

**Current flow (project-initiated):**
```
User runs `patina oxidize --repo dojo` from patina project
  → oxidize_for_repo() changes to dojo directory
  → symlinks patina's resources/models/ into dojo
  → runs full oxidize pipeline
  → cleans up symlink
  → restores directory
```

**Desired flow (Mother-owned):**
```
Mother daemon heartbeat detects dojo has new commits
  → Mother pulls latest
  → Mother scrapes using central model from ~/.patina/cache/models/
  → Mother indexes into dojo's .patina/local/data/
  → Projects querying dojo get fresh results
```

## Solution

### 1. Mother Owns Repo Update + Index

Move the scrape/oxidize pipeline for ref repos into the daemon. On heartbeat
(configurable interval, default 1h):

1. Check each registered repo for new commits (`git rev-parse HEAD` vs stored)
2. If stale: `git pull`
3. If pulled new commits: re-scrape, re-index
4. Log results to Mother's own eventlog

### 2. Eliminate `oxidize_for_repo()`

Replace the project-initiated path with a Mother command:

```bash
# Instead of (from project context):
patina oxidize --repo dojo

# Mother manages it:
patina mother index dojo        # manual trigger
patina mother index --all       # re-index everything
patina mother index --stale     # only repos with new commits
```

Or it happens automatically via daemon heartbeat.

### 3. Repo Freshness in `patina mother status`

```
$ patina mother status

Mother daemon: running (PID 22096)
  Uptime: 3.7h
  Model: e5-base-v2@onnx (768d)

Repos: 25 registered
  ✓ 18 up to date
  ⚠  5 stale (new commits available)
  ✗  2 not indexed

  Stale:
    steveyegge/gastown     470 commits behind (last indexed: never)
    openai/codex           12 commits behind (last indexed: 3d ago)
    ...
```

### 4. Projects Declare Repo Dependencies

Projects declare which ref repos they care about in `.patina/config.toml`:

```toml
[repos]
depends = ["dojoengine/dojo", "unum-cloud/USearch", "sst/opencode"]
```

This feeds the graph (auto-creates USES edges) and tells Mother which repos
to prioritize for this project's queries.

## Acceptance Criteria

1. [ ] `patina mother index <repo>` indexes a ref repo using central models (no symlink hack)
2. [ ] `patina mother index --stale` indexes only repos with new commits
3. [ ] Daemon heartbeat checks repo freshness (configurable interval)
4. [ ] `patina mother status` shows repo freshness summary
5. [ ] `oxidize_for_repo()` removed or deprecated in favor of Mother path
6. [ ] Graph edges auto-created from project `[repos] depends` config
7. [ ] Ref repo scrape/index uses Mother-owned models (depends on [[mother-environment]])

## Non-Goals

- Automatic belief extraction from ref repos (that's [[mother-beliefs]])
- Contributing back to ref repos (contrib mode is separate)
- Cloning repos (patina repo add already handles this)
- Real-time watching (inotify/fsevents) — heartbeat polling is sufficient
