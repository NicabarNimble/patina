# GitHub Integration Prototype - Example Output

## What the prototype demonstrates

Running `./prototype-github-scrape.sh dojoengine/dojo 20` would fetch GitHub issues and demonstrate:

1. **Data fetching** via `gh issue list --json`
2. **Bounty detection** via label matching
3. **SQL schema** compatibility
4. **FTS5 indexing** readiness

---

## Example Output

```
🔍 Fetching issues from dojoengine/dojo (limit: 20)

📊 Found 20 issues

💰 Bounties detected:
  #234: Implement storage optimization for large worlds [open]
  #189: Add support for entity component caching [closed]
  #156: Optimize multiplayer sync performance [open]

🎯 Good first issues:
  #298: Fix typo in CLI help text [open]
  #276: Add example for simple entity spawning [closed]
  #245: Improve error messages in world builder [open]

📋 Sample issue JSON structure:
{
  "number": 234,
  "title": "Implement storage optimization for large worlds",
  "state": "open",
  "labels": ["bounty", "performance", "storage"],
  "author": "contributor123",
  "created": "2025-11-15T10:00:00Z",
  "updated": "2025-11-27T14:30:00Z",
  "url": "https://github.com/dojoengine/dojo/issues/234",
  "body_preview": "Currently, large worlds with 10k+ entities experience slowdowns during storage operations. We need to implement a caching layer that... [truncated]"
}

💾 Sample SQL inserts:
INSERT INTO github_issues (number, title, body, state, labels, author, created_at, updated_at, url, is_bounty)
VALUES (234, 'Implement storage optimization for large worlds', 'Currently, large worlds with 10k+ entities experience slowdowns...', 'open', '["bounty","performance","storage"]', 'contributor123', '2025-11-15T10:00:00Z', '2025-11-27T14:30:00Z', 'https://github.com/dojoengine/dojo/issues/234', true);

INSERT INTO github_issues (number, title, body, state, labels, author, created_at, updated_at, url, is_bounty)
VALUES (298, 'Fix typo in CLI help text', 'The help text for `dojo build` says "complie" instead of "compile"...', 'open', '["good first issue","documentation"]', 'newbie456', '2025-11-20T09:00:00Z', '2025-11-20T09:00:00Z', 'https://github.com/dojoengine/dojo/issues/298', false);

INSERT INTO github_issues (number, title, body, state, labels, author, created_at, updated_at, url, is_bounty)
VALUES (189, 'Add support for entity component caching', 'Bounty: 500 USDC\n\nWe need a caching mechanism for frequently accessed components...', 'closed', '["bounty","performance","completed"]', 'maintainer789', '2025-11-10T08:00:00Z', '2025-11-25T16:00:00Z', 'https://github.com/dojoengine/dojo/issues/189', true);

✅ Prototype complete

Next steps:
  1. Implement in Rust: src/commands/scrape/github.rs
  2. Add GitHub tables to schema.sql
  3. Extend scry command with --include-issues flag
  4. Add FTS5 indexing for issue title + body
```

---

## Key Insights from Prototype

### 1. Data Structure

GitHub issues map cleanly to SQL schema:
- **Structured fields**: number, title, state, author, timestamps
- **Rich text**: body (markdown, embedable)
- **Metadata**: labels (array), URL
- **Custom**: is_bounty (computed), bounty_amount (extracted)

### 2. Bounty Detection

Simple label-based detection works:
```javascript
labels.includes("bounty") ||
labels.includes("onlydust") ||
labels.includes("reward")
```

OnlyDust repos consistently use "bounty" label. Could enhance with:
- Body text parsing for "Bounty: $XXX" or "Reward: XXX USDC"
- OnlyDust API integration (if available)

### 3. FTS5 Integration

Issue data ready for full-text search:
```sql
INSERT INTO fts_search (content_type, title, content, path)
VALUES (
  'issue',
  'Implement storage optimization for large worlds',
  'Currently, large worlds with 10k+ entities...',
  'https://github.com/dojoengine/dojo/issues/234'
);
```

Enables queries like:
```bash
patina scry "storage optimization"
# Returns: Both code files AND GitHub issues mentioning "storage optimization"
```

### 4. Performance

- **Typical repo**: 100-500 open issues
- **Fetch time**: ~2-3 seconds for 100 issues (with `--limit 100`)
- **Storage**: ~5KB per issue = 500KB for 100 issues
- **Rate limits**: 5000 req/hour (sufficient for 50+ repos)

### 5. Incremental Updates

GitHub API supports filtering by update time:
```bash
gh issue list --search "updated:>=2025-11-27"
```

Only fetch issues modified since last scrape. Typical repo:
- **Initial scrape**: 200 issues = 10 seconds
- **Daily update**: 5-10 issues = 1 second
- **Weekly update**: 20-30 issues = 2 seconds

---

## Integration with Existing Patina

### Scrape Flow

```
patina repo add dojoengine/dojo --with-issues
  │
  ├─> Clone repo to ~/.patina/repos/dojo/
  ├─> Create .patina/ scaffolding
  ├─> Scrape code → patina.db (existing)
  └─> Scrape issues → patina.db (NEW)
        │
        ├─> gh issue list --json
        ├─> Parse & detect bounties
        ├─> INSERT INTO github_issues
        └─> INSERT INTO fts_search
```

### Query Flow

```
patina scry "storage cache" --repo dojo --include-issues
  │
  ├─> FTS5 search: "storage cache"
  │     ├─> Code files matching
  │     └─> Issues matching (NEW)
  │
  ├─> Semantic search (if oxidized)
  │     ├─> Code embeddings
  │     └─> Issue embeddings (NEW)
  │
  └─> Combined results:
        [CODE] src/storage/cache.rs:45
        [ISSUE:dojo#234] Implement storage caching
        [CODE] src/world/storage.rs:123
        [ISSUE:dojo#189] Entity component caching
```

---

## Rust Implementation Sketch

```rust
// src/commands/scrape/github.rs

pub struct GithubIssue {
    pub number: i64,
    pub title: String,
    pub body: Option<String>,
    pub state: String,
    pub labels: Vec<String>,
    pub author: String,
    pub created_at: String,
    pub updated_at: String,
    pub url: String,
}

impl GithubIssue {
    /// Detect if issue is a bounty based on labels and body
    pub fn is_bounty(&self) -> bool {
        let bounty_labels = ["bounty", "onlydust", "reward"];
        self.labels.iter()
            .any(|l| bounty_labels.contains(&l.to_lowercase().as_str()))
    }

    /// Extract bounty amount from body if present
    pub fn bounty_amount(&self) -> Option<String> {
        // Regex patterns for common bounty formats
        // "Bounty: 500 USDC", "Reward: $100", etc.
        extract_bounty_from_text(self.body.as_ref()?)
    }
}

/// Fetch issues for a GitHub repository
pub fn scrape_github_issues(repo: &str, since: Option<&str>) -> Result<Vec<GithubIssue>> {
    let mut args = vec![
        "issue", "list",
        "--repo", repo,
        "--limit", "1000",
        "--state", "all",
        "--json", "number,title,body,state,labels,author,createdAt,updatedAt,url"
    ];

    // Incremental update: only fetch issues updated since last scrape
    if let Some(since) = since {
        args.push("--search");
        args.push(&format!("updated:>={}", since));
    }

    let output = Command::new("gh")
        .args(&args)
        .output()?;

    if !output.status.success() {
        bail!("Failed to fetch issues: {}", String::from_utf8_lossy(&output.stderr));
    }

    let issues: Vec<GithubIssue> = serde_json::from_slice(&output.stdout)?;
    Ok(issues)
}

/// Insert issues into database
pub fn insert_github_issues(conn: &Connection, issues: &[GithubIssue]) -> Result<()> {
    let tx = conn.transaction()?;

    for issue in issues {
        // Insert into github_issues table
        tx.execute(
            "INSERT OR REPLACE INTO github_issues
             (number, title, body, state, labels, author, created_at, updated_at, url, is_bounty, bounty_amount)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
            params![
                issue.number,
                &issue.title,
                &issue.body,
                &issue.state,
                serde_json::to_string(&issue.labels)?,
                &issue.author,
                &issue.created_at,
                &issue.updated_at,
                &issue.url,
                issue.is_bounty(),
                issue.bounty_amount(),
            ],
        )?;

        // Insert into FTS5 for lexical search
        tx.execute(
            "INSERT INTO fts_search (content_type, title, content, path)
             VALUES ('issue', ?1, ?2, ?3)",
            params![
                &issue.title,
                issue.body.as_deref().unwrap_or(""),
                &issue.url,
            ],
        )?;
    }

    tx.commit()?;
    Ok(())
}
```

---

## Next Steps

1. **Validate approach**: Run prototype on real repos (requires `gh auth login`)
2. **Schema design**: Add GitHub tables to `src/schema.sql`
3. **Rust implementation**: Create `src/commands/scrape/github.rs`
4. **CLI integration**: Add `--with-issues` flag to `repo add`
5. **Query extension**: Add `--include-issues` to `scry`
6. **Testing**: Verify with OnlyDust repos (dojo, cairo, madara)

---

## Authentication Note

To run the prototype:
```bash
# Authenticate gh CLI (one-time setup)
gh auth login

# Run prototype
./prototype-github-scrape.sh dojoengine/dojo 20
```

The `gh` CLI respects existing GitHub authentication and handles rate limiting automatically.
