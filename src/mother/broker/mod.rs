//! Broker — routing engine for Mother.
//!
//! Routes source data into project event streams based on sources.toml
//! declarations. The broker owns source fetch + write behavior and avoids
//! child-specific runtime coupling.

// Data types and source parsing now live in the mother crate.
// Re-export for existing callers.
pub use mother_crate::broker::cursor;
pub use mother_crate::broker::sources;
pub use mother_crate::broker::{SourceStatus, WriteResult};

use anyhow::{Context, Result};
use std::collections::HashMap;
use std::path::Path;

use self::cursor::get_cursor;
use self::sources::{Destination, SourceEntry};

/// Run a single source: resolve auth, fetch facts, and write broker events.
///
/// FAIL CLOSED: auth resolution errors propagate — the child is never
/// executed without a valid AuthPlan.
pub fn run_source(
    source: &SourceEntry,
    project_root: &Path,
    _no_sandbox: bool,
) -> Result<WriteResult> {
    // 1. Load connection record from connect module
    let record = crate::connect::load(&source.connection)
        .map_err(|e| anyhow::anyhow!("{}", e))
        .with_context(|| format!("loading connection for source '{}'", source.name))?;

    // 2. Resolve auth plan — FAIL CLOSED (no "proceeding without auth")
    let auth_plan = crate::connect::resolve_auth(&record)
        .map_err(|e| anyhow::anyhow!("{}", e))
        .with_context(|| format!("resolving auth for source '{}'", source.name))?;

    // 3. Fetch and persist source events directly via broker runtime.
    route_source_via_broker(source, project_root, &auth_plan)
}

fn route_source_via_broker(
    source: &SourceEntry,
    project_root: &Path,
    auth_plan: &crate::connect::AuthPlan,
) -> Result<WriteResult> {
    const DEFAULT_TYPES: [&str; 2] = ["issues", "prs"];

    crate::connect::require_auth_plan_domain(&source.connection, auth_plan, "api.github.com")
        .map_err(|e| {
            anyhow::anyhow!(
                "source '{}' connection is invalid for GitHub lake route: {}",
                source.name,
                e
            )
        })?;

    let owner = source
        .params
        .get("owner")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow::anyhow!("source '{}' is missing params.owner", source.name))?;
    let repo = source
        .params
        .get("repo")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow::anyhow!("source '{}' is missing params.repo", source.name))?;
    let include_pr_commits = source
        .params
        .get("include_pr_commits")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    let source_types = if source.types.is_empty() {
        DEFAULT_TYPES.iter().map(|t| (*t).to_string()).collect()
    } else {
        source.types.clone()
    };

    let normalized_types: Vec<String> = source_types
        .into_iter()
        .filter_map(|value| match value.as_str() {
            "issues" => Some("issues".to_string()),
            "prs" | "pulls" => Some("prs".to_string()),
            _ => None,
        })
        .collect();

    if normalized_types.is_empty() {
        anyhow::bail!(
            "source '{}' has no supported types (expected one of: issues, prs)",
            source.name
        );
    }

    let conn = crate::eventlog::open_events_db_at(project_root)
        .with_context(|| format!("opening events.db for {}", project_root.display()))?;
    let cursor_before = get_cursor(&conn, &source.name)?;
    let source_id = format!("source:{}", source.name);

    let mut inserted = 0_u64;
    let mut max_cursor = cursor_before.clone();

    for data_type in &normalized_types {
        let rows = fetch_github_rows(
            owner,
            repo,
            data_type,
            cursor_before.as_deref(),
            include_pr_commits,
            auth_plan,
        )?;

        let event_type = match data_type.as_str() {
            "issues" => "github.issue",
            "prs" => "github.pr",
            _ => continue,
        };

        for row in rows {
            let timestamp = row
                .get("updated_at")
                .and_then(|v| v.as_str())
                .map(ToString::to_string)
                .unwrap_or_else(|| chrono::Utc::now().to_rfc3339());
            let payload = serde_json::to_string(&row)?;
            crate::eventlog::insert_event(
                &conn, event_type, &timestamp, &source_id, None, &payload,
            )?;
            inserted += 1;

            if let Some(updated_at) = row.get("updated_at").and_then(|v| v.as_str()) {
                if max_cursor
                    .as_deref()
                    .map(|c| updated_at > c)
                    .unwrap_or(true)
                {
                    max_cursor = Some(updated_at.to_string());
                }
            }
        }
    }

    if let Some(cursor_value) = max_cursor.as_deref() {
        save_broker_cursor(&conn, &source.name, cursor_value)?;
        let runtime = crate::mother::MotherRuntimeStore::default();
        let lake_name = match &source.destination {
            Destination::Lake { name } => name.as_str(),
            Destination::Project => "project",
        };
        for data_type in &normalized_types {
            runtime.save_lake_cursor(&crate::mother::state::LakeCursorUpdate {
                lake_name,
                source_name: &source.name,
                data_type,
                cursor_value: Some(cursor_value),
                records_written: inserted,
                status: "ok",
                last_error: None,
            })?;
        }
    }

    Ok(WriteResult {
        inserted,
        dedup_skipped: 0,
        cursor: max_cursor,
    })
}

fn save_broker_cursor(
    conn: &rusqlite::Connection,
    source_name: &str,
    cursor_value: &str,
) -> Result<()> {
    conn.execute(
        "INSERT INTO broker_cursors (source_name, cursor_value, updated_at)
         VALUES (?1, ?2, ?3)
         ON CONFLICT(source_name)
         DO UPDATE SET cursor_value = excluded.cursor_value, updated_at = excluded.updated_at",
        rusqlite::params![source_name, cursor_value, chrono::Utc::now().to_rfc3339()],
    )?;
    Ok(())
}

fn fetch_github_rows(
    owner: &str,
    repo: &str,
    data_type: &str,
    since: Option<&str>,
    include_pr_commits: bool,
    auth_plan: &crate::connect::AuthPlan,
) -> Result<Vec<serde_json::Value>> {
    let credential = auth_plan
        .credential
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("connection has no credential"))?;

    let mut url = match data_type {
        "issues" => {
            format!("https://api.github.com/repos/{owner}/{repo}/issues?state=all&per_page=100")
        }
        "prs" => {
            format!("https://api.github.com/repos/{owner}/{repo}/pulls?state=all&per_page=100")
        }
        other => anyhow::bail!("unsupported data_type '{}'", other),
    };

    if let Some(since_value) = since {
        url.push_str("&since=");
        url.push_str(since_value);
    }
    if data_type == "prs" && include_pr_commits {
        url.push_str("&sort=updated&direction=desc");
    }

    let client = reqwest::blocking::Client::new();
    let mut rows = Vec::new();
    let mut next_url = Some(url);

    while let Some(current_url) = next_url.take() {
        let mut request = client
            .get(&current_url)
            .header("Accept", "application/vnd.github+json")
            .header("X-GitHub-Api-Version", "2022-11-28")
            .header("User-Agent", "patina");

        request = match &credential.injection {
            crate::connect::InjectionStrategy::Bearer => {
                request.header("Authorization", format!("Bearer {}", credential.value))
            }
            crate::connect::InjectionStrategy::Header { name } => {
                request.header(name, credential.value.as_str())
            }
            crate::connect::InjectionStrategy::InProcess => {
                anyhow::bail!("in-process credential injection is not supported for broker HTTP")
            }
        };

        let response = request
            .send()
            .with_context(|| format!("requesting {}", current_url))?;
        let status = response.status();
        let headers: HashMap<String, String> = response
            .headers()
            .iter()
            .map(|(k, v)| {
                (
                    k.as_str().to_string(),
                    v.to_str().unwrap_or_default().to_string(),
                )
            })
            .collect();
        let body = response.text().unwrap_or_default();

        if !status.is_success() {
            anyhow::bail!("GitHub API returned {}: {}", status, body);
        }

        let payload: serde_json::Value =
            serde_json::from_str(&body).with_context(|| "parsing GitHub response as JSON")?;
        let arr = payload
            .as_array()
            .ok_or_else(|| anyhow::anyhow!("GitHub API response must be a JSON array"))?;
        rows.extend(arr.iter().cloned());

        next_url = headers.get("link").and_then(|value| parse_next_link(value));
    }

    Ok(rows)
}

fn parse_next_link(link_header: &str) -> Option<String> {
    link_header.split(',').map(str::trim).find_map(|part| {
        if !part.contains("rel=\"next\"") {
            return None;
        }
        let start = part.find('<')? + 1;
        let end = part[start..].find('>')? + start;
        Some(part[start..end].to_string())
    })
}

/// Get status for all sources in a project.
pub fn status(project_root: &Path) -> Result<Vec<SourceStatus>> {
    let project_sources = sources::load_project_sources(project_root)?;
    let sources_list = match project_sources {
        Some(ps) => ps.sources,
        None => return Ok(vec![]),
    };

    let events_conn = crate::eventlog::open_events_db_at(project_root)?;
    let mut statuses = Vec::new();

    for source in &sources_list {
        let cursor = get_cursor(&events_conn, &source.name)?;

        // Get last run timestamp from cursor's updated_at
        let last_run: Option<String> = events_conn
            .query_row(
                "SELECT updated_at FROM broker_cursors WHERE source_name = ?1",
                [&source.name],
                |row| row.get(0),
            )
            .ok();

        // Count facts from this source (current + legacy source id patterns).
        let source_id = format!("source:{}", source.name);
        let legacy_source_id = format!("child:{}", source.name);

        let fact_count: i64 = events_conn
            .query_row(
                "SELECT COUNT(*) FROM eventlog WHERE source_id = ?1 OR source_id = ?2",
                rusqlite::params![source_id, legacy_source_id],
                |row| row.get(0),
            )
            .unwrap_or(0);

        statuses.push(SourceStatus {
            name: source.name.clone(),
            last_run,
            fact_count,
            status: if cursor.is_some() {
                "ok".to_string()
            } else {
                "never run".to_string()
            },
        });
    }

    Ok(statuses)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    use crate::connect::{AuthPlan, InjectionStrategy, ResolvedCredential};
    use crate::mother::broker::sources::{Destination, SourceEntry};

    #[test]
    fn lake_route_fails_closed_without_oauth_credential() {
        let project = tempfile::tempdir().unwrap();
        let source = SourceEntry {
            name: "gh-main".to_string(),
            connection: "github".to_string(),
            params: HashMap::from([
                (
                    "owner".to_string(),
                    toml::Value::String("NicabarNimble".into()),
                ),
                ("repo".to_string(), toml::Value::String("patina".into())),
            ]),
            types: vec!["issues".into(), "prs".into()],
            schedule: "manual".to_string(),
            destination: Destination::Lake {
                name: "default".to_string(),
            },
        };
        let auth_plan = AuthPlan {
            credential: None,
            allowed_domains: vec!["api.github.com".to_string()],
        };

        let error = route_source_via_broker(&source, project.path(), &auth_plan).unwrap_err();
        assert!(
            error
                .to_string()
                .contains("connection 'github' has no credential"),
            "unexpected error: {error}"
        );
    }

    #[test]
    fn lake_route_fails_closed_without_github_domain_scope() {
        let project = tempfile::tempdir().unwrap();
        let source = SourceEntry {
            name: "gh-main".to_string(),
            connection: "github".to_string(),
            params: HashMap::from([
                (
                    "owner".to_string(),
                    toml::Value::String("NicabarNimble".into()),
                ),
                ("repo".to_string(), toml::Value::String("patina".into())),
            ]),
            types: vec!["issues".into(), "prs".into()],
            schedule: "manual".to_string(),
            destination: Destination::Lake {
                name: "default".to_string(),
            },
        };
        let auth_plan = AuthPlan {
            credential: Some(ResolvedCredential {
                value: "token".to_string(),
                injection: InjectionStrategy::Bearer,
            }),
            allowed_domains: vec!["example.com".to_string()],
        };

        let error = route_source_via_broker(&source, project.path(), &auth_plan).unwrap_err();
        assert!(
            error
                .to_string()
                .contains("connection 'github' is not allowed for api.github.com"),
            "unexpected error: {error}"
        );
    }

    #[test]
    fn run_source_fails_when_connection_missing() {
        let project = tempfile::tempdir().unwrap();
        let source = SourceEntry {
            name: "legacy-project-path".to_string(),
            connection: "github".to_string(),
            params: HashMap::new(),
            types: vec!["issues".into()],
            schedule: "manual".to_string(),
            destination: Destination::Project,
        };

        let error = run_source(&source, project.path(), true).unwrap_err();
        assert!(
            error
                .to_string()
                .contains("loading connection for source 'legacy-project-path'"),
            "unexpected error: {error}"
        );
    }
}
