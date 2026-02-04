---
type: fix
id: repo-update-with-issues
status: complete
created: 2026-02-04
sessions:
  origin: 20260204-163132
related:
  - src/commands/repo/mod.rs
  - src/commands/repo/internal.rs
beliefs:
  - spec-before-code
---

# fix: `repo update --with-issues` missing flag

> Cannot enable forge issue sync on repos registered without `--with-issues`.

## Problem

`patina repo add --with-issues` fetches GitHub issues at registration time. But if a repo was added without `--with-issues`, there is no way to fetch issues later:

- `repo add --with-issues <existing>` → rejects: "already registered"
- `repo update` → has no `--with-issues` flag
- No standalone `scrape forge --repo` command

Result: 17 registered repos, none with issues synced.

## Fix

Add `--with-issues` flag to `repo update`. When set, call `scrape_github_issues()` after the normal git pull + rescrape cycle.

### Changes

1. **`src/commands/repo/mod.rs`**
   - Add `#[arg(long)] with_issues: bool` to `RepoCommands::Update`
   - Thread through `RepoCommand::Update` and `execute()`
   - Update `update()` and `update_all()` signatures

2. **`src/commands/repo/internal.rs`**
   - `update_repo(name, oxidize, with_issues)` — call `scrape_github_issues()` when flag set
   - `update_all_repos(oxidize, with_issues)` — pass through

### Exit Criteria

- [x] `patina repo update --with-issues anthropics/claude-code` fetches and indexes issues (22,271 issues + 458 PRs)
- [x] `patina scry` with `include_issues` returns forge issue results from ref repos
- [x] Existing `repo update` (without flag) behavior unchanged
- [x] `cargo clippy --workspace` clean, `cargo test --workspace` passing, `cargo fmt` clean
