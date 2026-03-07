//! Repos child plugin — reference repository freshness monitoring for Mother daemon.
//!
//! Phase 1 scope: host-fed state only (no WASI filesystem access).
//! The host pushes repo info via handle(), tick() returns toys for stale repos.
//!
//! Actions:
//! - "report_repo"     → host tells child about a repo (name, path, last_indexed)
//! - "check_freshness" → return staleness state for all known repos

use patina_sdk::mother_child::log;
use patina_sdk::{register_plugin, ChildHealth, HealthStatus, MotherChildPlugin, Toy};

/// Staleness threshold in seconds (24 hours).
const STALE_THRESHOLD_SECS: u64 = 86400;

/// A tracked reference repository.
struct RepoInfo {
    name: String,
    path: String,
    /// Unix timestamp (seconds) of last successful index.
    last_indexed: u64,
}

#[derive(Default)]
struct ReposChild {
    repos: Vec<RepoInfo>,
}

impl MotherChildPlugin for ReposChild {
    fn name(&self) -> String {
        "repos".into()
    }

    fn on_load(&mut self) -> Result<(), String> {
        log::log(
            log::LogLevel::Info,
            "repos child loaded (Phase 1: host-fed state)",
        );
        Ok(())
    }

    fn health(&self) -> ChildHealth {
        if self.repos.is_empty() {
            return ChildHealth {
                status: HealthStatus::Healthy,
                reason: None,
            };
        }
        let now = current_time_secs();
        let stale_count = self
            .repos
            .iter()
            .filter(|r| now.saturating_sub(r.last_indexed) > STALE_THRESHOLD_SECS)
            .count();
        if stale_count == 0 {
            ChildHealth {
                status: HealthStatus::Healthy,
                reason: None,
            }
        } else {
            ChildHealth {
                status: HealthStatus::Degraded,
                reason: Some(format!("{} repo(s) stale", stale_count)),
            }
        }
    }

    fn handle(&mut self, action: &str, payload: &str) -> Result<String, String> {
        match action {
            "report_repo" => self.report_repo(payload),
            "check_freshness" => self.check_freshness(),
            _ => Err(format!("repos: unknown action '{}'", action)),
        }
    }

    fn tick(&mut self) -> Vec<Toy> {
        if self.repos.is_empty() {
            return vec![];
        }

        let now = current_time_secs();
        let mut toys = Vec::new();

        for repo in &self.repos {
            let age = now.saturating_sub(repo.last_indexed);
            if age > STALE_THRESHOLD_SECS {
                log::log(
                    log::LogLevel::Info,
                    &format!(
                        "repo '{}' stale ({}s since last index), requesting toys",
                        repo.name, age
                    ),
                );

                // Toy 1: git pull
                toys.push(Toy {
                    name: format!("pull-{}", repo.name),
                    command: "git".into(),
                    args: vec!["-C".into(), repo.path.clone(), "pull".into()],
                });

                // Toy 2: re-scrape
                toys.push(Toy {
                    name: format!("scrape-{}", repo.name),
                    command: "patina".into(),
                    args: vec!["scrape".into(), "--repo".into(), repo.name.clone()],
                });
            }
        }

        toys
    }
}

impl ReposChild {
    /// Register or update a repo's state. Host feeds this data to the child.
    fn report_repo(&mut self, payload: &str) -> Result<String, String> {
        let value: serde_json::Value =
            serde_json::from_str(payload).map_err(|e| format!("invalid JSON: {}", e))?;

        let name = value
            .get("name")
            .and_then(|v| v.as_str())
            .ok_or("missing 'name' field")?
            .to_string();

        let path = value
            .get("path")
            .and_then(|v| v.as_str())
            .ok_or("missing 'path' field")?
            .to_string();

        let last_indexed = value
            .get("last_indexed")
            .and_then(|v| v.as_u64())
            .unwrap_or(0);

        // Update existing or insert new
        if let Some(existing) = self.repos.iter_mut().find(|r| r.name == name) {
            existing.path = path;
            existing.last_indexed = last_indexed;
        } else {
            self.repos.push(RepoInfo {
                name: name.clone(),
                path,
                last_indexed,
            });
        }

        let response = serde_json::json!({
            "status": "registered",
            "name": name,
            "total_repos": self.repos.len(),
        });
        serde_json::to_string(&response).map_err(|e| format!("json error: {}", e))
    }

    /// Return freshness state for all known repos.
    fn check_freshness(&self) -> Result<String, String> {
        let now = current_time_secs();
        let repos: Vec<serde_json::Value> = self
            .repos
            .iter()
            .map(|r| {
                let age = now.saturating_sub(r.last_indexed);
                serde_json::json!({
                    "name": r.name,
                    "path": r.path,
                    "last_indexed": r.last_indexed,
                    "age_secs": age,
                    "stale": age > STALE_THRESHOLD_SECS,
                })
            })
            .collect();

        let response = serde_json::json!({
            "repos": repos,
            "total": self.repos.len(),
            "stale_count": repos.iter().filter(|r| r["stale"] == true).count(),
        });
        serde_json::to_string(&response).map_err(|e| format!("json error: {}", e))
    }
}

/// Get current time as Unix seconds.
/// In WASM without WASI clocks, this returns 0.
/// In practice wasm32-wasip2 provides clocks, so this works.
fn current_time_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

register_plugin!(ReposChild);
