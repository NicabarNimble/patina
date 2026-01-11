# Spec: Forge Sync Engine

**Status**: Phases 1-2 Complete
**Created**: 2026-01-10
**Session**: 20260110-192723
**Core References**: [dependable-rust](../../core/dependable-rust.md), [unix-philosophy](../../core/unix-philosophy.md), [adapter-pattern](../../core/adapter-pattern.md)

## Problem Statement

When scraping forge data (GitHub issues/PRs) for repositories with significant history, the current implementation hits API rate limits. Scraping opencode's 1215 PR references caused TLS handshake timeouts due to rapid sequential API calls with no pacing.

**Root causes:**
1. Discovery and resolution are coupled - finding `#123` immediately triggers an API call
2. No pacing - sequential calls with zero delay
3. No persistence - sync progress lost on interruption
4. Ambiguous refs - `#123` could be issue OR PR, we blindly try PR first

## Design Principles

Grounded in patina's core patterns:

### From Unix Philosophy

> "One tool, one job, done well. Complex functionality emerges from composition."

**Applied here:**
- **Discover refs** - one tool (scan commits, extract `#123` patterns)
- **Resolve refs** - one tool (fetch from API, handle errors)
- **Pace requests** - one tool (wait between calls, respect limits)

Not a "sync manager" that does everything. Three focused tools composed together.

### From Dependable-Rust

> "Keep your public interface small and stable. Hide implementation in internal."

**Applied here:**
```
src/forge/sync/
├── mod.rs          # Public: sync_forge(), SyncStats
└── internal.rs     # Private: pacing, batching, state management
```

The command sees: `sync_forge(conn, reader) -> Result<SyncStats>`
Everything else is internal.

### From Adapter Pattern

> "Use trait-based adapters to remain agnostic to external systems."

**Applied here:**
- Sync engine uses `&dyn ForgeReader`, not `GitHubReader`
- Works with GitHub today, Gitea/Forgejo tomorrow
- Testable with mock reader that returns canned responses

## Architecture

```
┌────────────────────────────────────────────────────────────────┐
│                     FORGE SYNC (Composed Tools)                │
├────────────────────────────────────────────────────────────────┤
│                                                                │
│   ┌─────────────┐    ┌─────────────┐    ┌─────────────┐       │
│   │  discover   │───►│   backlog   │───►│   resolve   │       │
│   │             │    │             │    │             │       │
│   │ Scan commits│    │ forge_refs  │    │ Fetch API   │       │
│   │ Extract #N  │    │ (pending)   │    │ Wait between│       │
│   │ Check known │    │             │    │ Handle errs │       │
│   └─────────────┘    └─────────────┘    └─────────────┘       │
│         │                   │                  │               │
│         │                   │                  ▼               │
│         │                   │          ┌─────────────┐        │
│         │                   │          │  eventlog   │        │
│         │                   └─────────►│ forge.issue │        │
│         │                              │ forge.pr    │        │
│         └─────────────────────────────►│ forge.ref   │        │
│                                        └─────────────┘        │
└────────────────────────────────────────────────────────────────┘
```

**Three tools, three jobs:**

| Tool | Job | Input | Output |
|------|-----|-------|--------|
| `discover` | Find refs in commits | eventlog commits | forge_refs (pending) |
| `backlog` | Track what needs fetching | forge_refs table | pending ref numbers |
| `resolve` | Fetch from API with pacing | ref number | forge item or error |

### Walk-Back Pattern

Refs are resolved **newest-first**. Recent commits get their PRs resolved immediately; historical refs resolve gradually over subsequent runs.

```
TIME ──────────────────────────────────────────────────►

     OLD                                           NEW
     (resolves later)                    (resolves first)

├─────────────────────────────────────────┼────────────┤
│▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒│████████████│
│         backlog (pending)               │  resolved  │
│         fills in over time              │  first     │
├─────────────────────────────────────────┼────────────┤
                ◄─── direction of resolution ───
```

This ensures current context is always fresh while history fills in over time. If you need historical data urgently, run `--drain` to process the full backlog.

## The "Do X" Test

From dependable-rust: *"Before creating a module, ensure you can clearly state what it does."*

- **forge::sync** - "Sync forge data incrementally with rate limiting"
- **discover_refs** - "Extract #N references from commit messages"
- **resolve_ref** - "Fetch a single issue or PR from forge API"

Each passes the test. No vague "manage" or "handle" verbs.

## Data Model

### Sync State (Minimal)

```sql
-- Track how far we've synced
CREATE TABLE forge_sync_state (
    stream      TEXT PRIMARY KEY,  -- 'refs'
    last_sync   TEXT,              -- ISO timestamp
    pending     INTEGER DEFAULT 0  -- Count of unresolved refs
);
```

### Reference Backlog

```sql
-- Discovered refs awaiting resolution
CREATE TABLE forge_refs (
    repo        TEXT NOT NULL,       -- owner/repo
    ref_number  INTEGER NOT NULL,

    -- What we know
    ref_kind    TEXT DEFAULT 'unknown',  -- 'unknown', 'issue', 'pr'
    discovered  TEXT NOT NULL,       -- When found
    source      TEXT,                -- Commit SHA where found

    -- Resolution status
    resolved    TEXT,                -- When fetched (NULL = pending)
    error       TEXT,                -- Error message if failed

    PRIMARY KEY (repo, ref_number)
);
```

**Why this schema:**
- Inspectable with raw SQL (`sqlite3 patina.db "SELECT * FROM forge_refs WHERE resolved IS NULL"`)
- No JSON blobs - every field queryable
- `ref_kind` is text in DB (simple), enum in Rust (safe)

## Implementation

### The Unbreakable Core (~80 lines)

```rust
// src/forge/sync/mod.rs

mod internal;

use crate::forge::ForgeReader;
use rusqlite::Connection;

/// Stats returned from sync operation
pub struct SyncStats {
    pub discovered: usize,
    pub resolved: usize,
    pub pending: usize,
    pub errors: usize,
}

/// Sync forge data incrementally with rate limiting.
///
/// Discovers refs from commits, resolves them via API with pacing.
/// Safe to interrupt - progress is saved after each item.
///
/// # Example
/// ```no_run
/// let stats = forge::sync::run(&conn, &reader)?;
/// println!("Resolved {}, {} pending", stats.resolved, stats.pending);
/// ```
pub fn run(conn: &Connection, reader: &dyn ForgeReader) -> Result<SyncStats> {
    internal::sync_forge(conn, reader)
}

/// Check sync status without making changes.
pub fn status(conn: &Connection) -> Result<SyncStats> {
    internal::get_status(conn)
}
```

```rust
// src/forge/sync/internal.rs

use std::time::Duration;
use std::thread::sleep;

// ============================================================================
// Constants - visible, not configurable
// ============================================================================

/// Delay between API requests. GitHub recommends 1000ms for mutations,
/// we use 500ms for reads. Conservative but not glacial.
const DELAY_BETWEEN_REQUESTS: Duration = Duration::from_millis(500);

/// Maximum refs to resolve per sync run. Keeps each run bounded.
/// At 500ms delay, 50 refs = ~25 seconds.
const BATCH_SIZE: usize = 50;

// ============================================================================
// Core sync logic
// ============================================================================

pub(crate) fn sync_forge(
    conn: &Connection,
    reader: &dyn ForgeReader,
) -> Result<SyncStats> {
    // Step 1: Discover new refs (instant, local)
    let discovered = discover_refs(conn)?;

    // Step 2: Get pending refs to resolve (newest first - walk-back pattern)
    let pending = get_pending_refs(conn, BATCH_SIZE)?;
    let total_pending = count_pending_refs(conn)?;

    println!("Forge sync: {} pending refs, processing batch of {}",
             total_pending, pending.len());

    // Step 3: Resolve with pacing
    let mut resolved = 0;
    let mut errors = 0;

    for (repo, ref_num) in &pending {
        // Always wait - simple, correct, unbreakable
        sleep(DELAY_BETWEEN_REQUESTS);

        match resolve_ref(conn, reader, repo, *ref_num) {
            Ok(_) => {
                resolved += 1;
                // Progress saved immediately - safe to interrupt
            }
            Err(e) => {
                errors += 1;
                eprintln!("  #{}: {}", ref_num, e);
                // Error recorded - won't retry forever
            }
        }
    }

    Ok(SyncStats {
        discovered,
        resolved,
        pending: total_pending - resolved,
        errors,
    })
}

// ============================================================================
// Backlog - get pending refs, newest first
// ============================================================================

fn get_pending_refs(conn: &Connection, limit: usize) -> Result<Vec<(String, i64)>> {
    let sql = r#"
        SELECT repo, ref_number
        FROM forge_refs
        WHERE resolved IS NULL
        ORDER BY discovered DESC  -- Walk-back: newest first
        LIMIT ?
    "#;

    let mut stmt = conn.prepare(sql)?;
    let refs = stmt.query_map([limit], |row| {
        Ok((row.get(0)?, row.get(1)?))
    })?.filter_map(|r| r.ok()).collect();

    Ok(refs)
}

// ============================================================================
// Discovery - extract refs from commits
// ============================================================================

fn discover_refs(conn: &Connection) -> Result<usize> {
    // Find #N patterns in commit messages not already in forge_refs
    let sql = r#"
        INSERT OR IGNORE INTO forge_refs (repo, ref_number, discovered, source)
        SELECT
            json_extract(data, '$.repo') as repo,
            json_extract(data, '$.parsed.pr_ref') as ref_number,
            datetime('now') as discovered,
            json_extract(data, '$.hash') as source
        FROM eventlog
        WHERE event_type = 'git.commit'
          AND json_extract(data, '$.parsed.pr_ref') IS NOT NULL
    "#;

    let count = conn.execute(sql, [])?;
    Ok(count)
}

// ============================================================================
// Resolution - fetch from API with pacing
// ============================================================================

fn resolve_ref(
    conn: &Connection,
    reader: &dyn ForgeReader,
    repo: &str,
    ref_num: i64,
) -> Result<()> {
    // Check if it's a known issue first (no API call needed)
    if is_known_issue(conn, ref_num)? {
        mark_resolved(conn, repo, ref_num, "issue")?;
        return Ok(());
    }

    // Try as PR first (more common in commit refs)
    match reader.get_pull_request(ref_num) {
        Ok(pr) => {
            insert_pr(conn, &pr)?;
            mark_resolved(conn, repo, ref_num, "pr")?;
            return Ok(());
        }
        Err(_) => {
            // Not a PR - might be an issue we haven't fetched yet
        }
    }

    // Try as issue
    match reader.get_issue(ref_num) {
        Ok(issue) => {
            insert_issue(conn, &issue)?;
            mark_resolved(conn, repo, ref_num, "issue")?;
            Ok(())
        }
        Err(e) => {
            // Neither PR nor issue - record error, move on
            mark_failed(conn, repo, ref_num, &e.to_string())?;
            Err(e)
        }
    }
}
```

### Why This Is Unbreakable

| Threat | Mitigation |
|--------|------------|
| Rate limit | Always wait 500ms between calls |
| Interruption | Progress saved after each item |
| Network error | Error recorded, continues with next |
| Invalid ref | Marked failed, won't retry forever |
| Ambiguous #N | Tries PR then issue, records what it found |
| Large backlog | Bounded batch size (50 per run) |

### Issue vs PR API Costs

```
┌─────────────────────────────────────────────────────────────┐
│                    API CALL BREAKDOWN                       │
├─────────────────────────────────────────────────────────────┤
│                                                             │
│  BATCH OPERATIONS (cheap):                                  │
│  ─────────────────────────                                  │
│  gh issue list --limit 500    → 1 call  → 500 issues       │
│  gh pr list --limit 500       → 1 call  → 500 PRs          │
│                                                             │
│  INDIVIDUAL FETCHES (expensive, need pacing):               │
│  ────────────────────────────────────────────               │
│  gh pr view #123              → 1 call  → 1 PR             │
│  gh issue view #123           → 1 call  → 1 issue          │
│                                                             │
└─────────────────────────────────────────────────────────────┘
```

**Resolution order minimizes API calls:**

1. Check `forge_issues` table (from batch) → **0 calls** if found
2. Try `gh pr view` → **1 call**
3. Fall back to `gh issue view` → **1 call** (only if PR failed)

**Worst case per ref:** 2 API calls (PR miss + issue hit)
**Best case per ref:** 0 API calls (issue already in DB)

The 500ms delay applies to ALL individual fetches, whether PR or issue.

### Type Safety (Jon Gjengset)

```rust
/// Reference kind - enum in Rust, text in SQLite
#[derive(Debug, Clone, Copy)]
pub enum RefKind {
    Unknown,
    Issue,
    PullRequest,
}

impl RefKind {
    fn as_str(&self) -> &'static str {
        match self {
            Self::Unknown => "unknown",
            Self::Issue => "issue",
            Self::PullRequest => "pr",
        }
    }

    fn from_str(s: &str) -> Self {
        match s {
            "issue" => Self::Issue,
            "pr" => Self::PullRequest,
            _ => Self::Unknown,
        }
    }
}
```

Rust code uses the enum (compiler-checked). Database stores strings (inspectable).

## Command Interface

### Updated `scrape forge`

```bash
# Default: discover + resolve one batch (50 refs, ~25 sec)
patina scrape forge

# Just show status, no changes
patina scrape forge --status

# Keep running until backlog is empty (still paced!)
patina scrape forge --drain
```

### How `--drain` Respects Limits

`--drain` doesn't bypass pacing - it just keeps running batches until done:

```rust
pub fn drain(conn: &Connection, reader: &dyn ForgeReader) -> Result<SyncStats> {
    let mut total = SyncStats::default();

    loop {
        let stats = sync_forge(conn, reader)?;  // Still has 500ms delays
        total.resolved += stats.resolved;
        total.errors += stats.errors;

        if stats.pending == 0 {
            break;  // Backlog empty, done
        }

        // Optional: small pause between batches
        println!("Batch complete. {} remaining...", stats.pending);
    }

    Ok(total)
}
```

**Time estimates:**
| Backlog Size | Time at 500ms/req |
|--------------|-------------------|
| 50 refs | ~25 seconds |
| 500 refs | ~4 minutes |
| 1215 refs | ~10 minutes |
| 5000 refs | ~42 minutes |

All well under GitHub's 5,000 requests/hour limit. Slow but unbreakable.

### Example Output

```
$ patina scrape forge
Forge sync: 1215 pending refs, processing batch of 50
  #1201: resolved as PR
  #1198: resolved as PR
  #1195: resolved as issue
  ...
Resolved 48, 2 errors, 1165 pending

$ patina scrape forge --status
Forge sync status:
  Pending: 1165 refs
  Last sync: 2 minutes ago
  Est. completion: ~12 runs (at 50/run)
```

## Testing Strategy

### Mock Adapter for Tests

```rust
struct MockForgeReader {
    prs: HashMap<i64, PullRequest>,
    issues: HashMap<i64, Issue>,
}

impl ForgeReader for MockForgeReader {
    fn get_pull_request(&self, num: i64) -> Result<PullRequest> {
        self.prs.get(&num)
            .cloned()
            .ok_or_else(|| anyhow!("PR not found"))
    }

    fn get_issue(&self, num: i64) -> Result<Issue> {
        self.issues.get(&num)
            .cloned()
            .ok_or_else(|| anyhow!("Issue not found"))
    }
}

#[test]
fn test_sync_resolves_pr_refs() {
    let conn = setup_test_db();
    let reader = MockForgeReader::with_prs(vec![pr(123), pr(456)]);

    // Insert some refs to resolve
    insert_ref(&conn, "owner/repo", 123);
    insert_ref(&conn, "owner/repo", 456);

    let stats = sync::run(&conn, &reader).unwrap();

    assert_eq!(stats.resolved, 2);
    assert_eq!(stats.pending, 0);
}
```

### Integration Test (Real API, Careful)

```rust
#[test]
#[ignore]  // Only run manually - hits real API
fn test_real_github_sync() {
    let conn = setup_test_db();
    let reader = GitHubReader::new("rust-lang/rust").unwrap();

    insert_ref(&conn, "rust-lang/rust", 1);  // First ever PR

    let stats = sync::run(&conn, &reader).unwrap();
    assert_eq!(stats.resolved, 1);
}
```

## Implementation Phases

### Phase 1: Unbreakable MVP ✅

- [x] Add `forge_refs` table
- [x] Implement `discover_refs` (SQL insert from eventlog)
- [x] Implement `resolve_ref` with 500ms delay
- [x] Add `--status` flag to `scrape forge`
- [x] Wire into existing `scrape forge` command

**Outcome**: No more rate limit crashes. Resumable. Inspectable.

### Phase 2: Polish ✅

- [x] Add `--drain` flag for continuous sync
- [x] Better progress output (batch progress between runs)
- [x] Add `get_issue` to ForgeReader trait
- [x] Skip refs that are clearly issues (from existing forge_issues)

**Outcome**: Pleasant to use.

### Phase 3: Optimization (Maybe Never)

- [ ] GraphQL batching (if 500ms is too slow)
- [ ] Adaptive rate limiting (if GitHub changes limits)
- [ ] Parallel resolution (if we need speed)

**Outcome**: Faster. But only if Phase 1-2 prove insufficient.

## What We're NOT Building

Staying true to Unix philosophy - avoiding feature creep:

| Feature | Why Not |
|---------|---------|
| Webhooks | Different tool - real-time vs batch |
| Full PR history | Scrape what commits reference, not everything |
| Comment sync | Different data type, different tool |
| Cross-repo sync | One repo at a time, compose if needed |
| Config file | Constants are fine, visible in code |

## Handling Incomplete Data

Until the backlog is fully resolved, some queries may return incomplete results. Rather than complex query-driven fetching, we handle this simply:

**In scry/query output:**
```
Results for "authentication changes":
  PR #1201: Add OAuth2 support (2026-01-08)
  PR #1195: Fix token refresh (2026-01-05)
  ...

  Note: 847 forge refs pending sync. Run `patina scrape forge --drain` for complete history.
```

**Implementation:**
```rust
fn maybe_warn_incomplete(conn: &Connection) -> Option<String> {
    let pending = count_pending_refs(conn).ok()?;
    if pending > 0 {
        Some(format!("{} forge refs pending sync", pending))
    } else {
        None
    }
}
```

Simple. Honest. User decides if they need more.

## Open Questions

1. **Should `scrape forge` auto-run discovery + resolution?**
   - Current plan: Yes, both in one command
   - Alternative: Separate `scrape forge discover` and `scrape forge resolve`
   - Leaning: Combined is simpler for users

2. **What about refs to deleted PRs/issues?**
   - Current plan: Mark as error after first failure
   - Alternative: Retry N times before giving up
   - Leaning: Fail fast, user can investigate manually

## Pre-Implementation Fixes

Before implementing the sync engine, address these gaps in existing forge code:

### 1. Add `get_issue()` to ForgeReader Trait

```rust
// src/forge/mod.rs
pub trait ForgeReader {
    fn list_issues(&self, limit: usize, since: Option<&str>) -> Result<Vec<Issue>>;
    fn list_pull_requests(&self, limit: usize, since: Option<&str>) -> Result<Vec<PullRequest>>;
    fn get_pull_request(&self, number: i64) -> Result<PullRequest>;
    fn get_issue(&self, number: i64) -> Result<Issue>;  // ADD THIS
}
```

### 2. Fix FTS Duplicate Inserts

```sql
-- Current (creates duplicates on re-run):
INSERT INTO code_fts (symbol_name, file_path, content, event_type) ...

-- Fixed:
DELETE FROM code_fts WHERE event_type = 'forge.issue';
INSERT INTO code_fts ...

-- Or: add unique constraint and use INSERT OR IGNORE
```

### 3. Enhanced ScrapeStats

```rust
pub struct ScrapeStats {
    // Existing
    pub items_processed: usize,
    pub time_elapsed: Duration,
    pub database_size_kb: u64,

    // Add for observability
    pub refs_discovered: usize,
    pub refs_resolved: usize,
    pub refs_failed: usize,
    pub refs_pending: usize,
    pub cache_hits: usize,  // Issues already in DB
}
```

### 4. Error Classification

```rust
pub enum ResolveError {
    NotFound,           // 404 - PR/issue doesn't exist (permanent)
    RateLimited,        // 429 - retry later
    NetworkError(String), // Transient - retry
    AuthExpired,        // Re-auth needed
}

// In forge_refs table:
// error TEXT  ->  error_kind TEXT, error_message TEXT
```

## Session Notes

Discovered during session 20260110-181504 when scraping opencode:
- 6945 commits with ~1215 PR references
- Hit rate limits after ~200 sequential `gh pr view` calls
- TLS handshake timeout errors
- Many `#123` refs were actually issues, not PRs

Design reviewed through lens of:
- Jon Gjengset (Rust correctness) - type-safe states, APIs that prevent misuse
- Eskil Steenberg (lasting programs) - simplest thing that works, debuggable, no magic
