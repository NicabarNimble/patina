---
type: feat
id: mother-repos
status: ready
created: 2026-02-09
sessions:
  origin: 20260209-215657
related:
- layer/surface/build/feat/mother-architecture/SPEC.md
- layer/surface/build/feat/mother-environment/SPEC.md
- layer/surface/build/feat/plugin-system/SPEC.md
beliefs:
- mother-is-the-daemon
- mother-owns-ref-repo-indexing
- four-layer-architecture
- corpus-composition-over-model
- coupling-is-complexity
- de-risk-runtime-with-simplest-payload
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

---

## Phase 1 Scope (Plugin System Phase 1)

Phase 1 proves the toy system end-to-end with a real second WASM child.
The repos child is the vehicle — it demonstrates that a child can request
work and Mother can run it. Full repo lifecycle ownership comes later.

### Phase 1 Constraints

With only `patina:host/log` available (no WASI filesystem, no database):

- **No filesystem access** — cannot read registry.yaml or check git HEADs directly
- **No database access** — cannot query repo state from SQLite
- **No model access** — AC6 (Mother-owned models) deferred to [[mother-environment]]

### Phase 1 Design

The repos child uses **host-fed state**: the host calls `handle()` to push
repo information into the child, and `tick()` uses that state to decide
what toys to return.

**`on_load()`**: Log initialization. No registry read (no filesystem).

**`handle()`**:
- `"report_repo"` — host tells child about a repo: name, path, last_indexed timestamp
- `"check_freshness"` — return current staleness state for all known repos

**`health()`**: Reports stale count. Healthy if all repos fresh or no repos known.
Degraded if any repo is stale beyond threshold.

**`tick()`**: For each repo that hasn't been indexed recently (comparing current
time vs last_indexed), return toys:
- `Toy { name: "pull-{repo}", command: "git", args: ["-C", path, "pull"] }`
- `Toy { name: "scrape-{repo}", command: "patina", args: ["scrape", "--repo", name] }`

The staleness threshold is hardcoded (24 hours) in Phase 1. No config.

### Phase 1 Acceptance Criteria

- [x] Repos child implements MotherChild as WASM plugin — `patina-plugin-repos/` (178KB)
- [x] `tick()` detects stale repos and requests re-index toys — pull + scrape toys
- [x] Toy system proven end-to-end (child requests work, Mother runs it) — 4 tests
- [x] At least one repos child test in `cargo test` — 4 tests in `plugin::internal::tests`

### What Phase 1 Does NOT Do

- Read registry.yaml (needs WASI filesystem — Phase 2+)
- Check git HEADs directly (child uses host-fed timestamps)
- Remove or deprecate `oxidize_for_repo()` (AC4 — requires full filesystem access)
- Use Mother-owned models (AC6 — requires [[mother-environment]])

---

## Full Acceptance Criteria (Post-Phase 1)

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

---

## Status Log

| Date | Status | Note |
|------|--------|------|
| 2026-02-09 | design | Created from session [[20260209-215657]]. Full lifecycle spec for repos child. |
| 2026-02-12 | ready | Session [[20260212-091430]]: Scoped Phase 1 boundaries. Repos child uses host-fed state (no filesystem/database). tick() returns toys for stale repos. Proves toy system end-to-end. AC4 and AC6 deferred. Related to [[plugin-system]] Phase 1 exit criteria. |
