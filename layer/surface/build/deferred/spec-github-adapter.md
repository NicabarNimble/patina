# Spec: GitHub Adapter

**Status:** Backlog (Phase 1 Partial)
**Location:** `src/commands/scrape/github/`

---

> **Why Deferred:**
>
> Phase 1 partially implemented: basic issue scraping works via `patina repo add <url> --with-issues`.
> Bounty detection was removed (over-engineered for current needs).
>
> **Remaining work not prioritized:**
> - PR/discussion indexing
> - Semantic search over issues
> - Integration with scry results
>
> **Resume trigger:** When GitHub issues become a meaningful knowledge source for projects using patina.

---

## Purpose

Index GitHub metadata (issues, PRs, discussions) for context enrichment. Currently supports basic issue scraping and FTS5 search.

---

## Architecture

```
patina repo add <url> --with-issues
         │
         ├─> Git Clone ─────────────────┐
         │                              │
         └─> GitHub Fetch ──────────────┤
             (gh issue list --json)     │
                    │                   │
                    ▼                   ▼
             ┌─────────────────────────────────┐
             │         patina.db               │
             │  ├─ eventlog (github.issue)     │
             │  ├─ github_issues (mat. view)   │
             │  └─ code_fts (FTS5 search)      │
             └─────────────────────────────────┘
                           │
                           ▼
             ┌─────────────────────────────────┐
             │         patina scry             │
             │  --include-issues               │
             └─────────────────────────────────┘
```

**Key insight:** Issues and code share the same semantic space via E5 embeddings.

---

## Current Implementation (Phase 1)

### What Works

| Feature | Status | Location |
|---------|--------|----------|
| `gh issue list` fetching | ✅ | `github/mod.rs:fetch_issues()` |
| `github_issues` table | ✅ | `github/mod.rs:create_materialized_views()` |
| FTS5 indexing | ✅ | `github/mod.rs:populate_fts5_github()` |
| Incremental updates | ✅ | `github/mod.rs:get_last_scrape()` |
| `--with-issues` flag on `repo add` | ✅ | `src/commands/repo/` |

### What's Missing (Deferred)

| Feature | Status | Issue |
|---------|--------|-------|
| Bounty detection | 🗑️ | Removed 2025-12-06, stub returns false. Will return as plugin. |
| Expose bounty fields in ScryResult | ❌ | Schema preserved but not populated |
| `--label` filter in scry | ❌ | Documented but not implemented |
| `repo update` calls github scraper | ❌ | Currently skipped |
| `--sort bounty` option | ❌ | Not started |
| `--all-repos` cross-repo search | ❌ | Not started |

---

## Schema

```sql
-- Materialized view (from eventlog github.issue events)
CREATE TABLE github_issues (
    number INTEGER PRIMARY KEY,
    title TEXT NOT NULL,
    body TEXT,
    state TEXT NOT NULL,          -- open, closed
    labels TEXT,                  -- JSON array of label names
    author TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    closed_at TEXT,
    url TEXT NOT NULL,
    is_bounty INTEGER DEFAULT 0,
    bounty_amount TEXT,
    bounty_provider TEXT,         -- algora, dorahacks, ethglobal, etc.
    bounty_currency TEXT,         -- USD, USDC, ETH, STRK
    event_seq INTEGER,            -- Link to eventlog
    FOREIGN KEY (event_seq) REFERENCES eventlog(seq)
);

-- FTS5 integration (shared with code)
INSERT INTO code_fts (symbol_name, file_path, content, event_type)
SELECT title, url, body, 'github.issue' FROM github_issues;
```

---

## CLI Interface

### Current (Working)

```bash
# Add repo with issues
patina repo add dojoengine/dojo --with-issues

# Scrape issues for existing repo
patina scrape github --repo owner/repo

# Search includes issues via FTS5
patina scry "entity spawning" --repo dojo --include-issues
```

### Planned (Not Implemented)

```bash
# Filter by label
patina scry "bounty" --repo dojo --include-issues --label bounty

# Filter by state
patina scry "testing" --repo dojo --include-issues --state open

# Sort by bounty amount
patina scry "bounty" --repo dojo --include-issues --sort bounty

# Cross-repo search
patina scry "bounty cairo" --all-repos --include-issues
```

---

## Bounty Detection (REMOVED)

**Status:** Removed from core on 2025-12-06. Will return as plugin when module system is designed.

The `detect_bounty()` function in `github/mod.rs` is now a stub that always returns `is_bounty: false`. The database schema preserves bounty columns for future compatibility:

```rust
// Current stub (github/mod.rs:140-152)
fn detect_bounty(_issue: &GitHubIssue) -> BountyInfo {
    BountyInfo {
        is_bounty: false,
        amount: None,
        provider: None,
        currency: None,
    }
}
```

### Original Design (Archived)

The provider system and detection logic were removed but documented here for future reference. When re-implemented as a plugin, consider:
- Provider config via `.patina/opportunity_providers.toml`
- Label-based detection (Algora, DoraHacks patterns)
- Body regex for amount extraction

---

## Future Phases

### Phase 2: PRs and Discussions

```sql
CREATE TABLE github_prs (
    number INTEGER PRIMARY KEY,
    title TEXT NOT NULL,
    body TEXT,
    state TEXT NOT NULL,        -- open, closed, merged
    labels TEXT,
    author TEXT,
    created_at TEXT NOT NULL,
    merged_at TEXT,
    url TEXT NOT NULL,
    base_ref TEXT,
    head_ref TEXT
);

CREATE TABLE github_discussions (
    number INTEGER PRIMARY KEY,
    title TEXT NOT NULL,
    body TEXT,
    category TEXT,              -- Q&A, Ideas, etc.
    author TEXT,
    created_at TEXT NOT NULL,
    url TEXT NOT NULL
);
```

### Phase 3: Semantic Search

- Generate E5 embeddings for issue title + body
- Store in embeddings table (same space as code)
- Enable semantic similarity search across code + issues

---

## Files

```
src/commands/scrape/github/
└── mod.rs              # Main scraper: fetch_issues(), insert_issues(), detect_bounty() stub
```

Note: The `opportunity/` subdirectory was removed on 2025-12-06 when bounty detection was extracted from core.

---

## Validation Criteria

**Phase 1 (Current) - Partial:**
- [x] Can scrape issues via `patina repo add --with-issues`
- [x] FTS5 search includes issues
- [ ] ~~Bounty detection works~~ (removed 2025-12-06, will return as plugin)
- [ ] ScryResult exposes bounty fields
- [ ] `repo update` includes github scrape

**Phase 2:**
- [ ] PRs and discussions scraped
- [ ] `--include-prs`, `--include-discussions` flags work

**Phase 3:**
- [ ] Semantic search works across code + issues
- [ ] Cross-repo discovery via `--all-repos`
