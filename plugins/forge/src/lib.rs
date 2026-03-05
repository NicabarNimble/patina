//! Forge connector plugin — GitHub issues and PRs via REST API.
//!
//! Replaces src/forge/ (gh CLI wrapper) with direct GitHub REST API
//! calls through host_http. Emits forge.issue and forge.pr facts
//! via host_emit with provenance=external.
//!
//! Actions:
//! - "sync"    → full/incremental sync of issues and PRs
//! - "resolve" → resolve a single #N reference (issue or PR)

use patina_sdk::mother_child::host_log;
use patina_sdk::{register_plugin, ChildHealth, HealthStatus, MotherChildPlugin, Toy};

mod github;

use github::GitHubClient;

/// Delay between API calls (milliseconds). GitHub allows 5000/hour;
/// 750ms pacing is conservative and matches the existing sync engine.
const RATE_LIMIT_MS: u64 = 750;

/// Maximum items per page for GitHub API pagination.
const PER_PAGE: usize = 100;

#[derive(Default)]
struct ForgeChild {
    /// Total issues emitted this session.
    issues_emitted: usize,
    /// Total PRs emitted this session.
    prs_emitted: usize,
}

impl MotherChildPlugin for ForgeChild {
    fn name(&self) -> String {
        "forge".into()
    }

    fn on_load(&mut self) -> Result<(), String> {
        host_log::log(
            host_log::LogLevel::Info,
            "forge connector loaded (GitHub REST API)",
        );
        Ok(())
    }

    fn health(&self) -> ChildHealth {
        ChildHealth {
            status: HealthStatus::Healthy,
            reason: Some(format!(
                "emitted {} issues, {} PRs",
                self.issues_emitted, self.prs_emitted
            )),
        }
    }

    fn handle(&mut self, action: &str, payload: &str) -> Result<String, String> {
        match action {
            "sync" => self.sync(payload),
            "resolve" => self.resolve(payload),
            _ => Err(format!("forge: unknown action '{}'", action)),
        }
    }

    fn tick(&mut self) -> Vec<Toy> {
        // Session 2: discover #N refs from recent commits via toys.
        // For now, forge runs via explicit handle("sync", ...) calls.
        vec![]
    }
}

impl ForgeChild {
    /// Full or incremental sync of issues and PRs.
    ///
    /// Payload JSON:
    /// ```json
    /// {
    ///   "owner": "anthropics",
    ///   "repo": "claude-code",
    ///   "limit": 100,
    ///   "since": "2024-01-01T00:00:00Z"  // optional, for incremental
    /// }
    /// ```
    fn sync(&mut self, payload: &str) -> Result<String, String> {
        let params: serde_json::Value =
            serde_json::from_str(payload).map_err(|e| format!("invalid JSON: {}", e))?;

        let owner = params["owner"].as_str().ok_or("missing 'owner' field")?;
        let repo = params["repo"].as_str().ok_or("missing 'repo' field")?;
        let limit = params["limit"].as_u64().unwrap_or(100) as usize;
        let since = params["since"].as_str();

        let client = GitHubClient::new(owner, repo);

        // Fetch and emit issues
        host_log::log(
            host_log::LogLevel::Info,
            &format!("syncing issues for {}/{} (limit: {})", owner, repo, limit),
        );
        let issues_result = client.fetch_and_emit_issues(limit, since)?;

        // Fetch and emit PRs
        host_log::log(
            host_log::LogLevel::Info,
            &format!("syncing PRs for {}/{} (limit: {})", owner, repo, limit),
        );
        let prs_result = client.fetch_and_emit_prs(limit, since)?;

        self.issues_emitted += issues_result.emitted;
        self.prs_emitted += prs_result.emitted;

        let response = serde_json::json!({
            "owner": owner,
            "repo": repo,
            "issues_fetched": issues_result.fetched,
            "issues_emitted": issues_result.emitted,
            "prs_fetched": prs_result.fetched,
            "prs_emitted": prs_result.emitted,
        });

        host_log::log(
            host_log::LogLevel::Info,
            &format!(
                "sync complete: {} issues ({} emitted), {} PRs ({} emitted)",
                issues_result.fetched,
                issues_result.emitted,
                prs_result.fetched,
                prs_result.emitted
            ),
        );

        serde_json::to_string(&response).map_err(|e| format!("json error: {}", e))
    }

    /// Resolve a single #N reference — try as PR first, fall back to issue.
    ///
    /// Payload JSON:
    /// ```json
    /// { "owner": "anthropics", "repo": "claude-code", "number": 42 }
    /// ```
    fn resolve(&mut self, payload: &str) -> Result<String, String> {
        let params: serde_json::Value =
            serde_json::from_str(payload).map_err(|e| format!("invalid JSON: {}", e))?;

        let owner = params["owner"].as_str().ok_or("missing 'owner' field")?;
        let repo = params["repo"].as_str().ok_or("missing 'repo' field")?;
        let number = params["number"].as_i64().ok_or("missing 'number' field")?;

        let client = GitHubClient::new(owner, repo);

        // Try PR first (more common for #N refs in commits)
        match client.fetch_single_pr(number) {
            Ok(()) => {
                self.prs_emitted += 1;
                let response = serde_json::json!({
                    "resolved": true,
                    "kind": "pr",
                    "number": number,
                });
                return serde_json::to_string(&response).map_err(|e| format!("json error: {}", e));
            }
            Err(e) => {
                host_log::log(
                    host_log::LogLevel::Debug,
                    &format!("#{} not a PR ({}), trying issue", number, e),
                );
            }
        }

        // Fall back to issue
        rate_limit_sleep();
        match client.fetch_single_issue(number) {
            Ok(()) => {
                self.issues_emitted += 1;
                let response = serde_json::json!({
                    "resolved": true,
                    "kind": "issue",
                    "number": number,
                });
                serde_json::to_string(&response).map_err(|e| format!("json error: {}", e))
            }
            Err(e) => Err(format!("failed to resolve #{}: {}", number, e)),
        }
    }
}

/// Sleep for rate limiting between API calls.
fn rate_limit_sleep() {
    std::thread::sleep(std::time::Duration::from_millis(RATE_LIMIT_MS));
}

register_plugin!(ForgeChild);
