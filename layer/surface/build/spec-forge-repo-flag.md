# Spec: scrape forge --repo

**Status:** Implementation
**Created:** 2026-01-11
**Origin:** Session 20260111-083741 (understanding UI changes)

---

## Problem

`scrape forge` only works on current project. For ref repos, `--with-issues` only works at `repo add` time. This means:

1. No incremental sync for ref repos after initial add
2. Must remove/re-add repo to refresh issues (fetches ALL again)
3. `run_legacy()` bypasses all rate-limiting infrastructure we built

This breaks the API-friendly, incremental design of forge sync.

---

## Solution

Add `--repo <name>` flag to `scrape forge`. Same command, different target.

```bash
# Current project (unchanged)
patina scrape forge

# Ref repo (new)
patina scrape forge --repo claude-code
patina scrape forge --repo claude-code --drain
patina scrape forge --repo claude-code --status
```

---

## Design

### Core Values Applied

| Value | Application |
|-------|-------------|
| unix-philosophy | One tool, one job. Extend existing command, don't create new one. |
| dependable-rust | Minimal interface change. One flag, reuse all internals. |
| Eskil/Gjengset | Obvious behavior. No hidden state, no magic. |

### Changes

| File | Change |
|------|--------|
| `src/main.rs` | Add `--repo` arg to Forge subcommand |
| `src/commands/scrape/mod.rs` | Pass repo to `execute_forge()`, resolve path |
| `src/commands/scrape/forge/mod.rs` | Add `working_dir` to config, use in `run()` |
| `src/commands/repo/internal.rs` | Call `forge::run()` instead of `run_legacy()` |

### Deletions

| Item | Reason |
|------|--------|
| `run_legacy()` | Replaced by `run()` with `working_dir` |
| `GitHubScrapeConfig` | Duplicate of `ForgeScrapeConfig` |

---

## Implementation

### 1. CLI (main.rs)

```rust
Forge {
    #[arg(long)]
    full: bool,
    #[arg(long)]
    status: bool,
    #[arg(long)]
    drain: bool,
    #[arg(long)]
    repo: Option<String>,  // NEW
}
```

### 2. Config (forge/mod.rs)

```rust
pub struct ForgeScrapeConfig {
    pub limit: usize,
    pub force: bool,
    pub drain: bool,
    pub working_dir: Option<PathBuf>,  // NEW: target directory
}
```

### 3. Run function

- If `working_dir` provided, use it for:
  - Git remote URL detection
  - Database path (`.patina/data/patina.db` relative to working_dir)
- All sync infrastructure unchanged

### 4. Repo command migration

```rust
// Before (bypasses sync)
let stats = run_legacy(config)?;

// After (uses full sync)
let config = ForgeScrapeConfig {
    working_dir: Some(repo_path),
    ..Default::default()
};
let stats = forge::run(config)?;
```

---

## Success Criteria

1. `patina scrape forge --repo claude-code` works
2. `--status` and `--drain` work with `--repo`
3. `repo add --with-issues` still works (uses new path)
4. `run_legacy()` and `GitHubScrapeConfig` deleted
5. Net negative line count

---

## References

- `layer/core/unix-philosophy.md`
- `layer/core/dependable-rust.md`
- `spec-forge-abstraction.md` (archived) - original forge design
