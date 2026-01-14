---
id: spec-ref-repo-health
status: design
created: 2026-01-14
tags: [spec, ref-repo, forge, issues, rate-limit, adapter-tracking]
references: [spec-forge-sync-v2, spec-patina-local, adapter-pattern]
---

# Spec: Ref Repo Health

**Problem:** Ref repos (external repos we track for context) have broken data paths, inefficient issue sync, and no mechanism for tracking adapter evolution.

**Goal:** Make ref repos reliable, efficient, and useful for tracking upstream changes (especially claude-code).

---

## Problems Identified

### 1. Path Migration Incomplete

Ref repos in `~/.patina/cache/repos/*/` still use old path:
- **Old:** `.patina/data/patina.db` (orphaned data)
- **New:** `.patina/local/data/patina.db` (code expects this)

Result: Old issue data lost, new scrapes start from scratch.

### 2. Inefficient Issue Discovery

Current flow for a repo with 18,000 issues:
```
1. gh issue list --limit 500     → 500 issues (30 seconds)
2. Discover refs 1..18074        → Creates 18074 backlog entries
3. Resolve one-by-one @ 750ms    → 3.7 HOURS
```

This is insane. GitHub provides bulk APIs that return 500+ issues per call.

### 3. No Retry on Rate Limits

Current code:
- Fixed 750ms delay (conservative but slow)
- No detection of 429/rate limit errors
- No exponential backoff
- Silent failures

### 4. No Adapter Change Tracking

We track claude-code issues but don't:
- Surface changes relevant to our adapters
- Maintain a changelog of upstream evolution
- Alert when new features/breaking changes appear

---

## Design

### Phase 1: Fix Paths

Run migration on all cached ref repos:

```bash
# resources/scripts/migrate-ref-repos.sh
for repo in ~/.patina/cache/repos/*/; do
    if [ -d "$repo/.patina/data" ] && [ ! -d "$repo/.patina/local" ]; then
        mkdir -p "$repo/.patina/local"
        mv "$repo/.patina/data" "$repo/.patina/local/"
        echo "Migrated: $repo"
    fi
done
```

### Phase 2: Smart Issue Sync

**Insight:** We don't need to resolve every issue number. We need issues, not refs.

**New flow:**
```
1. gh issue list --limit 5000 --state all  → Bulk fetch (paginated)
2. Insert directly into forge_issues        → No backlog
3. Track last_updated for incremental       → Only fetch changed
```

**Keep ref backlog for:** PR refs found in commit messages (these need resolution).

**Skip ref backlog for:** Issues (use bulk API instead).

```rust
// src/commands/scrape/forge/mod.rs

pub fn run(config: ForgeScrapeConfig) -> Result<ScrapeStats> {
    // BULK: Fetch issues directly (no ref backlog)
    let issues = if config.force {
        reader.list_issues(5000, None)?  // All issues, paginated
    } else {
        let since = get_last_scrape(&conn)?;
        reader.list_issues(1000, since.as_deref())?  // Updated since
    };
    insert_issues(&conn, &issues)?;

    // REF BACKLOG: Only for PRs referenced in commits
    let pr_refs = discover_pr_refs(&conn, repo)?;
    // ... resolve PRs with pacing ...
}
```

### Phase 3: Rate Limit Awareness

Add basic retry with backoff on rate limit errors:

```rust
// src/forge/github/internal.rs

fn fetch_with_retry<T, F>(mut fetch_fn: F) -> Result<T>
where
    F: FnMut() -> Result<T>,
{
    let mut attempts = 0;
    let max_attempts = 3;
    let mut delay = Duration::from_secs(60);  // Start with 1 minute

    loop {
        match fetch_fn() {
            Ok(result) => return Ok(result),
            Err(e) if is_rate_limit_error(&e) && attempts < max_attempts => {
                attempts += 1;
                eprintln!("Rate limited. Waiting {:?} (attempt {}/{})",
                         delay, attempts, max_attempts);
                std::thread::sleep(delay);
                delay *= 2;  // Exponential backoff
            }
            Err(e) => return Err(e),
        }
    }
}

fn is_rate_limit_error(e: &anyhow::Error) -> bool {
    let msg = e.to_string().to_lowercase();
    msg.contains("rate limit") || msg.contains("403") || msg.contains("429")
}
```

### Phase 4: Adapter Change Tracking

New table for tracking observed changes:

```sql
CREATE TABLE adapter_observations (
    id INTEGER PRIMARY KEY,
    adapter TEXT NOT NULL,           -- 'claude', 'gemini', 'opencode'
    observed_at TEXT NOT NULL,       -- When we noticed
    source_type TEXT NOT NULL,       -- 'issue', 'changelog', 'code', 'manual'
    source_ref TEXT,                 -- Issue URL, commit SHA, etc.
    observation TEXT NOT NULL,       -- What we noticed
    impact TEXT,                     -- How it affects patina
    tags TEXT,                       -- JSON array: ['breaking', 'feature', 'deprecation']
    session_id TEXT                  -- Link to session where discovered
);

CREATE INDEX idx_adapter_obs_adapter ON adapter_observations(adapter);
CREATE INDEX idx_adapter_obs_date ON adapter_observations(observed_at DESC);
```

New MCP tool for querying:

```rust
// In scry or new tool
"adapter_changes" => {
    // Query recent observations for an adapter
    // Filter by tags, date range
    // Surface in context when working on adapter code
}
```

---

## CLI Changes

### `patina scrape forge` (updated)

```bash
# Bulk issue fetch (fast, efficient)
patina scrape forge
# → Fetched 523 issues (incremental since 2026-01-12)
# → PR refs: 47 pending (from commits)

# Force full refresh
patina scrape forge --full
# → Fetched 18,074 issues (full rebuild)

# Sync PR refs in background (unchanged)
patina scrape forge --sync
```

### `patina adapter observe` (new)

```bash
# Record an observation manually
patina adapter observe claude "Skills now support frontmatter schema" \
    --source "https://github.com/anthropics/claude-code/issues/17000" \
    --impact "Should migrate /session-* to Skills format" \
    --tags breaking,feature

# List recent observations
patina adapter changes claude --since 30d

# Auto-scan issues for potential changes (future)
patina adapter scan claude-code --keywords "breaking,deprecat,removed"
```

---

## Implementation Phases

### Phase 1: Path Migration (Quick Fix)
- [ ] Create `migrate-ref-repos.sh` script
- [ ] Run on `~/.patina/cache/repos/*/`
- [ ] Update `patina repo add` to use new path from start

### Phase 2: Bulk Issue Sync
- [ ] Change `scrape forge` to use bulk list_issues
- [ ] Remove issue discovery from forge_refs backlog
- [ ] Keep PR ref backlog (commit-based discovery)
- [ ] Update incremental logic to use updated_at

### Phase 3: Rate Limit Handling
- [ ] Add retry wrapper in github/internal.rs
- [ ] Detect 429/403 rate limit errors
- [ ] Exponential backoff (60s → 120s → 240s)
- [ ] Log rate limit events

### Phase 4: Adapter Observations
- [ ] Add adapter_observations table to schema
- [ ] Create `patina adapter observe` command
- [ ] Create `patina adapter changes` query command
- [ ] Integrate with session notes (auto-tag #adapter-change)

---

## Success Criteria

1. `patina repo add` creates working database at correct path
2. `patina scrape forge` fetches 18,000+ issues in <5 minutes (not hours)
3. Rate limit errors don't cause silent failures
4. `patina adapter changes claude` shows recent observations
5. Session notes tagged #adapter-change flow to observations table

---

## Non-Goals

- Automatic breaking change detection (requires NLP/classification)
- Cross-repo rate limit coordination (each repo syncs independently)
- Real-time webhook updates (polling is sufficient)

---

## References

- [spec-forge-sync-v2](spec-forge-sync-v2.md) - Background sync design (keep for PR refs)
- [spec-patina-local](spec-patina-local.md) - Path migration pattern
- [adapter-pattern](../../core/adapter-pattern.md) - Trait-based adapter design
