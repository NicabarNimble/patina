# Spec: Unified Forge Sync

**Status:** Implementation (Phase 2)
**Created:** 2026-01-11
**Revised:** 2026-01-11
**Origin:** Session 20260111-083741 (understanding UI changes)

---

## Problem (Phase 1 - DONE)

`scrape forge` only worked on current project. For ref repos, `--with-issues` only worked at `repo add` time.

**Phase 1 Solution:** Added `--repo <name>` flag. Commit `2f8208bc`.

---

## Problem (Phase 2 - Current)

Issue fetching and PR fetching use different models:

| Aspect | PRs | Issues |
|--------|-----|--------|
| Discovery | From commit refs (#123) | None - fetch 500 newest |
| Backlog | `forge_refs` table | None |
| Rate limiting | 50/batch, 500ms delay | None |
| Full sync | `--drain` until empty | Cannot get older than 500 |

This is inconsistent. Both should use the backlog model.

---

## Solution

**Unify issue and PR fetching under the same backlog model.**

### Current Flow (PRs)
```
git.commit → discover pr_refs → forge_refs (pending)
                                      ↓
                            resolve 50/batch with pacing
                                      ↓
                            --drain until empty
```

### New Flow (Issues + PRs)
```
First run:
  1. Get max issue number from GitHub API (one call)
  2. Populate forge_refs with 1..max_issue (all as pending, kind='issue')
  3. Resolve in batches via existing infrastructure

Subsequent runs:
  1. Check for new issues (numbers > max we know)
  2. Add new ones to forge_refs
  3. Resolve pending
```

---

## Design

### Core Values

| Value | Application |
|-------|-------------|
| unix-philosophy | Extend existing sync, don't create parallel system |
| dependable-rust | Add to internal.rs, keep public interface unchanged |
| Eskil/Gjengset | Simple: populate backlog, let existing code resolve |

### Changes

| File | Change |
|------|--------|
| `src/forge/sync/internal.rs` | Add `discover_all_issues()` function |
| `src/forge/github/internal.rs` | Add `get_max_issue_number()` function |
| `src/forge/mod.rs` | Add `get_issue_count()` to ForgeReader trait |

### Key Insight

We don't need to fetch all 17,000 issues in one call. We just need to know the range (1 to max_number), then populate forge_refs with those numbers. The existing batch/pacing infrastructure does the rest.

---

## Implementation

### 1. ForgeReader trait extension

```rust
// src/forge/mod.rs
pub trait ForgeReader {
    // ... existing methods ...

    /// Get the highest issue number (for backlog population)
    fn get_max_issue_number(&self) -> Result<i64>;
}
```

### 2. GitHub implementation

```rust
// src/forge/github/internal.rs
pub(crate) fn get_max_issue_number(repo: &str) -> Result<i64> {
    // gh issue list --limit 1 --json number
    // Returns the newest issue number
}
```

### 3. Discovery function

```rust
// src/forge/sync/internal.rs
fn discover_all_issues(conn: &Connection, reader: &dyn ForgeReader, repo: &str) -> Result<usize> {
    // Get max issue number from API
    let max_num = reader.get_max_issue_number()?;

    // Get what we already have
    let known_max: i64 = conn.query_row(
        "SELECT COALESCE(MAX(ref_number), 0) FROM forge_refs
         WHERE repo = ?1 AND ref_kind = 'issue'",
        [repo], |row| row.get(0)
    )?;

    // Insert new issue refs (known_max+1 to max_num)
    let mut count = 0;
    for num in (known_max + 1)..=max_num {
        conn.execute(
            "INSERT OR IGNORE INTO forge_refs (repo, ref_number, ref_kind, discovered)
             VALUES (?1, ?2, 'issue', datetime('now'))",
            params![repo, num]
        )?;
        count += 1;
    }

    Ok(count)
}
```

### 4. Integration

Call `discover_all_issues()` at start of sync, alongside existing `discover_refs()` (for PRs).

---

## Rate Limit Math

For claude-code with ~17,500 issues:
- At 50/batch with 500ms delay = 25 seconds per batch
- 17,500 / 50 = 350 batches
- 350 × 25s = ~2.4 hours total

With `--drain`, this runs to completion. Without, it processes 50 per invocation.

**Acceptable:** This is a one-time backfill. Incremental updates after are fast.

---

## Success Criteria

1. `patina scrape forge --repo claude-code --drain` eventually gets ALL issues
2. `--status` shows accurate pending count for issues
3. PR discovery still works (commit refs)
4. Rate limiting respected (no API hammering)
5. Safe to interrupt (progress saved per item)

---

## References

- Phase 1 commit: `2f8208bc` (--repo flag)
- `layer/core/unix-philosophy.md`
- `layer/core/dependable-rust.md`
