# Design: Fast Incremental Scrape + Git Hooks

## Baseline (measured 2026-03-03, patina repo: ~3K commits, 248 source files, 181 beliefs)

```
patina scrape (incremental, 3 new commits): 16.1s wall (5.1s user, 7.6s system)
  code scraper:  ~4.5s — 248 files parsed, 17 WASM pipeline plugins loaded
  git scraper:   ~10s  — 3 new commits, BUT co-change full rebuild (26K pairs) + FTS5 full rebuild (3K messages)
  layer scraper: <0.5s — incremental, 1 item
  belief scraper: <0.5s — incremental, 0 new
```

Bottleneck: git scraper's `rebuild_co_changes()` does DELETE + full rebuild every run.
Second: code scraper re-parses all 248 files despite `index_state` table existing.

## Post-optimization (measured 2026-03-03, same repo after commits 1–5)

```
patina scrape (incremental, 0 new commits): 6.37s wall (3.87s user, 1.82s system)
  code scraper:  ~3.17s — 0 files parsed (250 skipped via mtime), BUT:
                          WASM plugin loading ~2s (17 grammars),
                          FTS5 full rebuild ~1s (6606 symbols)
  git scraper:   ~1.5s  — 0 new commits, tags+tracked files reindexed, 0 co-change pairs
  layer scraper: <0.1s  — 0 items (55 skipped)
  belief scraper: ~1.5s — 0 new beliefs, BUT grounding recomputed for 189 beliefs

patina scrape (incremental, 1 new commit): 6.47s wall (3.81s user, 1.62s system)
  Same profile + 1 file reparsed, 21 co-change pairs upserted
```

Improvement: **16.1s → 6.4s** (60% reduction). Bottlenecks eliminated:
co-change rebuild (10s → 0), code re-parse (4.5s → ~0).

**EC1 (<2s) not met.** Remaining ~6s is fixed overhead outside this spec's scope:
- WASM plugin loading: ~2s (17 grammar plugins instantiated even when 0 files need parsing)
- FTS5 full rebuilds: ~1s (code_search + commits_fts5, DELETE + re-insert every run)
- Belief grounding: ~1.5s (recomputes all 189 beliefs every run, no skip logic)
- Git tags + tracked files: ~1s (full rebuild every run, ~1800 tags + ~1555 files)

These are follow-up optimization targets, each requiring its own design work.

## Resolved Decisions

### D1: Code file-skip uses mtime via existing `index_state` (not git diff)

**Why not git diff (SPEC Strategy A)?** Git diff requires the git scraper to run first,
creating an execution ordering dependency enforced only by a code comment. Per
[[correctness-by-construction-not-convention]]: that's a convention, not a boundary.
Per [[unix-philosophy]]: each scraper is its own tool. The code scraper should not
depend on git scraper output.

**Why mtime?** The `index_state` table already exists (`code/database.rs:230`).
Every file's `(path, mtime, size)` is already written on every run
(`extract_v2.rs:142`). The infrastructure is write-only today — we add the read.
This follows the established pattern: layer scraper checks `SELECT id FROM patterns`,
beliefs scraper checks `SELECT id FROM beliefs`, code scraper will check
`SELECT mtime FROM index_state WHERE path = ?`.

Same pattern, same principle, no new coupling.

### D2: Hook install is Option B (documented manual step)

Per [[safety-boundaries]]: "User consent — Ask before major operations." A symlink
the user types is explicit consent. `patina init` already has enough responsibilities
(adapter setup, tmux, force/local modes). Adding hook logic violates
[[unix-philosophy]]: "No feature creep — new functionality = new commands, not new flags."

For patina's own repo: `ln -sf ../../resources/git/post-commit.sh .git/hooks/post-commit`
For other projects: user creates a 3-line script (documented in resources/git/README).

### D3: Hook observability comes from existing measure system

The hook's job is one thing: trigger `patina scrape`. Per [[unix-philosophy]]: one
tool, one job. The hook is not an observability system.

If the hook fails silently, `measure.capture` events stop arriving. The capture verb's
`age_hours` increases → `freshness` degrades → `status` goes to `needs_attention`.
This already works — `patina measure` surfaces it. The LLM sees it in every
`patina context` call via ambient health (built in session 20260303-054447).

The hook redirects stderr to a log file for manual debugging. No new infrastructure.

### D4: Part 5 (persist timings) is already complete

All 5 scrapers emit `measure::emit_or_warn("capture", "scrape", "<name>", ...)` with
`duration_ms`. Area 2 shipped in v0.33. The SPEC's "temporary scrape_meta key" section
is obsolete — Area 2 was complete before this spec was drafted.

Exit criterion #6 (baseline profile) is met by: (a) measure events persisting
duration_ms automatically, and (b) the baseline documented above in this DESIGN.md.

### D5: FTS5 rebuilds stay as DELETE + rebuild (for now)

`populate_commits_fts5()` and `populate_fts5()` do full rebuilds. FTS5 tables don't
support efficient incremental delete-by-key. The cost is ~1s total. Not the bottleneck.
If profiling after co-change fix shows FTS5 is the new bottleneck, address in a
follow-up. Don't optimize what isn't measured.

### D6: Tags and tracked-files stay as full rebuild

`insert_tags()` and `insert_tracked_files()` do DELETE + rebuild. Tags are cheap
(~1.8K items), tracked files are cheap (~1.5K items). Both are idempotent and fast.
The simplicity is worth more than the milliseconds saved.

## Approach

### Part 1: Incremental co-change

Replace `rebuild_co_changes()` with `update_co_changes(new_commits)`.

The `co_changes` table already has `PRIMARY KEY (file_a, file_b)` (git/mod.rs:349),
so the upsert works directly:

```sql
INSERT INTO co_changes (file_a, file_b, count) VALUES (?1, ?2, 1)
ON CONFLICT(file_a, file_b) DO UPDATE SET count = count + 1
```

Watermark: add `set_last_processed(conn, "co_changes", sha)` alongside the existing
`set_last_processed(conn, "git", sha)`. This lets co-change tracking have its own
cursor (if a previous run failed between commit ingestion and co-change computation,
co-change picks up from its own watermark, not git's).

Keep `rebuild_co_changes()` as-is for `--rebuild` / `full=true`. The incremental path
only processes commits from the new batch (already in memory from `parse_git_log()`).
Pass `&[GitCommit]` directly — no second DB read needed.

### Part 2: Code scraper mtime skip

In `extract_v2.rs`, before parsing a file, check `index_state`:

```rust
// Check if file changed since last scrape
let stored = db.get_index_state(&relative_path)?;
if let Some((stored_mtime, stored_size)) = stored {
    if mtime == stored_mtime && size == stored_size && !config.force {
        continue; // Skip unchanged file
    }
}
```

Add `Database::get_index_state(path) -> Option<(mtime, size)>` to `code/database.rs`.
The table and data already exist. This is adding a read to a write-only table.

No execution order change needed. No coupling to git scraper. `--force` bypasses
the check (existing pattern: `config.force` already exists in `ScrapeConfig`).

### Part 3: Post-commit hook — shell shim + `patina hook post-commit`

**Architecture:** Per [[dependable-rust]], the shell script is a 2-line shim.
All logic lives in Rust, testable in CI, enforceable by the compiler.

Shell shim (`resources/git/post-commit.sh`):
```bash
#!/bin/sh
# Install: ln -sf ../../resources/git/post-commit.sh .git/hooks/post-commit
command -v patina >/dev/null 2>&1 || exit 0
patina hook post-commit
```

Create `resources/git/post-merge.sh` with the same content (calls
`patina hook post-merge`).

`patina hook post-commit` (new subcommand in `src/commands/hook/`):
- Forks `patina scrape` to background (spawn child process, detach)
- Logs to `.patina/local/hook.log` (patina's data dir, not `.git/`)
- Emits `hook.post-commit` event to events.db (timestamp, outcome)
- Returns immediately so git commit is never blocked

Per [[dependable-rust]]: `src/commands/hook/mod.rs` is the external interface
(small: subcommand dispatch for `post-commit`, `post-merge`),
`src/commands/hook/internal.rs` is the implementation (fork, log, emit).
The shell shim can't grow because there's nothing left in it.

For projects without `resources/git/`, users create the same 3-line shim
directly in `.git/hooks/post-commit`.

**Repo root resolution:** Git sets `$PWD` to the worktree root before invoking
`post-commit` hooks (non-bare repos). `patina scrape` already uses
`std::env::current_dir()` (`scrape/mod.rs:63`), so `.patina/` is found
automatically. No `--repo-root` flag or `git rev-parse --show-toplevel`
needed. Add a one-line comment in `internal.rs` documenting this assumption.

### Part 3a: Scrape regression diagnostic

Add `fn diagnostics()` to `CaptureGitScrapeMetrics`, `CaptureCodeMetrics` (and
any other capture metrics with `duration_ms`): emit a warning when
`duration_ms > 5000`.

Existing infrastructure — no new types, no new wiring:
- `CaptureGitScrapeMetrics` already has `duration_ms: i64` (`measure/internal.rs:508`)
- `collect_source_diagnostics()` at line 764 already dispatches to each metrics
  type's `diagnostics()` — just add `CaptureGitScrape` to the match arm
- The diagnostic flows through `FullVerbSummary.diagnostics` → `patina measure`
  terminal output + `patina measure --json` + ambient health in `patina context`

This is the regression gate: if a future change regresses scrape performance,
the system notices automatically. Per Andrew Ng: "ship, measure, iterate" — but
without the gate, you only have "ship" and "measure once."

### Part 4: Hook install documentation

Add `resources/git/README.md` with install instructions:

```bash
# Install post-commit hook (keeps patina knowledge base current after each commit)
ln -sf ../../resources/git/post-commit.sh .git/hooks/post-commit

# Install post-merge hook (keeps patina current after pull/merge)
ln -sf ../../resources/git/post-merge.sh .git/hooks/post-merge

# Verify
ls -la .git/hooks/post-*
```

For projects that don't have `resources/git/`:
```bash
cat > .git/hooks/post-commit << 'EOF'
#!/bin/sh
command -v patina >/dev/null 2>&1 || exit 0
patina hook post-commit
EOF
chmod +x .git/hooks/post-commit
```

## Commits

1. `profile(scrape): capture baseline timings in DESIGN.md` — document the baseline
   numbers measured in the audit session. This commit checks EC6 (baseline captured).

2. `feat(scrape/git): incremental co-change upsert` — replace DELETE+rebuild with
   upsert for new commits only. Add `co_changes` watermark in `scrape_meta`. Keep
   `rebuild_co_changes()` for `--rebuild`. Add `PRAGMA busy_timeout = 5000` to
   `patina.db` initialization. Checks EC2 (incremental co-change).

3. `feat(scrape/code): skip unchanged files via index_state mtime` — add
   `get_index_state()` read, skip files where mtime+size match. Prune stale
   `index_state` rows for deleted files (adopt layer scraper pattern). Force flag
   bypasses. Checks EC7 (code scraper skips unchanged files).

4. `feat(hook): add patina hook subcommand with shell shims` — create
   `src/commands/hook/mod.rs` + `internal.rs` (post-commit, post-merge handlers:
   fork scrape to background, log to `.patina/local/hook.log`, emit hook event).
   Create `resources/git/post-commit.sh` and `post-merge.sh` (2-line shims).
   Create `resources/git/README.md` with install docs. Checks EC3 (hook exists),
   EC4 (install mechanism), EC5 (hook completes fast / runs in background).

5. `feat(measure): scrape duration regression diagnostic` — add `diagnostics()`
   to `CaptureGitScrapeMetrics` and other capture metrics with `duration_ms`:
   warn when >5s. Add to `collect_source_diagnostics()` match arm. Regression
   gate for the 2s target.

6. `verify(scrape): benchmark incremental after single commit` — run the benchmark
   defined in SPEC.md alignment notes. Document result. Checks EC1 (< 2s after
   single commit). If >2s, profile and identify remaining bottleneck.

## Key Files

- `src/commands/scrape/git/mod.rs` — `rebuild_co_changes()` rewrite, watermark
- `src/commands/scrape/code/extract_v2.rs` — mtime skip logic in main loop
- `src/commands/scrape/code/database.rs` — `get_index_state()` query
- `src/commands/hook/mod.rs` — new subcommand: external interface
- `src/commands/hook/internal.rs` — fork, log, emit implementation
- `src/commands/measure/internal.rs` — duration regression diagnostics
- `src/eventlog.rs` — `PRAGMA busy_timeout = 5000` for patina.db
- `resources/git/post-commit.sh` — shell shim (2 lines + guard)
- `resources/git/post-merge.sh` — shell shim (2 lines + guard)
- `resources/git/README.md` — hook install documentation

## Open Questions

None. All decisions resolved in audit session 20260303-070328.

## Implementation Notes

- **Stale row pruning for mtime skip.** When a file is deleted or renamed, the mtime
  gate means it's never walked, so old `index_state`, `code_search`, `function_facts`,
  etc. rows persist. This is a pre-existing condition (the code scraper never pruned
  deleted files), but mtime skip makes it more visible. The layer scraper already has
  pruning logic (`layer/mod.rs:590-610`): it collects `current_file_ids`, queries DB
  for all IDs, deletes the difference. The code scraper should adopt the same pattern
  in commit 3: after processing, query `index_state` for paths not in the walked set,
  delete those rows and their downstream facts. This keeps the mtime gate clean.

- **Busy timeout for concurrent hook scrapes.** `patina.db` intentionally does not use
  WAL — it's rebuildable projections (`patina scrape --rebuild` regenerates it from
  source files and git), so the durability guarantees of WAL+synchronous=FULL aren't
  needed. That combination is reserved for `events.db` (irreplaceable runtime events,
  `eventlog.rs:194-197`). However, `patina.db` sets no `busy_timeout` (default: 0ms →
  immediate SQLITE_BUSY), which means two concurrent background scrapes from rapid
  commits would fail. Commit 2 adds `PRAGMA busy_timeout = 5000` to
  `eventlog::initialize()` for `patina.db`. No WAL, just patience.

- The `commits_fts5` full rebuild (~1s) may become the new bottleneck after co-change
  is fixed. Profile in commit 5 and address in follow-up if needed.

- `MAX_FILES_PER_COMMIT = 50` guard prints to stdout but isn't in measure diagnostics.
  Not blocking — note for future improvement.

- Deleted files leave stale entries in code tables (symbols, functions, etc.). This is
  addressed by the pruning note above — the implementation should adopt the layer
  scraper's pruning pattern.
