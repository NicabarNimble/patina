---
type: feat
id: data-fast-incremental
status: draft
created: 2026-02-27
sessions:
  origin: 20260227-062333
related:
- data-architecture-v2
beliefs:
- if-its-patina-its-git
- correctness-by-construction-not-convention
exit_criteria:
- id: incremental-scrape-after-single-commit-under-2s
  text: 'incremental scrape after a single commit completes in < 2s (measured, not estimated)'
  checked: false
- id: co-change-update-is-incremental-not-full-rebuild
  text: co-change computation updates only pairs affected by new commits — no DELETE + rebuild
  checked: false
- id: post-commit-hook-exists-and-triggers-incremental-scrape
  text: 'post-commit hook in `resources/git/` triggers `patina scrape` incrementally'
  checked: false
- id: hook-install-mechanism-exists
  text: '`patina init` or documented manual step installs hooks — no silent filesystem writes'
  checked: false
- id: hook-completes-under-2s-with-no-visible-delay-to-developer
  text: 'hook runs in background or completes fast enough (< 2s) that `git commit` feels instant'
  checked: false
- id: baseline-profile-captured-before-any-optimization
  text: baseline scrape timings (per-scraper, per-phase) captured and documented before changes
  checked: false
- id: code-scraper-skips-unchanged-files
  text: code scraper skips files unchanged since last scrape (mtime or git status check)
  checked: false
---
# feat: Fast Incremental Scrape + Git Hooks

> Full scrape re-processes everything on every run. After a single commit,
> scrape takes too long for hook-driven automation. Area 5 of
> [[data-architecture-v2]]. Phase D — performance polish after the
> architecture is correct.

## Problem

`patina scrape` runs 4 scrapers in sequence: code, git, layer, beliefs.
After a single commit, most of the work is redundant — only one file
changed, but the system re-processes everything.

The specific bottlenecks:

1. **Co-change is O(C*F²) and rebuilds from scratch every run.** The git
   scraper's `rebuild_co_changes()` does `DELETE FROM co_changes` then
   recomputes all file pairs from all commits. With ~8K commits and an
   average of ~5 files per commit, this is the dominant cost. Even on an
   incremental 2-commit scrape, the co-change table is wiped and rebuilt
   from the full `commit_files` table.

2. **Code scraper re-parses unchanged files.** `scrape code` walks every
   file matching the grammar set and re-parses it with tree-sitter. No
   mtime or content-hash check to skip files that haven't changed.

3. **No git hooks — the belief system drifts between sessions.** There is no
   post-commit hook to trigger incremental scrape automatically. The developer
   must remember to run `patina scrape` after committing. This means the
   knowledge base drifts until the next manual scrape or session start.
   Patina is a belief system that lives on top of git — hooks aren't
   performance polish, they're how the system stays alive. Without hooks,
   beliefs grounded in code become stale the moment a commit lands.

4. **No timing data.** `ScrapeStats` captures `time_elapsed` but only
   prints it to stdout — never persisted. There's no baseline to measure
   improvements against or detect regressions.

## Solution

### 0. Profile before optimizing

Before any code changes, capture baseline timings:

```bash
patina scrape          # full incremental — time each scraper
patina scrape --full   # forced full — time each scraper
```

Record per-scraper wall-clock time in `measure.capture` events (Area 2
wires this). If Area 2 isn't complete yet, capture manually and document
in this spec's DESIGN.md. The point: no optimization without measurement.

### 1. Incremental co-change computation

Replace `rebuild_co_changes()` with incremental upsert:

**Current (full rebuild every run):**
```
DELETE FROM co_changes
for each commit in ALL commits:
    for each file pair in commit:
        accumulate count
INSERT all pairs
```

**After (incremental):**
```
for each commit in NEW commits only:
    for each file pair in commit:
        INSERT INTO co_changes (file_a, file_b, count)
        VALUES (?1, ?2, 1)
        ON CONFLICT(file_a, file_b) DO UPDATE SET count = count + 1
```

Requirements:
- Add a UNIQUE constraint on `co_changes(file_a, file_b)` (or create the
  index if not present — check current schema)
- Track the last-processed commit SHA for co-changes specifically (may
  differ from `scrape_meta` last_processed_git if a previous run failed
  mid-co-change)
- Keep the `MAX_FILES_PER_COMMIT = 50` guard for bulk commits
- `--rebuild` still does the full DELETE + rebuild — rebuild is the
  correctness backstop

### 2. Code scraper file-skip optimization

Before parsing a file, check if it's changed since last scrape:

**Strategy A — git status (preferred):**
```
git diff --name-only <last_code_sha>..HEAD
```
Only parse files in the diff set. This piggybacks on git's efficient
tree comparison. Files not in the diff are guaranteed unchanged.

**Strategy B — mtime check (fallback):**
Compare file mtime against `scrape_meta` last_processed_code timestamp.
Skip files older than the last scrape. Less reliable than git (mtime can
be reset by builds, editors) but works without git.

Strategy A is preferred because it's exact and already available via git.

**Execution order prerequisite:** Today `execute_all()` runs code BEFORE git.
This must be swapped: git first, then code. The git scraper produces the diff
of changed files; the code scraper consumes it to skip unchanged files. Add a
comment in `execute_all()` documenting this ordering dependency so future
refactors don't break the assumption.

### 3. Post-commit hook

Create `resources/git/post-commit.sh`:

```bash
#!/bin/sh
# Trigger incremental scrape after commit
# Runs in background — git commit returns immediately
patina scrape --incremental &
```

**Design constraints (from safety-boundaries):**
- Hook runs scrape in background (`&`) so `git commit` returns instantly.
  The developer never waits for patina.
- Hook is a shell script in `resources/git/`, not auto-installed. The user
  copies or symlinks it deliberately. No silent filesystem writes.
- Hook must handle the case where `patina` is not on PATH (fail silently,
  don't break git).
- Add a `post-merge` hook with the same pattern — `git pull` + `git merge`
  should also trigger incremental scrape.

### 4. Hook install mechanism

Two options (pick during implementation):

**Option A — `patina init` integration:**
`patina init` already sets up `.patina/`. Add a step: "Install git hooks?
[y/N]". If yes, symlink `resources/git/post-commit.sh` → `.git/hooks/post-commit`.

**Option B — documented manual step:**
Document in README/CLAUDE.md:
```bash
ln -sf ../../resources/git/post-commit.sh .git/hooks/post-commit
ln -sf ../../resources/git/post-merge.sh .git/hooks/post-merge
```

Option A is friendlier. Option B is simpler and respects user consent more
explicitly. Both satisfy the exit criterion.

### 5. Persist scrape timings

Each scraper already computes `ScrapeStats.time_elapsed`. After Area 2
wires `measure.capture` events for all scrapers, the timing data flows
automatically into events.db. No additional work needed here — Area 2
handles the emission, this spec consumes it for regression detection.

If Area 2 isn't complete when this work begins, add a temporary
`scrape_meta` key: `last_scrape_duration_ms_<scraper>`. Replace with
proper event emission when Area 2 ships.

## Non-Goals

- **Parallel scraping.** Running scrapers concurrently (code + git in
  parallel) would help but adds complexity (shared DB writes, error
  handling). The 2s target is achievable with incremental alone. Parallel
  is a future optimization if needed.
- **Daemon/watch mode.** A long-running process watching for file changes
  and scraping continuously. Hooks are simpler, more Unix, and sufficient.
- **Incremental oxidize.** Embedding regeneration after scrape is a separate
  concern. Oxidize already has its own incremental logic (`index_state`
  table). Out of scope.
- **Layer/beliefs scraper optimization.** These are already fast (small data
  sets — ~200 patterns, ~168 beliefs). Not worth optimizing until profiling
  shows they're bottlenecks.
- **Hook for non-git VCS.** Patina is git-first ([[if-its-patina-its-git]]).
  Hooks for other VCS are out of scope.

## Alignment Notes (session 20260227-075037)

**Hooks are system integrity, not just performance.** Patina is a belief
system that lives on top of git. Hooks are how the system stays current
between sessions — without them, every belief grounded in code becomes
potentially stale the moment a commit lands. This framing upgrades hooks
from Phase D polish to a core system concern. The exit criteria already
cover hook existence and install mechanism; this note captures the *why*.

**Future direction:** Pre-commit hooks for belief verification (does this
commit contradict active beliefs?) are a natural extension. Not v2 scope,
but the hook infrastructure built here should accommodate it. Link to
[[if-its-patina-its-git]] and watch for contradictions — this is a future
target, not a past grounding.

**Benchmark definition:** The 2s gate is measured against patina's own
repository (~8K commits, ~200 patterns, ~168 beliefs). Benchmark invocation:
```bash
echo "// touch" >> src/lib.rs
git add src/lib.rs && git commit -m "benchmark touch"
time patina scrape
```
Baseline profile (exit criterion 6) captures per-scraper timings before
optimization. That becomes the comparison point. Hardware specs documented
alongside baseline.

**Execution order dependency:** git scraper must run before code scraper
in `execute_all()` so git diff data is available for file-skip. This is
a hard ordering constraint that must be enforced with a code comment and
potentially an assertion.
