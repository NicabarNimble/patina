# Spec: Forge Abstraction

**Status:** Phase 4 Complete, Phase 5 Ready
**Created:** 2026-01-09
**Revised:** 2026-01-10
**Origin:** Sessions 20260109-170426, 20260110-061843, 20260110-101440

---

## Problem

GitHub-specific code is scattered across the codebase with no abstraction:

```
src/commands/scrape/github/mod.rs  → gh issue list
src/commands/repo/internal.rs      → gh repo fork, gh repo view
src/git/fork.rs                    → gh api user, gh repo create/clone
```

This creates:
1. **Tight coupling** to GitHub platform
2. **No path** to Gitea, Codeberg support
3. **Scattered** gh CLI calls with inconsistent error handling
4. **Mixed concerns** - scraping knowledge vs. repository operations

---

## Design Principles

This spec applies three core patterns:

| Pattern | Application |
|---------|-------------|
| **unix-philosophy** | Split read (scraping) from write (repo ops) - separate tools |
| **dependable-rust** | Each provider is a black-box with internal implementation hidden |
| **adapter-pattern** | Trait-based abstraction, 3-5 methods per trait, domain types only |

---

## The Boundary

```
┌─────────────────────────────────────────────────────────────┐
│                         GIT                                  │
│                    (universal, local)                        │
│                                                              │
│  commits, branches, tags, remotes, co-changes               │
│  + conventional commit parsing (feat/fix/scope/pr_ref)      │
│                                                              │
│  Tool: git CLI + regex                                      │
│  Location: src/commands/scrape/git/                         │
└─────────────────────────────────────────────────────────────┘
                              │
                              │ pr_refs discovered → fetch details
                              ▼
┌─────────────────────────────────────────────────────────────┐
│                     FORGE (Read)                             │
│                 "Fetch code review data"                     │
│                                                              │
│  issues, PRs, reviews, comments, labels                     │
│  Read-only, network, cacheable                              │
│                                                              │
│  Trait: ForgeReader                                         │
│  Location: src/forge/                                       │
└─────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────┐
│                    FORGE (Write)                             │
│              "Perform repository operations"                 │
│                                                              │
│  fork, create repo, current user                            │
│  Write operations, auth required                            │
│                                                              │
│  Trait: ForgeWriter                                         │
│  Location: src/repo/ (not src/forge/)                       │
└─────────────────────────────────────────────────────────────┘
```

**Rule**: If it exists in `.git/`, it's git. If it reads from API, it's ForgeReader. If it writes to API, it's ForgeWriter.

---

## CLI-First Approach

We shell out to `gh` CLI rather than making direct REST/GraphQL API calls.

**Why CLI over API:**

| Aspect | CLI (`gh`) | Direct API |
|--------|------------|------------|
| Auth | `gh auth login` handles OAuth, tokens, SSH | Manage tokens, refresh, storage |
| Rate limits | `gh` shows warnings, may auto-wait | Parse headers, implement backoff |
| Pagination | `--limit 500` just works | Follow `Link` headers manually |
| JSON output | `--json` gives exactly what you ask | Full objects, filter client-side |
| Cross-platform | Same everywhere | Different TLS, proxy handling |
| Offline testing | Mock command output | Mock HTTP layer |

**Trade-offs accepted:**
- Requires `gh` installed (already checked in `patina init`)
- Subprocess overhead (negligible for our use case)
- Less control over exact API behavior

**Escape hatch:** If we ever need direct API (rate limit control, GraphQL batching), only `internal.rs` changes - trait interface stays stable.

---

## Current Codebase Mapping

Existing `gh` CLI calls and their future home:

| Current Location | `gh` Command | Future Trait | Method |
|-----------------|--------------|--------------|--------|
| `src/commands/scrape/github/mod.rs:95` | `gh auth status` | `ForgeWriter` | `is_authenticated()` |
| `src/commands/scrape/github/mod.rs:105` | `gh issue list` | `ForgeReader` | `list_issues()` |
| `src/git/fork.rs:27` | `gh api user` | `ForgeWriter` | `current_user()` |
| `src/git/fork.rs:44` | `gh repo view` | `ForgeWriter` | (internal helper) |
| `src/git/fork.rs:54` | `gh repo fork` | `ForgeWriter` | `fork()` |
| `src/git/fork.rs:162` | `gh repo create` | `ForgeWriter` | `create_repo()` |
| `src/git/fork.rs:230,282` | `gh repo clone` | `ForgeWriter` | (internal helper) |
| `src/commands/repo/internal.rs:728` | `gh repo fork` | `ForgeWriter` | `fork()` |
| `src/commands/repo/internal.rs:742` | `gh repo view` | `ForgeWriter` | (internal helper) |

**New capability (not yet implemented):**

| Future Location | `gh` Command | Trait | Method |
|-----------------|--------------|-------|--------|
| `src/forge/github/internal.rs` | `gh pr list` | `ForgeReader` | `list_pull_requests()` |
| `src/forge/github/internal.rs` | `gh pr view --json ...` | `ForgeReader` | `get_pull_request()` |

---

## Consumer Commands

Which CLI commands will use ForgeReader/ForgeWriter:

```
patina repo add <url> --with-issues
    │
    ├── ForgeReader.list_issues()      ←── scrape issues into eventlog
    │       currently: scrape_github_issues() in repo/internal.rs:676
    │
    └── ForgeWriter.fork()             ←── create fork if --contrib
            currently: create_fork() in repo/internal.rs:725

patina scrape forge                    ←── NEW COMMAND
    │
    ├── collect pr_refs from git.commit events
    │
    └── ForgeReader.get_pull_request() ←── fetch PR context for each ref

patina init . --fork
    │
    └── ForgeWriter.fork()             ←── create GitHub fork
        ForgeWriter.create_repo()      ←── or create new repo
            currently: fork.rs functions
```

---

## Trait Design

### ForgeReader - "Fetch code review data from forge"

```rust
// src/forge/mod.rs

/// Read-only access to forge data (issues, PRs, reviews)
///
/// "Do X": Fetch code review data from a forge platform
pub trait ForgeReader {
    /// Fetch issues (with optional since filter for incremental updates)
    fn list_issues(&self, limit: usize, since: Option<&str>) -> Result<Vec<Issue>>;

    /// Fetch pull requests
    fn list_pull_requests(&self, limit: usize, since: Option<&str>) -> Result<Vec<PullRequest>>;

    /// Get single PR with full details (body, comments, reviews, linked issues)
    fn get_pull_request(&self, number: i64) -> Result<PullRequest>;
}
```

3 methods. Focused. Read-only. Cacheable.

### ForgeWriter - "Perform repository operations on forge"

```rust
// src/repo/forge.rs (NOT in src/forge/ - different concern)

/// Write operations on forge (fork, create, auth)
///
/// "Do X": Create and fork repositories on a forge platform
pub trait ForgeWriter {
    /// Check if authenticated to this forge
    fn is_authenticated(&self) -> Result<bool>;

    /// Get current authenticated user
    fn current_user(&self) -> Result<String>;

    /// Fork a repository, returns fork URL
    fn fork(&self) -> Result<String>;

    /// Create a new repository
    fn create_repo(&self, name: &str, private: bool) -> Result<String>;
}
```

4 methods. Write operations. Auth-dependent. Separate module.

---

## Why Two Traits?

**Different consumers:**
```rust
// Scraping only needs ForgeReader
fn scrape_forge(reader: &dyn ForgeReader, pr_refs: &[i64]) -> Result<()> {
    for pr_num in pr_refs {
        let pr = reader.get_pull_request(*pr_num)?;
        // store in eventlog
    }
}

// Repo operations need ForgeWriter
fn fork_repo(writer: &dyn ForgeWriter) -> Result<String> {
    if !writer.is_authenticated()? {
        return Err(Error::NotAuthenticated);
    }
    writer.fork()
}
```

**Different error handling:**
- ForgeReader: graceful degradation (no data = empty vec)
- ForgeWriter: must fail loudly (auth errors, permission denied)

**Different caching:**
- ForgeReader: aggressive caching, incremental updates
- ForgeWriter: no caching, always live

---

## Target Structure

```
src/
├── forge/                      # "Fetch code review data"
│   ├── mod.rs                  # ForgeReader trait, detect_forge(), types re-export
│   ├── types.rs                # Issue, PullRequest, Comment (domain types)
│   ├── github/
│   │   ├── mod.rs              # GitHubReader: impl ForgeReader
│   │   └── internal.rs         # gh CLI calls, JSON parsing
│   ├── gitea/
│   │   ├── mod.rs              # GiteaReader: impl ForgeReader
│   │   └── internal.rs         # tea CLI or API calls
│   └── none.rs                 # NoneReader: returns empty (simple, no internal)
│
├── repo/                       # "Perform repository operations"
│   ├── mod.rs                  # Existing repo command
│   ├── forge.rs                # ForgeWriter trait
│   ├── github.rs               # GitHubWriter: impl ForgeWriter
│   └── internal.rs             # Existing internals
│
└── commands/scrape/
    ├── git/                    # Unchanged + conventional commit parsing
    │   ├── mod.rs
    │   └── commits.rs          # Add: parse_conventional_commit()
    └── forge/                  # Replaces github/
        └── mod.rs              # Uses ForgeReader trait
```

---

## Core Types

```rust
// src/forge/types.rs

/// Detected forge information
pub struct Forge {
    pub kind: ForgeKind,
    pub owner: String,
    pub repo: String,
    pub host: String,  // "github.com", "codeberg.org", etc.
}

pub enum ForgeKind {
    GitHub,
    Gitea,   // Covers Gitea, Codeberg, Forgejo
    None,    // Local-only repo, no forge
}

/// Issue from any forge
pub struct Issue {
    pub number: i64,
    pub title: String,
    pub body: Option<String>,
    pub state: IssueState,
    pub author: String,
    pub labels: Vec<String>,
    pub created_at: String,
    pub updated_at: String,
    pub url: String,
}

/// Pull/Merge Request from any forge
pub struct PullRequest {
    pub number: i64,
    pub title: String,
    pub body: Option<String>,
    pub state: PrState,
    pub author: String,
    pub labels: Vec<String>,
    pub created_at: String,
    pub merged_at: Option<String>,
    pub url: String,
    // The valuable context
    pub linked_issues: Vec<i64>,
    pub comments: Vec<Comment>,
    pub approvals: i32,
}

pub struct Comment {
    pub author: String,
    pub body: String,
    pub created_at: String,
}

pub enum IssueState { Open, Closed }
pub enum PrState { Open, Merged, Closed }
```

---

## Forge Detection

```rust
// src/forge/mod.rs

mod types;
mod github;
mod gitea;
mod none;

pub use types::*;

/// Detect forge from git remote URL
pub fn detect(remote_url: &str) -> Forge {
    // Parse: git@github.com:owner/repo.git
    // Parse: https://github.com/owner/repo
    // Parse: https://codeberg.org/owner/repo

    if remote_url.contains("github.com") {
        Forge { kind: ForgeKind::GitHub, /* parse owner/repo */ }
    } else if is_gitea_host(remote_url) {
        Forge { kind: ForgeKind::Gitea, /* parse owner/repo */ }
    } else {
        Forge { kind: ForgeKind::None, /* empty */ }
    }
}

/// Get reader for detected forge
pub fn reader(forge: &Forge) -> Box<dyn ForgeReader> {
    match forge.kind {
        ForgeKind::GitHub => Box::new(github::GitHubReader::new(forge)),
        ForgeKind::Gitea => Box::new(gitea::GiteaReader::new(forge)),
        ForgeKind::None => Box::new(none::NoneReader),
    }
}

fn is_gitea_host(url: &str) -> bool {
    url.contains("codeberg.org")
        || url.contains("gitea.")
        || url.contains("forgejo.")
    // Could also probe API endpoint
}
```

---

## GitHub Reader Implementation

```rust
// src/forge/github/mod.rs

mod internal;

use crate::forge::{ForgeReader, Forge, Issue, PullRequest};

pub struct GitHubReader {
    owner: String,
    repo: String,
}

impl GitHubReader {
    pub fn new(forge: &Forge) -> Self {
        Self {
            owner: forge.owner.clone(),
            repo: forge.repo.clone(),
        }
    }

    fn repo_spec(&self) -> String {
        format!("{}/{}", self.owner, self.repo)
    }
}

impl ForgeReader for GitHubReader {
    fn list_issues(&self, limit: usize, since: Option<&str>) -> Result<Vec<Issue>> {
        internal::fetch_issues(&self.repo_spec(), limit, since)
    }

    fn list_pull_requests(&self, limit: usize, since: Option<&str>) -> Result<Vec<PullRequest>> {
        internal::fetch_pull_requests(&self.repo_spec(), limit, since)
    }

    fn get_pull_request(&self, number: i64) -> Result<PullRequest> {
        internal::fetch_pull_request(&self.repo_spec(), number)
    }
}
```

```rust
// src/forge/github/internal.rs

use std::process::Command;
use crate::forge::types::*;

/// Fetch issues via gh CLI
pub(crate) fn fetch_issues(repo: &str, limit: usize, since: Option<&str>) -> Result<Vec<Issue>> {
    let mut args = vec![
        "issue", "list",
        "--repo", repo,
        "--limit", &limit.to_string(),
        "--json", "number,title,body,state,author,labels,createdAt,updatedAt,url",
    ];

    if let Some(date) = since {
        // gh doesn't have --since, would need to filter post-fetch
        // or use gh api with search query
    }

    let output = Command::new("gh").args(&args).output()?;
    parse_issues(&output.stdout)
}

/// Fetch single PR with full context
pub(crate) fn fetch_pull_request(repo: &str, number: i64) -> Result<PullRequest> {
    let output = Command::new("gh")
        .args([
            "pr", "view", &number.to_string(),
            "--repo", repo,
            "--json", "number,title,body,state,author,labels,createdAt,mergedAt,url,comments,reviews,closingIssuesReferences"
        ])
        .output()?;

    parse_pull_request(&output.stdout)
}

fn parse_issues(json: &[u8]) -> Result<Vec<Issue>> {
    // serde_json parsing, map gh format to our types
}

fn parse_pull_request(json: &[u8]) -> Result<PullRequest> {
    // serde_json parsing
    // Extract linked_issues from closingIssuesReferences
    // Extract comments from comments + reviews
}
```

---

## None Reader Implementation

```rust
// src/forge/none.rs

use crate::forge::{ForgeReader, Issue, PullRequest};

/// Null implementation for repos without a forge
pub struct NoneReader;

impl ForgeReader for NoneReader {
    fn list_issues(&self, _limit: usize, _since: Option<&str>) -> Result<Vec<Issue>> {
        Ok(vec![])  // No forge = no issues
    }

    fn list_pull_requests(&self, _limit: usize, _since: Option<&str>) -> Result<Vec<PullRequest>> {
        Ok(vec![])  // No forge = no PRs
    }

    fn get_pull_request(&self, _number: i64) -> Result<PullRequest> {
        Err(Error::NoForge)  // Can't fetch what doesn't exist
    }
}
```

Simple enough - no `internal.rs` needed.

---

## Integration: Scrape Flow

```
patina scrape
    │
    ├── scrape git ──────────────────────────────────────────┐
    │       │                                                 │
    │       ├── fetch commits                                │
    │       │       └── parse conventional commits           │
    │       │               └── extract: type, scope, pr_ref │
    │       │                                                │
    │       └── store git.commit events                      │
    │               (with parsed fields)                     │
    │                                                        │
    └── scrape forge ────────────────────────────────────────┤
            │                                                 │
            ├── collect pr_refs from git.commit events       │
            │                                                │
            ├── detect forge from origin remote              │
            │       └── get ForgeReader                      │
            │                                                │
            ├── for each pr_ref:                             │
            │       └── reader.get_pull_request(pr_ref)      │
            │               └── store forge.pr event         │
            │                                                │
            └── optionally: reader.list_issues()             │
                    └── store forge.issue events             │
```

**Key insight**: Conventional commit parsing happens in git scrape (local, fast). Forge scrape only fetches what we have references for (targeted, efficient).

---

## Conventional Commit Parsing

Separate from forge, but related. Add to git scrape:

```rust
// src/commands/scrape/git/commits.rs

use regex::Regex;
use lazy_static::lazy_static;

lazy_static! {
    // feat(sozo): add invoke command (#3384)
    static ref CONVENTIONAL: Regex = Regex::new(
        r"^(?P<type>\w+)(?:\((?P<scope>[^)]+)\))?(?P<breaking>!)?: (?P<desc>.+?)(?:\s*\(#(?P<pr>\d+)\))?$"
    ).unwrap();

    // Fixes #123, Closes #456
    static ref ISSUE_REF: Regex = Regex::new(
        r"(?i)(?:fix(?:es)?|close[sd]?|resolve[sd]?)[:\s]+#?(\d+)"
    ).unwrap();
}

pub struct ParsedCommit {
    pub commit_type: Option<String>,   // "feat", "fix", "docs"
    pub scope: Option<String>,          // "sozo", "cli"
    pub breaking: bool,                 // ! marker
    pub pr_ref: Option<i64>,            // 3384
    pub issue_refs: Vec<i64>,           // [123, 456]
}

pub fn parse_conventional(message: &str) -> ParsedCommit {
    let first_line = message.lines().next().unwrap_or("");

    let (commit_type, scope, breaking, pr_ref) = CONVENTIONAL
        .captures(first_line)
        .map(|c| (
            c.name("type").map(|m| m.as_str().to_string()),
            c.name("scope").map(|m| m.as_str().to_string()),
            c.name("breaking").is_some(),
            c.name("pr").and_then(|m| m.as_str().parse().ok()),
        ))
        .unwrap_or((None, None, false, None));

    let issue_refs: Vec<i64> = ISSUE_REF
        .captures_iter(message)
        .filter_map(|c| c.get(1)?.as_str().parse().ok())
        .collect();

    ParsedCommit { commit_type, scope, breaking, pr_ref, issue_refs }
}
```

Stored in `git.commit` event:
```json
{
  "event_type": "git.commit",
  "data": {
    "sha": "abc123",
    "message": "feat(sozo): add invoke command (#3384)",
    "author": "...",
    "parsed": {
      "type": "feat",
      "scope": "sozo",
      "pr_ref": 3384,
      "issue_refs": [],
      "breaking": false
    }
  }
}
```

---

## Event Types

| Event Type | Source | Contains |
|------------|--------|----------|
| `git.commit` | git scrape | sha, message, author, files, **parsed** (type, scope, pr_ref) |
| `forge.issue` | forge scrape | number, title, body, labels, state |
| `forge.pr` | forge scrape | number, title, body, linked_issues, comments |

---

## Migration Plan

### Phase 1: Conventional Commit Parsing - DONE
1. Add `parse_conventional()` to `scrape/git/` ✓
2. Enrich `git.commit` events with parsed fields ✓
3. No new dependencies, no network calls ✓

### Phase 2: ForgeReader Module - DONE
1. Create `src/forge/` with trait and types ✓
2. Move issue fetching from `scrape/github/` to `forge/github/` ✓
3. Add `get_pull_request()` method ✓
4. `scrape/github/` → `scrape/forge/` using trait ✓

Session 20260110-101440: 2 commits (feat + refactor), 600+ lines

### Phase 3: PR Context Fetching - DONE
1. After git scrape, collect pr_refs ✓
2. Fetch PR details for each ref ✓
3. Store as `forge.pr` events ✓

Session 20260110-101440: Added `patina scrape forge` CLI command, 190 lines

### Phase 4: ForgeWriter - DONE
1. Create `src/forge/writer.rs` with ForgeWriter trait (5 methods) ✓
2. Implement GitHubWriter using gh CLI ✓
3. Migrate `git/fork.rs` to use ForgeWriter ✓
4. Migrate `repo/internal.rs` to use ForgeWriter ✓

Session 20260110-143305: 2 commits (feat + refactor), removed ~73 lines of duplication

### Phase 5: Gitea Support
1. Implement `GiteaReader` using tea CLI or API
2. Auto-detect Codeberg, Forgejo instances

---

## Measurement Strategy

Forge data is only valuable if it improves retrieval. We measure before committing to network fetches.

### Phase 1 Validation: PR Ref Density

Before implementing Phase 3 (PR fetching), measure what we have:

```bash
# How many commits have PR refs? (after conventional commit parsing)
sqlite3 .patina/data/patina.db "
  SELECT
    COUNT(*) as total_commits,
    SUM(CASE WHEN json_extract(data, '$.parsed.pr_ref') IS NOT NULL THEN 1 ELSE 0 END) as with_pr_ref,
    ROUND(100.0 * SUM(CASE WHEN json_extract(data, '$.parsed.pr_ref') IS NOT NULL THEN 1 ELSE 0 END) / COUNT(*), 1) as pct
  FROM eventlog
  WHERE event_type = 'git.commit'
"
```

**Gate**: If <20% of commits have pr_refs, PR fetching has limited value for that repo.

### Phase 3 Validation: PR Body Quality

Sample before full scrape:

```bash
# Fetch 10 random PRs, check body length
for pr in $(shuf -n 10 pr_refs.txt); do
  gh pr view $pr --repo owner/repo --json body --jq '.body | length'
done | awk '{sum+=$1; count++} END {print "Avg PR body length:", sum/count}'
```

**Gate**: If avg body length <100 chars, PR bodies don't add much over commit messages.

### A/B Benchmark: Does Forge Data Help?

Use existing `patina bench` infrastructure:

```bash
# Generate querysets
patina bench generate --repo dojo -o dojo-commits.json      # commit messages
patina bench generate --from-prs --repo dojo -o dojo-prs.json  # PR bodies (new)

# Compare retrieval quality
patina bench retrieval -q dojo-commits.json --json > baseline.json
patina bench retrieval -q dojo-prs.json --json > with-forge.json

# Compare MRR and Recall@10
jq -s '.[0].mrr, .[1].mrr, .[1].mrr - .[0].mrr' baseline.json with-forge.json
```

**Success**: MRR improvement >0.05 or Recall@10 improvement >5% justifies forge fetching.

### Feedback Loop Validation

After forge data is live, measure real-world impact:

```bash
patina eval --feedback
```

Compare precision before/after forge data was added. If `forge.pr` results have higher hit rate than `git.commit` results, forge data is working.

---

## Graceful Degradation

Patina works without forge access. Git is the foundation, forge is enrichment.

### Dependency Matrix

| Command | Requires `git` | Requires `gh` | Without `gh` |
|---------|----------------|---------------|--------------|
| `patina repo add <url>` | ✅ | ❌ | Works (git clone) |
| `patina repo add <url> --with-issues` | ✅ | ✅ | Warns, continues without issues |
| `patina repo add <url> --contrib` | ✅ | ✅ | Fails (can't fork) |
| `patina repo update <name>` | ✅ | ❌ | Works (git pull) |
| `patina scrape` | ✅ | ❌ | Works (all local scrapers) |
| `patina scrape forge` | ✅ | ✅ | Warns, skips forge data |
| `patina scry` | ❌ | ❌ | Works |
| `patina bench` | ❌ | ❌ | Works |

### Forge Detection Behavior

```rust
// In scrape forge command
let forge = forge::detect(&remote_url);
let reader = forge::reader(&forge);

match forge.kind {
    ForgeKind::GitHub => {
        if !reader.is_available() {
            println!("⚠️  GitHub detected but `gh` not authenticated. Skipping forge data.");
            println!("   Run `gh auth login` to enable issue/PR fetching.");
            return Ok(());  // Not an error - graceful skip
        }
    }
    ForgeKind::Gitea => {
        println!("ℹ️  Gitea detected. Forge scraping not yet implemented.");
        return Ok(());
    }
    ForgeKind::None => {
        // Silent - no forge is normal for local repos
        return Ok(());
    }
}
```

### Data Availability by Forge Status

| Forge Status | Git Data | Forge Data | User Experience |
|--------------|----------|------------|-----------------|
| GitHub + `gh` auth | ✅ commits, co-changes | ✅ issues, PRs | Full knowledge |
| GitHub, no `gh` | ✅ commits, co-changes | ❌ | Warn once, continue |
| GitLab/Codeberg | ✅ commits, co-changes | ❌* | Info message |
| Self-hosted git | ✅ commits, co-changes | ❌ | Silent (expected) |
| Local repo (no remote) | ✅ commits, co-changes | ❌ | Silent (expected) |

*Until GiteaReader/GitLabReader implemented

### NoneReader Implementation

The null implementation ensures no crashes:

```rust
impl ForgeReader for NoneReader {
    fn list_issues(&self, _: usize, _: Option<&str>) -> Result<Vec<Issue>> {
        Ok(vec![])  // Empty, not error
    }

    fn list_pull_requests(&self, _: usize, _: Option<&str>) -> Result<Vec<PullRequest>> {
        Ok(vec![])  // Empty, not error
    }

    fn get_pull_request(&self, number: i64) -> Result<PullRequest> {
        // This is the only method that errors - caller asked for specific PR
        Err(ForgeError::NoForge {
            message: format!("No forge configured, cannot fetch PR #{}", number)
        })
    }
}
```

### Error Handling Philosophy

| Error Type | Behavior | Rationale |
|------------|----------|-----------|
| No forge detected | Silent return | Normal for many repos |
| `gh` not installed | Warn once, skip | User can install later |
| `gh` not authenticated | Warn with fix command | Actionable |
| Rate limited | Warn, partial results | Better than nothing |
| Network error | Retry 3x, then warn | Transient failures |
| PR not found (deleted) | Skip that PR, continue | Don't block on one failure |

---

## Open Questions (Resolved)

| Question | Resolution |
|----------|------------|
| CLI vs API? | CLI first (gh/tea), auth handled, works now |
| Single or split traits? | Split: ForgeReader + ForgeWriter |
| Where does ForgeWriter live? | `src/repo/` - it's about repo operations |
| Parse commits where? | In git scrape, before forge scrape |

## Open Questions (Remaining)

1. **Rate limiting**: How to handle across multiple ref repos?
   - Cache aggressively in eventlog
   - Incremental updates via `since`
   - Track last-fetched timestamp per repo

2. **PR scraping strategy**: All PRs or only commit-referenced?
   - Start with commit-referenced (we have the numbers)
   - Expand to recent PRs if surface layer needs them

---

## Success Criteria

1. `ForgeReader` trait with 3 methods, `ForgeWriter` trait with 4 methods
2. `src/forge/github/internal.rs` contains all `gh` CLI parsing
3. `git.commit` events include parsed conventional commit fields
4. `forge.pr` events available for ref repos with PR context
5. Repos without forge work gracefully (empty results, no errors)
6. Clear "Do X" for each module in the hierarchy

---

## References

- `layer/core/unix-philosophy.md` - Decomposition into focused tools
- `layer/core/dependable-rust.md` - Black-box module pattern
- `layer/core/adapter-pattern.md` - Trait-based abstraction
- Session 20260109-170426: Initial exploration
- Session 20260110-061843: Design refinement
