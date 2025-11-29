# Spec: GitHub Integration

**Status:** Design Phase
**Goal:** Index and query GitHub issues, PRs, and discussions for OnlyDust bounties and hackathon workflows

---

## The Problem

**Current state:** `patina repo <url>` clones and indexes **code only**.

**Missing capability:** GitHub metadata (issues, PRs, discussions) contains critical context:
- **Bounties** on OnlyDust are tagged via GitHub issues
- **Feature requests** live in discussions/issues
- **Contribution patterns** visible in PR history
- **Domain expertise** signals from issue labels and discussions

**Use case:**
```bash
# Today: Can only search code
patina scry "spawn entity patterns" --repo dojo

# Desired: Also search issues/PRs/discussions
patina scry "good first issue ECS" --repo dojo --include-issues
patina scry "bounty cairo testing" --repo dojo --include-all
```

---

## Design Principles

1. **Layered approach** - GitHub data is *enrichment*, not replacement of code search
2. **Explicit opt-in** - Code scraping is default, GitHub metadata is optional (via flags)
3. **Same query interface** - Issues/PRs/discussions queryable via `scry`, stored in eventlog
4. **Semantic + lexical** - Both FTS5 and vector search work on GitHub content
5. **Incremental updates** - Re-scraping only fetches new/updated items

---

## Architecture

### Event Types

Add new event types to the eventlog schema:

```sql
-- Core GitHub entities
CREATE TABLE github_issues (
    id INTEGER PRIMARY KEY,
    number INTEGER NOT NULL,
    title TEXT NOT NULL,
    body TEXT,
    state TEXT NOT NULL,        -- open, closed
    labels TEXT,                -- JSON array of label names
    author TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    closed_at TEXT,
    url TEXT NOT NULL,
    is_bounty BOOLEAN DEFAULT 0, -- Detect OnlyDust bounties
    bounty_amount TEXT           -- If bounty, the amount
);

CREATE TABLE github_prs (
    id INTEGER PRIMARY KEY,
    number INTEGER NOT NULL,
    title TEXT NOT NULL,
    body TEXT,
    state TEXT NOT NULL,        -- open, closed, merged
    labels TEXT,                -- JSON array
    author TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    merged_at TEXT,
    url TEXT NOT NULL,
    base_ref TEXT,              -- Target branch
    head_ref TEXT               -- Source branch
);

CREATE TABLE github_discussions (
    id INTEGER PRIMARY KEY,
    number INTEGER NOT NULL,
    title TEXT NOT NULL,
    body TEXT,
    category TEXT,              -- Q&A, Ideas, etc.
    labels TEXT,
    author TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    url TEXT NOT NULL
);

-- Comments (shared across issues, PRs, discussions)
CREATE TABLE github_comments (
    id INTEGER PRIMARY KEY,
    parent_type TEXT NOT NULL,  -- issue, pr, discussion
    parent_number INTEGER NOT NULL,
    author TEXT,
    body TEXT NOT NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);
```

### FTS5 Index

Extend the existing FTS5 table to include GitHub content:

```sql
-- Existing: code content
-- Add: GitHub issues/PRs/discussions
CREATE VIRTUAL TABLE IF NOT EXISTS fts_search USING fts5(
    content_type,    -- code, issue, pr, discussion, comment
    title,           -- For issues/PRs/discussions
    content,         -- Code or markdown body
    path,            -- File path or GitHub URL
    symbols,         -- Code symbols (existing)
    tokenize = 'porter unicode61'
);
```

### Vector Embeddings

GitHub content uses the **same semantic dimension** as code:
- Issue title + body → E5 embedding → Semantic MLP → 256-dim
- Stored in same `embeddings` table with `event_type = 'github.issue'`
- Queryable via `scry` with semantic search

---

## CLI Commands

### Scrape GitHub Data

```bash
# Scrape code only (default, current behavior)
patina repo add https://github.com/dojoengine/dojo

# Scrape code + issues
patina repo add https://github.com/dojoengine/dojo --with-issues

# Scrape code + issues + PRs + discussions
patina repo add https://github.com/dojoengine/dojo --with-all-github

# Update existing repo to include GitHub data
patina repo update dojo --with-issues
patina repo update dojo --with-all-github

# Scrape GitHub data only (no code rescrape)
patina repo scrape-github dojo
```

### Query GitHub Data

```bash
# Default: code only (current behavior)
patina scry "entity spawning" --repo dojo

# Include issues in results
patina scry "good first issue" --repo dojo --include-issues

# Include issues + PRs
patina scry "bounty cairo" --repo dojo --include-issues --include-prs

# Include all GitHub data
patina scry "ECS patterns" --repo dojo --include-all-github

# GitHub-only search
patina scry "bounty" --repo dojo --github-only
```

### Filter by Labels/State

```bash
# Find open bounties
patina scry "cairo testing" --repo dojo --issues-only --label bounty --state open

# Find merged PRs about ECS
patina scry "ECS refactor" --repo dojo --prs-only --state merged

# Find discussions in Q&A category
patina scry "how to spawn" --repo dojo --discussions-only --category QA
```

---

## Data Collection

### Using `gh` CLI

Leverage existing `gh` CLI (already used in `src/git/fork.rs`):

```bash
# Fetch issues as JSON
gh issue list --repo dojoengine/dojo \
  --limit 1000 \
  --state all \
  --json number,title,body,state,labels,author,createdAt,updatedAt,closedAt,url

# Fetch PRs as JSON
gh pr list --repo dojoengine/dojo \
  --limit 1000 \
  --state all \
  --json number,title,body,state,labels,author,createdAt,updatedAt,mergedAt,url,baseRefName,headRefName

# Fetch discussions via GraphQL
gh api graphql -f query='
  query($owner: String!, $repo: String!) {
    repository(owner: $owner, name: $repo) {
      discussions(first: 100) {
        nodes {
          number
          title
          body
          category { name }
          author { login }
          createdAt
          updatedAt
          url
        }
      }
    }
  }
' -f owner=dojoengine -f repo=dojo
```

### Bounty Detection

OnlyDust bounties are typically tagged with labels. Auto-detect:

```rust
fn detect_bounty(labels: &[String], body: &str) -> (bool, Option<String>) {
    let bounty_labels = ["bounty", "onlydust", "reward", "paid"];

    let is_bounty = labels.iter().any(|l| {
        bounty_labels.contains(&l.to_lowercase().as_str())
    });

    // Extract amount from body (common patterns)
    // "Bounty: 100 USDC", "Reward: $50", etc.
    let amount = extract_bounty_amount(body);

    (is_bounty, amount)
}
```

---

## Implementation Plan

### Phase 1: Issues Only (MVP)

**Goal:** Enable `patina scry --include-issues` for OnlyDust bounty discovery

- [ ] Add `github_issues` table to schema
- [ ] Implement `scrape_github_issues()` using `gh issue list --json`
- [ ] Store issues in eventlog with `event_type = "github.issue"`
- [ ] Add issues to FTS5 index (title + body)
- [ ] Add `--include-issues` flag to `scry` command
- [ ] Filter by labels, state in scry
- [ ] Auto-detect bounties (label + body parsing)

**Acceptance criteria:**
```bash
patina repo add dojoengine/dojo --with-issues
patina scry "bounty cairo" --repo dojo --include-issues --label bounty
# Returns: GitHub issues tagged as bounties with "cairo" in title/body
```

### Phase 2: PRs and Discussions

- [ ] Add `github_prs` and `github_discussions` tables
- [ ] Implement scraping via `gh pr list` and `gh api graphql`
- [ ] Extend scry with `--include-prs`, `--include-discussions`
- [ ] Add comments table for richer context

### Phase 3: Semantic Search

- [ ] Generate E5 embeddings for issue/PR/discussion bodies
- [ ] Store in `embeddings` table with `event_type = "github.issue"`, etc.
- [ ] Train semantic dimension on GitHub data (related issues = same session/label)
- [ ] Unified semantic search across code + GitHub data

### Phase 4: Cross-Project Bounty Discovery

- [ ] Query multiple repos for bounties: `patina scry "bounty" --all-repos`
- [ ] Aggregate bounties from all registered repos
- [ ] Filter by domains: `patina scry "bounty" --domain cairo`
- [ ] Persona-aware bounty matching (your skills → relevant bounties)

---

## Registry Schema Extension

Add GitHub metadata to `RepoEntry`:

```yaml
repos:
  dojo:
    path: ~/.patina/repos/dojo
    github: dojoengine/dojo
    contrib: true
    fork: nicabar/dojo
    registered: 2025-11-20T10:00:00Z
    domains: [cairo, starknet, ecs]
    github_data:                     # NEW
      issues: true                   # Has issue data
      prs: false                     # No PR data
      discussions: false             # No discussion data
      last_github_scrape: 2025-11-28T10:00:00Z
      issue_count: 347
      open_bounties: 12
```

---

## Example Workflows

### OnlyDust Contributor

```bash
# Add repos you're interested in with GitHub data
patina repo add dojoengine/dojo --with-issues --contrib
patina repo add starkware-libs/cairo --with-issues
patina repo add keep-starknet-strange/madara --with-issues

# Find bounties matching your skills (cairo)
patina scry "bounty" --all-repos --label bounty --state open --domain cairo

# Results:
# [ISSUE:dojo#234] Bounty: Implement storage optimization (500 USDC)
# [ISSUE:cairo#891] Bounty: Add felt252 serialization tests (200 USDC)
# [ISSUE:madara#45] Good first issue: Fix CLI help text (100 USDC)

# Dive into specific issue context
patina scry "storage optimization patterns" --repo dojo --include-issues

# Work on the bounty with full code context
cd ~/.patina/repos/dojo
/session-start "bounty-234-storage-optimization"
patina scry "storage cache" --file src/storage.rs
# ... implement fix ...
/session-end
git push fork patina
gh pr create --base main
```

### Hackathon Builder

```bash
# You're building a Starknet game, need to understand Dojo patterns
patina repo add dojoengine/dojo --with-all-github

# Search code AND discussions for patterns
patina scry "multiplayer sync" --repo dojo --include-discussions

# Results include:
# [CODE] src/multiplayer/sync.rs:45 - Sync entity state across clients
# [DISCUSSION:dojo#12] Best practices for multiplayer games
# [ISSUE:dojo#156] Sync lag in multiplayer mode

# Learn from both code and community wisdom
```

---

## Technical Considerations

### Rate Limiting

- GitHub API: 5000 req/hour (authenticated)
- `gh` CLI handles auth automatically
- Batch requests: `--limit 1000` per call
- Cache responses: only fetch updated items (use `since` parameter)

### Incremental Updates

Track last update timestamp per repo:

```rust
struct GithubScrapeState {
    last_issue_scrape: Option<DateTime>,
    last_pr_scrape: Option<DateTime>,
    last_discussion_scrape: Option<DateTime>,
}

// Only fetch issues updated since last scrape
gh issue list --state all --json ... \
  --search "updated:>=2025-11-27"
```

### Storage Size

Typical repo:
- **100 issues** × 5KB avg = 500KB
- **50 PRs** × 5KB avg = 250KB
- **20 discussions** × 5KB avg = 100KB
- **Total**: ~1MB per repo

With 10 repos: ~10MB of GitHub metadata (negligible vs code)

### Privacy

- Only public repos supported initially
- Private repos require PAT with appropriate scopes
- `gh` CLI respects existing auth

---

## Open Questions

1. **Comments depth?** Should we index all issue/PR comments, or just top-level?
   - **Proposal:** Top-level only for MVP, comments in Phase 2

2. **Embedding strategy?** Embed full issue body, or chunked?
   - **Proposal:** Full body for MVP (usually <2K tokens), chunk in Phase 2 if needed

3. **Label taxonomy?** OnlyDust uses specific labels, but other projects vary
   - **Proposal:** Flexible label detection, user can configure in `oxidize.yaml`

4. **Cross-project deduplication?** Same issue discussed in multiple repos
   - **Proposal:** Defer to Phase 4, not critical for MVP

---

## Success Metrics

**Phase 1 complete when:**
- Can find OnlyDust bounties via `scry --include-issues --label bounty`
- Lexical search works: `scry "good first issue"` matches issue titles
- Results show issue metadata (labels, state, URL)
- Can filter by labels and state

**Hackathon-ready when:**
- Can query Dojo issues while building Starknet game
- Semantic search works across code + issues
- Bounty discovery across multiple repos

---

## References

- [GitHub REST API: Issues](https://docs.github.com/en/rest/issues/issues)
- [GitHub REST API: Pull Requests](https://docs.github.com/en/rest/pulls/pulls)
- [GitHub GraphQL API: Discussions](https://docs.github.com/en/graphql/guides/using-the-graphql-api-for-discussions)
- [OnlyDust Bounty Program](https://www.onlydust.com/)
- [`gh` CLI Manual](https://cli.github.com/manual/)
