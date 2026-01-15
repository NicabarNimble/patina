---
id: spec-ref-repo-storage
status: partial
created: 2026-01-14
tags: [spec, ref-repos, database, eventlog, storage]
references: [dependable-rust, unix-philosophy, spec-forge-bulk-fetch]
---

# Spec: Ref Repo Storage Design

**Problem:** Ref repo databases grow unboundedly. claude-code is 224MB with 19,509 forge.issue events for 17,510 actual issues. Git events duplicate what git already stores.

**Goal:** Lean ref repo storage that preserves expensive API data while rebuilding cheap git data on demand.

---

## Background

### Current State

Ref repos use the same eventlog pattern as project repos:

```
eventlog (append-only) → materialized views
```

This causes problems:

| Ref Repo | DB Size | Events | Issues | Waste |
|----------|---------|--------|--------|-------|
| claude-code | 224MB | 20,359 | 17,510 | ~2,000 duplicate events |
| gemini-cli | 176MB | 90,028 | 8,565 | 81,463 git/code events |
| opencode | 112MB | 59,847 | 5,161 | 54,686 git/code events |

### The Insight

**Git data is derived.** The git repository IS the source of truth. Storing git events in eventlog is storing a copy of what git already stores perfectly.

**GH data is cached.** API responses are expensive (rate limits, network) and ephemeral (issues can be deleted, repos made private). Worth preserving.

### Core Values Alignment

**Steenberg:** "Don't store what you can compute"
- Git events can be computed from `git log`
- GH events cannot be computed, only fetched

**Gjengset:** "Rebuild from source is always correct"
- Git is the source → rebuild is authoritative
- GH API is external → cache is valuable

**Unix Philosophy:** "One tool, one job"
- Git does git's job (commit history)
- We do our job (index for search, cache API data)

---

## Design

### Storage Model

```
~/.patina/cache/repos/{name}/.patina/local/data/patina.db

├── eventlog           # ONLY forge events (API responses)
├── forge_issues       # Materialized from eventlog
├── forge_prs          # Materialized from eventlog
├── commits            # DIRECT: rebuilt from git
├── code_fts           # DIRECT: rebuilt from git files
└── scrape_meta        # Track last processed timestamps
```

### Data Classification

| Data Type | Storage | Rebuild Strategy |
|-----------|---------|------------------|
| Git commits | Direct to `commits` | `DELETE` + re-insert from `git log` |
| Code symbols | Direct to `code_fts` | `DELETE` + re-insert from files |
| GH issues | Eventlog → materialized | Append new, dedupe on insert |
| GH PRs | Eventlog → materialized | Append new, dedupe on insert |

### The "Do X" Test

- `scrape git` → "Rebuild git tables from repository" (not "append git events")
- `scrape forge` → "Update cached forge data incrementally" (preserves history)

---

## Implementation

### Phase 1: Git Scrape - Direct Insert (No Eventlog)

```rust
// src/commands/scrape/git/mod.rs

pub fn run_ref_repo(repo_path: &Path, conn: &Connection) -> Result<Stats> {
    // Clear derived tables - git is source of truth
    conn.execute("DELETE FROM commits", [])?;
    conn.execute("DELETE FROM commits_fts", [])?;

    // Rebuild from git (always fresh, always correct)
    let commits = parse_git_log(repo_path)?;

    for commit in &commits {
        insert_commit_direct(conn, commit)?;  // No eventlog
    }

    // Rebuild FTS
    populate_commits_fts(conn)?;

    Ok(Stats { commits: commits.len(), ..Default::default() })
}

fn insert_commit_direct(conn: &Connection, commit: &Commit) -> Result<()> {
    conn.execute(
        "INSERT INTO commits (sha, message, author_name, timestamp, ...)
         VALUES (?1, ?2, ?3, ?4, ...)",
        params![commit.sha, commit.message, ...],
    )?;
    Ok(())
}
```

### Phase 2: Forge Scrape - Eventlog with Deduplication

```rust
// src/commands/scrape/forge/mod.rs

pub fn run_ref_repo(conn: &Connection, reader: &dyn ForgeReader) -> Result<Stats> {
    let issues = reader.list_issues(limit, since)?;

    let mut inserted = 0;
    for issue in &issues {
        // Dedupe on insert - don't create garbage
        if !issue_event_exists(conn, issue.number, &issue.updated_at)? {
            insert_event(conn, "forge.issue", &issue.updated_at, ...)?;
            inserted += 1;
        }
        // Always update materialized view (latest wins)
        upsert_issue(conn, issue)?;
    }

    Ok(Stats { issues: inserted, ..Default::default() })
}

/// Check if we already have this issue at this updated_at timestamp.
/// Prevents duplicate events from repeated scrapes.
fn issue_event_exists(conn: &Connection, number: i64, updated_at: &str) -> Result<bool> {
    let count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM eventlog
         WHERE event_type = 'forge.issue'
           AND json_extract(data, '$.number') = ?1
           AND json_extract(data, '$.updated_at') = ?2",
        params![number, updated_at],
        |row| row.get(0),
    )?;
    Ok(count > 0)
}
```

### Phase 3: Code Scrape - Direct Insert (No Eventlog)

```rust
// src/commands/scrape/code/mod.rs

pub fn run_ref_repo(repo_path: &Path, conn: &Connection) -> Result<Stats> {
    // Clear derived tables
    conn.execute("DELETE FROM code_fts WHERE event_type LIKE 'code.%'", [])?;

    // Parse and insert directly (no eventlog)
    let symbols = parse_code_files(repo_path)?;

    for symbol in &symbols {
        insert_symbol_direct(conn, symbol)?;
    }

    Ok(Stats { symbols: symbols.len(), ..Default::default() })
}
```

### Phase 4: Detect Ref Repo Context

```rust
// src/commands/scrape/mod.rs

fn is_ref_repo(path: &Path) -> bool {
    // Ref repos live in ~/.patina/cache/repos/
    path.to_string_lossy().contains(".patina/cache/repos")
}

pub fn run(config: ScrapeConfig) -> Result<Stats> {
    if is_ref_repo(&config.path) {
        run_ref_repo(config)  // Lean storage model
    } else {
        run_project(config)   // Full eventlog model
    }
}
```

---

## Migration

### Existing Ref Repos

Option A: **Compact on next scrape**
```rust
// First scrape after upgrade detects bloat and compacts
if eventlog_has_git_events(conn)? {
    compact_eventlog(conn)?;  // Remove git events, keep forge
}
```

Option B: **Manual rebuild**
```bash
patina scrape --rebuild  # Drops and rebuilds ref repo database
```

Recommend Option B - simpler, one-time operation.

### Database Schema Changes

None. Same tables, just different population strategy for ref repos.

---

## Behavior Changes

### Before (Current)

```
$ patina scrape git (ref repo)
  Scraping commits...
  Inserted 3,818 events into eventlog    # Wasteful
  Materialized 3,818 commits
```

### After (New)

```
$ patina scrape git (ref repo)
  Rebuilding commits from git...
  Inserted 3,818 commits                 # Direct, no eventlog
```

### Forge Behavior (Unchanged Pattern, Better Dedup)

```
$ patina scrape forge (ref repo)
  Found 17,510 issues
  Inserted 47 new events (17,463 unchanged)  # Dedup working
  Updated 17,510 issues in materialized view
```

---

## Success Criteria

- [x] Git scrape for ref repos inserts directly (no eventlog)
- [x] Code scrape for ref repos inserts directly (no eventlog)
- [x] Forge scrape dedupes on insert (no duplicate events)
- [x] Existing ref repos can be rebuilt with `--rebuild`
- [ ] claude-code database size < 50MB after rebuild (was 224MB) - needs validation
- [x] Project repos unchanged (full eventlog preserved)

---

## Non-Goals

- Changing project repo storage model (eventlog is correct there)
- Migrating existing eventlog data (rebuild is cleaner)
- Incremental git scrape (rebuild is fast enough, always correct)

---

## Future Considerations

### Eventlog History Value

For forge data, eventlog preserves history of how issues changed over time. This could enable:
- "Show me how issue #1234 evolved"
- "What issues were open on date X"

Not implementing now, but the data model supports it.

### Compaction Command

If eventlog grows too large even with dedup:
```bash
patina scrape forge --compact  # Keep only latest event per issue
```

---

## References

- [dependable-rust](../../core/dependable-rust.md) - Don't store what you can compute
- [unix-philosophy](../../core/unix-philosophy.md) - Git does git's job
- [spec-forge-bulk-fetch](spec-forge-bulk-fetch.md) - Bulk fetch reduces API calls
