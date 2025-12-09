# Spec: GitHub Adapter

**Status:** Phase 1 Complete, Phase 2+ Pending
**Location:** `src/commands/scrape/github/`

---

## Purpose

Index GitHub metadata (issues, PRs, discussions) for context enrichment. Currently supports issues with bounty detection.

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
             │  --label bounty                 │
             └─────────────────────────────────┘
```

**Key insight:** Issues and code share the same semantic space via E5 embeddings.

---

## Current Implementation (Phase 1)

### What Works

| Feature | Status | Location |
|---------|--------|----------|
| `gh issue list` fetching | ✅ | `github/mod.rs:fetch_issues()` |
| Bounty detection (labels) | ✅ | `github/opportunity/detector.rs` |
| Bounty detection (body regex) | ✅ | `github/opportunity/detector.rs` |
| Provider system (Algora, DoraHacks, etc.) | ✅ | `github/opportunity/provider.rs` |
| `github_issues` table | ✅ | `github/mod.rs:create_materialized_views()` |
| FTS5 indexing | ✅ | `github/mod.rs:populate_fts5_github()` |
| Incremental updates | ✅ | `github/mod.rs:get_last_scrape()` |
| `--with-issues` flag on `repo add` | ✅ | `src/commands/repo/` |

### What's Missing (Deferred)

| Feature | Status | Issue |
|---------|--------|-------|
| Expose bounty fields in ScryResult | ❌ | Bounty data in DB but not surfaced |
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

## Bounty Detection

### Provider System

Configured via `.patina/opportunity_providers.toml`:

```toml
[[providers]]
name = "algora"
labels = ["💎 Bounty", "bounty", "Bounty"]
body_patterns = ['Bounty:\s*\$?(\d+)', 'reward[:\s]+\$?(\d+)']
default_currency = "USD"

[[providers]]
name = "dorahacks"
labels = ["hackathon", "DoraHacks"]
body_patterns = ['prize[:\s]+\$?(\d+)']
default_currency = "USD"
```

### Detection Logic

```rust
pub fn detect_opportunity(issue: &GitHubIssue, providers: &[Provider]) -> OpportunityInfo {
    for provider in providers {
        // 1. Check labels
        if issue.labels.iter().any(|l| provider.labels.contains(&l.name)) {
            return match_found(provider, extract_amount(issue, provider));
        }

        // 2. Check body patterns
        if let Some(body) = &issue.body {
            for pattern in &provider.body_patterns {
                if let Some(amount) = pattern.captures(body) {
                    return match_found(provider, Some(amount));
                }
            }
        }
    }
    OpportunityInfo::none()
}
```

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
├── mod.rs              # Main scraper, fetch_issues(), insert_issues()
└── opportunity/
    ├── mod.rs          # Public interface
    ├── provider.rs     # Provider struct, load_providers()
    └── detector.rs     # detect_opportunity()
```

---

## Validation Criteria

**Phase 1 (Current) complete when:**
- [x] Can scrape issues via `patina repo add --with-issues`
- [x] Bounty detection works (labels + body patterns)
- [x] FTS5 search includes issues
- [ ] ScryResult exposes bounty fields ← **blocking**
- [ ] `repo update` includes github scrape

**Phase 2 complete when:**
- [ ] PRs and discussions scraped
- [ ] `--include-prs`, `--include-discussions` flags work

**Phase 3 complete when:**
- [ ] Semantic search works across code + issues
- [ ] Cross-repo bounty discovery via `--all-repos`
