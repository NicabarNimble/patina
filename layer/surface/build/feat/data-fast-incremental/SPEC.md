---
type: feat
id: data-fast-incremental
status: ready
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
  text: incremental scrape after a single commit completes in < 2s (measured, not estimated)
  checked: false
- id: co-change-update-is-incremental-not-full-rebuild
  text: co-change computation updates only pairs affected by new commits — no DELETE + rebuild
  checked: false
- id: post-commit-hook-exists-and-triggers-incremental-scrape
  text: post-commit hook in `resources/git/` triggers `patina scrape` incrementally
  checked: false
- id: hook-install-mechanism-exists
  text: '`patina init` or documented manual step installs hooks — no silent filesystem writes'
  checked: false
- id: hook-completes-under-2s-with-no-visible-delay-to-developer
  text: hook runs in background or completes fast enough (< 2s) that `git commit` feels instant
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

4. **No baseline documented.** Area 2 (v0.33) wired `measure.capture` events
   with `duration_ms` for all scrapers, so timing data now persists in events.db.
   But no baseline profile has been captured and documented for regression
   comparison.

## Solution

### 0. Profile before optimizing

Before any code changes, capture baseline timings:

```bash
patina scrape          # full incremental — time each scraper
patina scrape --full   # forced full — time each scraper
```

Per-scraper wall-clock time already flows to `measure.capture` events
(Area 2, v0.33). Capture the baseline numbers in this spec's DESIGN.md
before any optimization. The point: no optimization without measurement.

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

**Strategy: mtime + size check via existing `index_state` table.**

The code scraper already writes `(path, mtime, size)` to `index_state` on
every run (`extract_v2.rs:142`, `code/database.rs:230`). The table is
write-only today — add the read. Before parsing, check if mtime and size
match the stored values; if so, skip the file.

This follows the established incremental pattern: layer scraper checks
`SELECT id FROM patterns`, beliefs scraper checks `SELECT id FROM beliefs`,
code scraper checks `SELECT mtime, size FROM index_state WHERE path = ?`.

**Why mtime over git diff?** Git diff (Strategy A in the original draft)
would require the git scraper to run before the code scraper, creating an
execution ordering dependency enforced only by a code comment. Per
[[correctness-by-construction-not-convention]]: conventions aren't boundaries.
Per [[unix-philosophy]]: each scraper is its own tool — the code scraper
should not depend on git scraper output. The mtime approach is self-contained,
uses existing infrastructure, and avoids new coupling.

`--force` bypasses the check (existing `ScrapeConfig.force` flag).

### 3. Post-commit hook

**Architecture: shell shim + Rust implementation.** The git hook is a 2-line
shell script that calls `patina hook post-commit`. All logic — logging,
background execution, event emission — lives in testable Rust inside the
binary. Per [[dependable-rust]]: the shell script is the thin shim, the
Rust module is the black box. The shell script can't grow because there's
nothing left in it to grow.

Shell shim (`resources/git/post-commit.sh`):
```bash
#!/bin/sh
# Install: ln -sf ../../resources/git/post-commit.sh .git/hooks/post-commit
command -v patina >/dev/null 2>&1 || exit 0
patina hook post-commit
```

`patina hook post-commit` (Rust):
- Forks scrape to background (daemonize or spawn+detach)
- Logs to `.patina/local/hook.log` (patina's data dir, not `.git/`)
- Emits `hook.post-commit` event to events.db (timestamp, exit status)
- `command -v` guard remains in shell: if `patina` isn't on PATH, the
  shim exits cleanly. Git commit is never blocked by a missing tool.

**Design constraints (from [[safety-boundaries]]):**
- Hook runs scrape in background so `git commit` returns instantly.
  The developer never waits for patina.
- Hook is a shell script in `resources/git/`, not auto-installed. The user
  copies or symlinks it deliberately. No silent filesystem writes.
- No concurrent-scrape lock needed: add `PRAGMA busy_timeout = 5000` to
  `patina.db` so a second overlapping scrape waits rather than failing
  with SQLITE_BUSY. Data is idempotent (INSERT OR REPLACE, upsert), so
  two scrapes producing the same result is harmless.
- Add a `post-merge` hook with the same pattern — `git pull` + `git merge`
  should also trigger incremental scrape.

### 3a. Scrape regression diagnostic

Add a `diagnostics()` method to `CaptureGitScrapeMetrics` (and the other
capture metrics with `duration_ms`): when `duration_ms > 5000`, emit a
warning diagnostic. This flows through the existing
`collect_source_diagnostics()` → `FullVerbSummary.diagnostics` →
`patina measure` + ambient health in `patina context`.

This is the regression gate: if a future change makes scrape slow again,
`patina measure` shows a warning, the LLM sees it in ambient health, and
the developer sees it in `patina measure --full`. No new infrastructure —
just a `diagnostics()` impl on structs that already have `duration_ms`.

### 4. Hook install mechanism

**Documented manual step** (Option B). The user symlinks deliberately:

```bash
ln -sf ../../resources/git/post-commit.sh .git/hooks/post-commit
ln -sf ../../resources/git/post-merge.sh .git/hooks/post-merge
```

Per [[safety-boundaries]]: "User consent — Ask before major operations."
A symlink the user types is explicit consent. `patina init` already has
enough responsibilities (adapter setup, tmux, force/local modes) — adding
hook logic would violate [[unix-philosophy]]: "No feature creep — new
functionality = new commands, not new flags."

Install instructions live in `resources/git/README.md`. For projects
without `resources/git/`, the user creates a 3-line script directly.

### 5. Persist scrape timings

**Already complete.** Area 2 shipped in v0.33 (spec [[data-emission-completeness]]).
All 5 scrapers emit `measure::emit_or_warn("capture", "scrape", "<name>", ...)`
with `duration_ms` to events.db. No additional work needed — this spec
consumes the timing data for regression detection and baseline documentation.

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

**Execution order dependency (resolved):** The original draft proposed
git-diff-first, requiring git scraper before code scraper. Audit session
20260303-070328 rejected this: per [[correctness-by-construction-not-convention]],
a comment-enforced ordering is a convention, not a boundary. The mtime
approach via existing `index_state` is self-contained — no ordering
dependency. `execute_all()` order is unchanged.
