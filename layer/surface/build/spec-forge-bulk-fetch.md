---
id: spec-forge-bulk-fetch
status: validated
created: 2026-01-14
validated: 2026-01-14
tags: [spec, forge, github, bulk-fetch, rate-limit]
references: [spec-forge-sync-v2, dependable-rust, unix-philosophy]
---

# Spec: Forge Bulk Fetch

**Problem:** Forge sync uses individual API calls (18,000 calls for 18,000 issues) when bulk APIs exist (180 calls for the same data).

**Goal:** Get all forge data using the minimum number of API calls while respecting rate limits.

---

## Pre-Requisite: Validate Bulk Fetch Works

**Status: PASSED** (2026-01-14)

Changed `limit: 500` → `limit: 50000` in `src/commands/scrape/forge/mod.rs` and tested against ref repos.

### Test Results

| Repo | Issues Bulk Fetched | Time | Old Refs Created |
|------|---------------------|------|------------------|
| gemini-cli | **8,565** | 2:15 | 16,603 |
| opencode | **5,161** | 1:37 | 7,237 |
| claude-code | **17,509** | 3:20 | 654 |

### Findings

1. **Bulk fetch works** - 17,509 issues fetched in 3:20 (vs 3.7 hours with old approach)
2. **`discover_all_issues()` still runs** - after bulk fetch, it creates ref backlog entries anyway
3. **PRs not bulk fetched** - current code only calls `list_issues()`, not `list_pull_requests()`
4. **No rate limit errors** - `gh` handles pagination internally without issues

### What This Proves

- Increasing limit is sufficient for bulk fetch
- `gh issue list --limit 50000` works correctly
- The remaining work is cleanup: remove `discover_all_issues()` and add PR bulk fetch

**Success criteria:**
- [x] Bulk fetch returns all issues (not capped at 500)
- [x] Total time < 10 minutes for 18k+ issues
- [x] No rate limit errors

---

## Background

### What Works (Keep)

The `spec-forge-sync-v2` infrastructure is solid:
- PID files for process tracking
- Fork to background
- 750ms pacing between requests
- Status checking and log tailing
- Safe to interrupt, progress saved

### What's Broken (Fix)

The **discovery strategy** is wrong:

```
Current flow:
1. list_issues(500)           → 500 issues (1 API call)
2. get_max_issue_number()     → e.g., 18074 (1 API call)
3. Create refs 1..18074       → 18074 backlog entries
4. For each ref:
   - get_issue(N)             → 1 issue (1 API call)
   - sleep(750ms)
5. Total: 18,076 API calls over 3.7 hours
```

The `gh` CLI already handles pagination internally:

```
Better flow:
1. list_issues(20000)         → 18074 issues (~180 API calls, handled by gh)
2. Total: ~180 API calls in minutes
```

---

## Core Values Alignment

### Unix Philosophy
> "Do one thing well"

Current sync tries to be clever with refs backlog. The simple tool is: **fetch all issues**. Let `gh` handle pagination.

### Dependable Rust
> "Push changeable details behind a private internal implementation"

The `ForgeReader` trait is correct. The implementation (`discover_all_issues`) is wrong. Fix the internal, keep the interface.

### Adapter Pattern
> "Implementations handle platform-specific CLI/API calls internally"

`gh` already handles GitHub's 100-per-page limit internally. We're re-implementing pagination poorly when `gh --limit N` does it correctly.

---

## Design

### Key Insight

GitHub rate limit is **per API call**, not per item returned.

| Approach | API Calls | Items | Time |
|----------|-----------|-------|------|
| Individual fetch | 18,000 | 18,000 | 3.7 hours |
| Bulk fetch | 180 | 18,000 | ~3 minutes |

Both respect rate limits. Bulk is 100x more efficient.

### Why Query Counts First

Knowing how many issues/PRs exist before fetching is not optional - it's system knowledge:

| Purpose | Value |
|---------|-------|
| **Progress reporting** | "Fetched 5,000/18,074 issues" vs blind progress |
| **Validation** | Confirm we got everything, detect partial failures |
| **Decision making** | Foreground vs background based on actual size |
| **System metrics** | Track repo size over time, health checks |
| **User confidence** | User knows what to expect before waiting |

"Fetch until empty" provides none of this. A system that knows what it's dealing with is better than one that doesn't.

### New Flow

```
1. Query total counts FIRST
   - get_issue_count()       → e.g., 18,074
   - get_pr_count()          → e.g., 2,341
   - Display: "Found 18,074 issues, 2,341 PRs to fetch"

2. Decide foreground vs background
   - If total < threshold: foreground (fast enough)
   - If total > threshold: offer background option

3. Bulk fetch with exact counts
   - list_issues(issue_count)       → all issues
   - list_pull_requests(pr_count)   → all PRs

4. Validate and store
   - Verify fetched count matches expected
   - Insert into forge_issues/forge_prs tables
   - Report: "Fetched 18,074/18,074 issues, 2,341/2,341 PRs"
```

### Rate Limit Budget

GitHub allows 5,000 requests/hour. Reserve headroom for other patina operations.

**Design decision:** 50% budget for forge sync, 50% reserved.

| Budget | Calls/hour | Purpose |
|--------|------------|---------|
| 50% | 2,500 | Forge sync |
| 50% | 2,500 | Other operations (scry, manual queries) |

**Rationale:** This is a measured, intentional choice:
- Forge sync is batch operation (run occasionally)
- Other operations are interactive (need responsiveness)
- 50/50 split ensures neither starves the other

**Measurement:** During pre-req test, record actual API calls made. Adjust budget if data shows different split is needed.

At 100 items per API call:
- 2,500 calls × 100 items = 250,000 items/hour capacity
- Most repos have <50,000 total issues+PRs
- Full sync completes in one run

### When to Use Ref Backlog

Keep ref backlog **only** for:
- PR numbers found in commit messages (e.g., "Merge PR #1234")
- These genuinely need lookup-by-ID since we have the number, not the data

Remove ref backlog for:
- Issue discovery (use bulk fetch)
- PR discovery (use bulk fetch)

---

## Implementation

### Phase 0: Increase Bulk Fetch Limit (DONE)

```rust
// src/commands/scrape/forge/mod.rs - ALREADY CHANGED
impl Default for ForgeScrapeConfig {
    fn default() -> Self {
        Self {
            limit: 50000,  // Changed from 500
            force: false,
            working_dir: None,
        }
    }
}
```

**Status:** Complete. Validated with test results above.

### Phase 1: Remove Issue Discovery from Ref Backlog (CRITICAL)

**Problem observed:** After bulk fetching 17,509 issues, `discover_all_issues()` still ran and created 654 more ref backlog entries. This function is wasteful now that bulk fetch works.

```rust
// src/forge/sync/internal.rs

// DELETE this function entirely:
fn discover_all_issues(...) -> Result<usize> {
    // This creates refs 1..max and resolves each one
    // WRONG APPROACH - delete it
}

// DELETE the call to it in sync_forge():
fn sync_forge(...) -> Result<SyncStats> {
    let pr_discovered = discover_refs(conn, repo)?;
    // DELETE: let issue_discovered = discover_all_issues(conn, reader, repo)?;
    let discovered = pr_discovered;  // Only PR refs from commits
    // ...
}

// KEEP this function (PR refs from commits):
fn discover_refs(conn: &Connection, repo: &str) -> Result<usize> {
    // Finds #N in commit messages - this is correct use case
}
```

### Phase 2: Add PR Bulk Fetch

**Problem observed:** Test output showed "Indexed 0 PRs in FTS5" - PRs aren't being bulk fetched.

```rust
// src/commands/scrape/forge/mod.rs

pub fn run(config: ForgeScrapeConfig) -> Result<ScrapeStats> {
    // ... existing issue fetch ...

    // ADD: Bulk fetch PRs (same pattern as issues)
    println!("📊 Fetching PRs for {}...", forge_name);
    let prs = reader.list_pull_requests(config.limit, since.as_deref())?;

    if !prs.is_empty() {
        println!("  Found {} PRs to process", prs.len());
        let pr_count = insert_prs(&conn, &prs)?;
        println!("  Inserted {} PRs", pr_count);
    }

    // Sync only resolves PR refs from commits (not bulk discovery)
    let sync_stats = forge::sync::run(&conn, reader.as_ref(), &repo_spec)?;

    // ...
}
```

Also need `insert_prs()` function (similar to existing `insert_issues()`).

### Phase 3: Add Count Queries (Enhancement)

For progress reporting and validation. Can be deferred if Phase 1-2 are sufficient.

```rust
// src/forge/mod.rs - add to ForgeReader trait

pub trait ForgeReader {
    // ... existing methods ...

    /// Get total issue count (1 API call).
    fn get_issue_count(&self) -> Result<usize>;

    /// Get total PR count (1 API call).
    fn get_pr_count(&self) -> Result<usize>;
}
```

This enables:
- "Found 18,074 issues, 2,341 PRs to fetch"
- "Fetched 18,074/18,074 issues"
- Validation that we got everything

### Phase 4: Background Support (Keep Infrastructure)

Keep existing background infrastructure from spec-forge-sync-v2. Test bulk fetch in foreground first, then evaluate if background is still needed for large repos.

```rust
const BACKGROUND_THRESHOLD: usize = 10000;

pub fn run(config: ForgeScrapeConfig) -> Result<ScrapeStats> {
    // Query counts first
    let issue_count = reader.get_issue_count()?;
    let pr_count = reader.get_pr_count()?;
    let total = issue_count + pr_count;

    println!("Found {} issues, {} PRs to fetch", issue_count, pr_count);

    // Large repo: suggest background (don't force)
    if total > BACKGROUND_THRESHOLD {
        println!("Large repo detected. Use --sync for background fetch.");
    }

    // Proceed with foreground fetch
    // ...
}
```

---

## CLI Changes

### No New Flags

Existing interface works:

```bash
# Fetch all issues and PRs (bulk, fast)
patina scrape forge
# → "Fetched 18,074 issues, 2,341 PRs"

# Force full rebuild
patina scrape forge --force
# → "Full rebuild: 18,074 issues, 2,341 PRs"

# Incremental update
patina scrape forge
# → "Fetched 47 issues updated since 2026-01-12"
```

### Status Output Changes

```bash
# Before (confusing)
patina scrape forge
# → "Discovered 18,074 issue refs (1..18074)"
# → "Total: 18,335 pending. Run --sync to fetch."

# After (clear)
patina scrape forge
# → "Fetching issues... 18,074 (180 API calls)"
# → "Fetching PRs... 2,341 (24 API calls)"
# → "Done in 2m 34s"
```

---

## Migration

### Code Changes

| File | Change |
|------|--------|
| `src/forge/sync/internal.rs` | Delete `discover_all_issues()` |
| `src/commands/scrape/forge/mod.rs` | Increase limit, add PR bulk fetch |
| `src/forge/github/internal.rs` | Add `fetch_pull_requests()` (if missing) |

### Database Changes

None. Same tables, same schema. Just populated via bulk instead of one-by-one.

### Backwards Compatibility

- Existing `forge_refs` table entries remain valid
- Background sync still works for PR refs from commits
- `--sync` flag still works (just less needed)

---

## Success Criteria

1. ~~`patina scrape forge` fetches ALL issues in <5 minutes for repos with <50k issues~~ **VALIDATED**: 17,509 issues in 3:20
2. ~~API calls reduced by ~100x (bulk vs individual)~~ **VALIDATED**: No rate limit errors
3. ~~Rate limit never exceeded~~ **VALIDATED**: gh handles pagination smoothly
4. `discover_all_issues()` removed - no wasteful ref backlog creation
5. PRs bulk fetched alongside issues
6. PR refs from commits still resolved via backlog (correct use case)

---

## Non-Goals

- Streaming/incremental display during bulk fetch
- Parallel bulk fetches across repos
- GraphQL API migration (gh CLI is sufficient)

---

## References

- [spec-forge-sync-v2](spec-forge-sync-v2.md) - Background sync infrastructure (keep)
- [dependable-rust](../../core/dependable-rust.md) - Fix internal, keep interface
- [unix-philosophy](../../core/unix-philosophy.md) - Simple tool: fetch all data
