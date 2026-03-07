# Design: GitHub Connector — First Native Child on Pipe Architecture

## Approach

New binary crate `children/github-connector/` implementing the
`Child` trait from patina-pipe. Migrates the GitHub REST API client
from `plugins/forge/src/github.rs` (450 LOC) — replaces `host_http`
with direct `reqwest` calls, `host_emit` with `FactEmitter::emit()`.

The connector uses `github.*` schema (not `forge.*`). The WASM forge
plugin continues to work with `forge.*` schema — both coexist to
prove dual runtimes under mother-broker.

After parity is verified, `src/forge/` (2,216 LOC) and
`src/commands/scrape/forge/` (705 LOC) are deleted.

## 1. Crate Structure

```
children/github-connector/
  Cargo.toml
  child.toml              # manifest for Mother
  src/
    main.rs               # Child trait impl + main()
    github.rs             # GitHub REST API client (migrated)
```

### 1.1 Cargo.toml

```toml
[package]
name = "github-connector"
version = "0.1.0"
edition = "2021"
description = "GitHub connector for Patina — issues and PRs via REST API"

[dependencies]
patina-pipe = { path = "../../crates/patina-pipe" }
patina-pipe-types = { path = "../../crates/patina-pipe-types" }
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
reqwest = { version = "0.12", features = ["blocking", "json", "rustls-tls"], default-features = false }
```

Workspace Cargo.toml adds:
```toml
[workspace]
members = [..., "children/github-connector"]
```

### 1.2 child.toml

```toml
[child]
name = "github-connector"
version = "0.1.0"
description = "GitHub issues and PRs via REST API"
type = "connector"
runtime = "native"
lifecycle = "poll"

[capabilities]
data_types = ["issues", "prs"]
supports_incremental = true

[domains]
allowed = ["api.github.com"]

[auth]
required = true
provider = "github"

[schemas.github]
package = "patina:schema/github@1.0.0"
```

## 2. github.* Schema Definition

New schema in `.patina/schemas/github/schema.toml`. Same data shape
as forge facts — the difference is the namespace.

```toml
[schema]
name = "github"
version = "1.0.0"
package = "patina:schema/github@1.0.0"
description = "GitHub issues and pull requests (native connector)"

[[facts]]
name = "issue"
event_type = "github.issue"
record = "issue"

[[facts]]
name = "pull-request"
event_type = "github.pr"
record = "pull-request"

[embedding]
offset_slot = 6           # next available after forge (slot 5)
corpus_query = """
SELECT seq,
       json_extract(data, '$.title') || ' ' ||
       COALESCE(json_extract(data, '$.body'), '')
       as content
FROM eventlog
WHERE event_type LIKE 'github.%'
"""

[[indexes]]
fact = "issue"
fts_fields = ["title", "body"]
table = "github_issues"

[[indexes]]
fact = "pull-request"
fts_fields = ["title", "body", "comments"]
table = "github_prs"
```

Schema is installed by copying to `.patina/schemas/github/` in the
project (or globally in `~/.patina/schemas/github/`). The connector
ships with its schema — no separate install step.

## 3. Child Trait Implementation

### 3.1 main.rs

```rust
use patina_pipe::{run, Child, FactEmitter};
use patina_pipe_types::*;

mod github;
use github::GitHubClient;

/// Delay between API calls (ms). GitHub allows 5000/hour.
const RATE_LIMIT_MS: u64 = 750;

struct GitHubConnector {
    /// Auth token from pipe/initialize, None until initialized.
    auth_token: Option<String>,
}

impl Child for GitHubConnector {
    fn capabilities(&self) -> Capabilities {
        Capabilities {
            provider: "github".to_string(),
            data_types: vec!["issues".to_string(), "prs".to_string()],
            supports_incremental: true,
        }
    }

    fn initialize(&mut self, params: &InitializeParams) -> Result<(), PipeError> {
        self.auth_token = params.auth.as_ref().map(|a| a.token.clone());
        eprintln!("[github] initialized, auth: {}",
            if self.auth_token.is_some() { "provided" } else { "none" });
        Ok(())
    }

    fn fetch(
        &mut self,
        params: &FetchParams,
        emitter: &mut FactEmitter,
    ) -> Result<FetchResult, PipeError> {
        // Extract owner/repo from params
        let owner = params.params.get("owner")
            .and_then(|v| v.as_str())
            .ok_or_else(|| PipeError::Fatal {
                message: "missing 'owner' in params".into(),
            })?;
        let repo = params.params.get("repo")
            .and_then(|v| v.as_str())
            .ok_or_else(|| PipeError::Fatal {
                message: "missing 'repo' in params".into(),
            })?;

        let limit = if params.limit == 0 { 100 } else { params.limit as usize };
        let since = params.since.as_deref();

        let client = GitHubClient::new(owner, repo, self.auth_token.as_deref());

        // Determine which types to fetch
        let fetch_issues = params.types.is_empty()
            || params.types.iter().any(|t| t == "issues");
        let fetch_prs = params.types.is_empty()
            || params.types.iter().any(|t| t == "prs");

        if fetch_issues {
            eprintln!("[github] fetching issues for {}/{} (limit: {})", owner, repo, limit);
            client.fetch_and_emit_issues(limit, since, emitter)?;
        }

        if fetch_prs {
            eprintln!("[github] fetching PRs for {}/{} (limit: {})", owner, repo, limit);
            client.fetch_and_emit_prs(limit, since, emitter)?;
        }

        // Cursor: use current time as the "since" for next fetch
        let cursor = Some(chrono::Utc::now().to_rfc3339());

        eprintln!("[github] fetch complete: {} facts emitted", emitter.count());

        Ok(FetchResult {
            emitted: emitter.count(),
            cursor,
        })
    }

    fn health(&self) -> Result<HealthStatus, PipeError> {
        // Quick rate limit check — GET /rate_limit is free
        // For now, just report ok if we have a token
        Ok(HealthStatus {
            status: if self.auth_token.is_some() { Status::Ok } else { Status::Degraded },
            message: Some(if self.auth_token.is_some() {
                "authenticated".to_string()
            } else {
                "no auth token — requests may be rate limited".to_string()
            }),
            latency_ms: None,
        })
    }
}

fn main() {
    if let Err(e) = run(GitHubConnector { auth_token: None }) {
        eprintln!("[github] fatal: {}", e);
        std::process::exit(1);
    }
}
```

### 3.2 github.rs — Migration from plugins/forge/src/github.rs

Line-by-line migration guide. The domain logic (pagination, JSON
parsing, data shapes) stays identical. Only the I/O changes.

| Old (WASM via host functions) | New (native via reqwest) |
|------|------|
| `host_http::get(url)?` | `self.get(url)?` (reqwest) |
| `host_emit::emit_fact("forge", "issue", &json)` | `emitter.emit("github", "issue", &value)?` |
| `host_log::log(Level::Info, msg)` | `eprintln!("[github] {}", msg)` |
| `Result<_, String>` | `Result<_, PipeError>` |
| `crate::rate_limit_sleep()` | `std::thread::sleep(Duration::from_millis(RATE_LIMIT_MS))` |

```rust
use patina_pipe::FactEmitter;
use patina_pipe_types::PipeError;
use serde::Deserialize;

const PER_PAGE: usize = 100;
const RATE_LIMIT_MS: u64 = 750;

// ================================================================
// GitHub REST API JSON types (unchanged from forge)
// ================================================================

#[derive(Debug, Deserialize)]
struct GhApiIssue {
    number: i64,
    title: String,
    body: Option<String>,
    state: String,
    user: GhApiUser,
    labels: Vec<GhApiLabel>,
    created_at: String,
    updated_at: String,
    html_url: String,
    pull_request: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
struct GhApiPullRequest {
    number: i64,
    title: String,
    body: Option<String>,
    state: String,
    user: GhApiUser,
    labels: Vec<GhApiLabel>,
    created_at: String,
    updated_at: String,
    merged_at: Option<String>,
    html_url: String,
}

#[derive(Debug, Deserialize)]
struct GhApiUser { login: String }

#[derive(Debug, Deserialize)]
struct GhApiLabel { name: String }

#[derive(Debug, Deserialize)]
struct GhApiComment {
    user: GhApiUser,
    body: String,
    created_at: String,
}

#[derive(Debug, Deserialize)]
struct GhApiReview {
    user: GhApiUser,
    body: Option<String>,
    state: String,
    submitted_at: Option<String>,
}

// ================================================================
// Client
// ================================================================

pub struct GitHubClient<'a> {
    owner: &'a str,
    repo: &'a str,
    client: reqwest::blocking::Client,
}

impl<'a> GitHubClient<'a> {
    pub fn new(owner: &'a str, repo: &'a str, token: Option<&'a str>) -> Self {
        let mut builder = reqwest::blocking::Client::builder()
            .user_agent(format!("patina/{}", env!("CARGO_PKG_VERSION")));

        // Auth via default headers if token provided
        if let Some(token) = token {
            let mut headers = reqwest::header::HeaderMap::new();
            headers.insert(
                reqwest::header::AUTHORIZATION,
                reqwest::header::HeaderValue::from_str(&format!("Bearer {}", token))
                    .unwrap_or_else(|_| reqwest::header::HeaderValue::from_static("")),
            );
            builder = builder.default_headers(headers);
        }

        Self {
            owner, repo,
            client: builder.build().expect("build reqwest client"),
        }
    }

    fn api_base(&self) -> String {
        format!("https://api.github.com/repos/{}/{}", self.owner, self.repo)
    }

    /// GET with error mapping to PipeError.
    fn get(&self, url: &str) -> Result<String, PipeError> {
        let response = self.client.get(url).send().map_err(|e| {
            PipeError::Transient { message: format!("HTTP GET failed: {}", e), retry_after_ms: None }
        })?;

        let status = response.status().as_u16();
        let body = response.text().map_err(|e| {
            PipeError::Transient { message: format!("read body: {}", e), retry_after_ms: None }
        })?;

        match status {
            200 => Ok(body),
            401 | 403 => {
                // Check for rate limit
                if body.contains("rate limit") || body.contains("API rate") {
                    Err(PipeError::RateLimited {
                        message: format!("GitHub API rate limited ({})", status),
                        retry_after_ms: 60_000,
                    })
                } else {
                    Err(PipeError::Fatal {
                        message: format!("GitHub API {} ({}): {}", status, url, truncate(&body, 200)),
                    })
                }
            }
            404 => Err(PipeError::Fatal {
                message: format!("not found (404): {}", url),
            }),
            429 => {
                // Extract retry-after from response body if available
                Err(PipeError::RateLimited {
                    message: "GitHub API rate limited (429)".into(),
                    retry_after_ms: 60_000,
                })
            }
            500..=599 => Err(PipeError::Transient {
                message: format!("GitHub API server error ({})", status),
                retry_after_ms: Some(5_000),
            }),
            _ => Err(PipeError::Transient {
                message: format!("GitHub API error ({}): {}", status, truncate(&body, 200)),
                retry_after_ms: None,
            }),
        }
    }

    // ============================================================
    // Issues — same logic as plugins/forge/src/github.rs
    // ============================================================

    pub fn fetch_and_emit_issues(
        &self,
        limit: usize,
        since: Option<&str>,
        emitter: &mut FactEmitter,
    ) -> Result<(), PipeError> {
        let mut fetched = 0;
        let mut page = 1;

        loop {
            if fetched >= limit { break; }

            let url = self.issues_url(page, since);
            let body = self.get(&url)?;
            let items: Vec<GhApiIssue> = serde_json::from_str(&body)
                .map_err(|e| PipeError::Fatal { message: format!("JSON parse: {}", e) })?;

            if items.is_empty() { break; }
            let page_count = items.len();

            for item in items {
                if fetched >= limit { break; }
                if item.pull_request.is_some() { continue; } // skip PRs

                let data = issue_to_json(&item);
                emitter.emit("github", "issue", &data)?;
                fetched += 1;
            }

            if page_count < PER_PAGE { break; }
            page += 1;
            rate_limit_sleep();
        }

        Ok(())
    }

    fn issues_url(&self, page: usize, since: Option<&str>) -> String {
        let mut url = format!(
            "{}/issues?state=all&per_page={}&page={}&sort=updated&direction=desc",
            self.api_base(), PER_PAGE, page,
        );
        if let Some(ts) = since {
            url.push_str(&format!("&since={}", ts));
        }
        url
    }

    // ============================================================
    // Pull Requests — same logic as plugins/forge/src/github.rs
    // ============================================================

    pub fn fetch_and_emit_prs(
        &self,
        limit: usize,
        since: Option<&str>,
        emitter: &mut FactEmitter,
    ) -> Result<(), PipeError> {
        let mut fetched = 0;
        let mut page = 1;

        loop {
            if fetched >= limit { break; }

            let url = self.prs_url(page, since);
            let body = self.get(&url)?;
            let items: Vec<GhApiPullRequest> = serde_json::from_str(&body)
                .map_err(|e| PipeError::Fatal { message: format!("JSON parse: {}", e) })?;

            if items.is_empty() { break; }
            let page_count = items.len();

            for item in items {
                if fetched >= limit { break; }

                rate_limit_sleep();
                let comments = self.fetch_pr_comments(item.number).unwrap_or_default();
                rate_limit_sleep();
                let reviews = self.fetch_pr_reviews(item.number).unwrap_or_default();

                let data = pr_to_json(&item, &comments, &reviews);
                emitter.emit("github", "pull-request", &data)?;
                fetched += 1;
            }

            if page_count < PER_PAGE { break; }
            page += 1;
            rate_limit_sleep();
        }

        Ok(())
    }

    fn fetch_pr_comments(&self, number: i64) -> Result<Vec<GhApiComment>, PipeError> {
        let url = format!("{}/issues/{}/comments?per_page=100", self.api_base(), number);
        let body = self.get(&url)?;
        serde_json::from_str(&body)
            .map_err(|e| PipeError::Fatal { message: format!("JSON parse: {}", e) })
    }

    fn fetch_pr_reviews(&self, number: i64) -> Result<Vec<GhApiReview>, PipeError> {
        let url = format!("{}/pulls/{}/reviews?per_page=100", self.api_base(), number);
        let body = self.get(&url)?;
        serde_json::from_str(&body)
            .map_err(|e| PipeError::Fatal { message: format!("JSON parse: {}", e) })
    }

    fn prs_url(&self, page: usize, _since: Option<&str>) -> String {
        format!(
            "{}/pulls?state=all&per_page={}&page={}&sort=updated&direction=desc",
            self.api_base(), PER_PAGE, page,
        )
    }
}

// ================================================================
// JSON conversion — identical data shape as forge
// ================================================================

fn issue_to_json(item: &GhApiIssue) -> serde_json::Value {
    let state = match item.state.as_str() { "open" => "open", _ => "closed" };
    let labels: Vec<&str> = item.labels.iter().map(|l| l.name.as_str()).collect();

    serde_json::json!({
        "number": item.number,
        "title": item.title,
        "body": item.body,
        "state": state,
        "labels": labels,
        "author": item.user.login,
        "url": item.html_url,
        "created_at": item.created_at,
        "updated_at": item.updated_at,
    })
}

fn pr_to_json(
    item: &GhApiPullRequest,
    comments: &[GhApiComment],
    reviews: &[GhApiReview],
) -> serde_json::Value {
    let state = if item.merged_at.is_some() { "merged" }
        else { match item.state.as_str() { "open" => "open", _ => "closed" } };
    let labels: Vec<&str> = item.labels.iter().map(|l| l.name.as_str()).collect();

    let mut structured_comments: Vec<serde_json::Value> = Vec::new();
    for c in comments {
        structured_comments.push(serde_json::json!({
            "author": c.user.login, "body": c.body, "created_at": c.created_at,
        }));
    }
    for r in reviews {
        if let Some(body) = &r.body {
            if !body.is_empty() {
                structured_comments.push(serde_json::json!({
                    "author": r.user.login, "body": body, "created_at": r.submitted_at,
                }));
            }
        }
    }

    let comments_text: String = structured_comments.iter()
        .filter_map(|c| {
            let a = c["author"].as_str()?;
            let b = c["body"].as_str()?;
            Some(format!("{}: {}", a, b))
        })
        .collect::<Vec<_>>()
        .join("\n");

    let approvals = reviews.iter().filter(|r| r.state == "APPROVED").count() as i32;

    serde_json::json!({
        "number": item.number,
        "title": item.title,
        "body": item.body,
        "state": state,
        "labels": labels,
        "author": item.user.login,
        "url": item.html_url,
        "created_at": item.created_at,
        "updated_at": item.updated_at,
        "merged_at": item.merged_at,
        "linked_issues": [],
        "comments": structured_comments,
        "comments_text": comments_text,
        "approvals": approvals,
    })
}

fn rate_limit_sleep() {
    std::thread::sleep(std::time::Duration::from_millis(RATE_LIMIT_MS));
}

fn truncate(s: &str, max: usize) -> &str {
    if s.len() <= max { s } else { s.get(..max).unwrap_or(s) }
}
```

## 4. Parity Verification Plan

Compare output from old (`patina scrape forge`) and new
(`patina mother run github`) against the same repo.

### 4.1 Verification Steps

1. Run `patina scrape forge` against a test repo, capture events
   from events.db where `event_type LIKE 'forge.%'`
2. Run `patina mother run github` against same repo, capture events
   where `event_type LIKE 'github.%'`
3. Compare data shapes:

```sql
-- Forge issues
SELECT json_extract(data, '$.number'),
       json_extract(data, '$.title'),
       json_extract(data, '$.state')
FROM eventlog WHERE event_type = 'forge.issue'
ORDER BY json_extract(data, '$.number');

-- GitHub issues (should match)
SELECT json_extract(data, '$.number'),
       json_extract(data, '$.title'),
       json_extract(data, '$.state')
FROM eventlog WHERE event_type = 'github.issue'
ORDER BY json_extract(data, '$.number');
```

### 4.2 Expected Differences

| Field | forge.* | github.* | Why |
|-------|---------|----------|-----|
| event_type | forge.issue | github.issue | Schema namespace |
| source_id | plugin:patina-forge | child:github-connector | Source type |
| content_hash | (absent) | blake3:... | New feature |
| timestamp | host-generated | host-generated | Both set by Mother |

Data shape (`$.number`, `$.title`, etc.) should be identical.

## 5. src/forge/ Deletion Checklist

After parity is verified, delete the old gh CLI wrapper. Exact
files and their dependencies:

| File | LOC | Depends on it | Safe to delete? |
|------|-----|---------------|-----------------|
| `src/forge/mod.rs` | 181 | `src/commands/scrape/forge/mod.rs` | Yes, after scrape forge removed |
| `src/forge/types.rs` | 80 | `src/forge/` internal | Yes |
| `src/forge/writer.rs` | 231 | `src/forge/` internal | Yes |
| `src/forge/github/mod.rs` | 66 | `src/forge/mod.rs` | Yes |
| `src/forge/github/internal.rs` | 376 | `src/forge/github/mod.rs` | Yes |
| `src/forge/sync/mod.rs` | 93 | `src/forge/mod.rs` | Yes |
| `src/forge/sync/internal.rs` | 615 | `src/forge/sync/mod.rs` | Yes |
| `src/forge/none.rs` | 41 | `src/forge/mod.rs` | Yes |
| `src/commands/scrape/forge/mod.rs` | 705 | `src/commands/scrape/mod.rs` | Yes, remove subcommand |

**Pre-deletion checks:**
1. `grep -r "forge::" src/ --include="*.rs"` — find all imports
2. `grep -r "src/forge" src/ --include="*.rs"` — find path refs
3. Remove `mod forge;` from `src/lib.rs` (or wherever declared)
4. Remove forge subcommand from `src/commands/scrape/mod.rs`
5. `cargo build --release` — verify clean compile
6. Keep `plugins/forge/` — WASM plugin stays

**What stays:**
- `plugins/forge/` — WASM plugin, proves dual runtime under
  mother-broker
- `.patina/schemas/forge/schema.toml` — forge schema for WASM plugin
- `src/commands/scrape/mod.rs` — scrape command minus forge subcommand
- Projection queries that read `forge.*` events still work

## 6. Schema Installation

The github-connector ships with its schema. Installation:

```bash
# Manual (during development)
mkdir -p .patina/schemas/github/
cp children/github-connector/schema.toml .patina/schemas/github/schema.toml

# Future: automatic via child.toml [schemas.github] declaration
# Mother reads the package reference and installs on first run
```

For this spec, manual installation is sufficient. Mother-broker spec
handles automatic schema installation from child manifests.

## Commits

1. `github-connector: create binary crate with Child trait impl`
   — children/github-connector/ with Cargo.toml, child.toml,
   main.rs implementing Child trait (capabilities, initialize,
   fetch, health).

2. `github-connector: migrate GitHub REST API client`
   — github.rs migrated from plugins/forge/src/github.rs. Replace
   host_http with reqwest, host_emit with emitter.emit(), error
   types with PipeError. Data shapes unchanged.

3. `github-connector: add github.* schema definition`
   — .patina/schemas/github/schema.toml with github.issue and
   github.pr fact types.

4. `github-connector: wire patina mother run github`
   — Mother-side spawn logic for github-connector. pipe/initialize
   with credentials, pipe/fetch, pipe/shutdown. Verify facts land
   in events.db.

5. `github-connector: parity verification`
   — Run both forge and github connectors, compare data shapes.
   Document results. This is a verification commit, not code.

6. `forge: delete src/forge/ (2,216 LOC) and scrape forge command (705 LOC)`
   — Remove old gh CLI wrapper. Keep plugins/forge/ (WASM).
   Update mod declarations, verify clean compile.

## Key Files

- `children/github-connector/src/main.rs` — Child trait impl
- `children/github-connector/src/github.rs` — REST API client
- `children/github-connector/child.toml` — manifest for Mother
- `.patina/schemas/github/schema.toml` — schema definition
- `plugins/forge/src/github.rs` — migration source (450 LOC)
- `src/forge/` — deletion target (2,216 LOC)
- `src/commands/scrape/forge/mod.rs` — deletion target (705 LOC)

## Open Questions

1. **chrono dependency.** The cursor uses `chrono::Utc::now()`. chrono
   is already in the main binary's dependency tree but not currently
   in the connector's deps. Add it, or use a simpler timestamp
   approach (e.g., pass the connector the last `updated_at` from
   fetched items as the cursor instead of wall clock time)?
