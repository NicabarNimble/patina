use anyhow::Result;
use rusqlite::Connection;
use serde::Serialize;
use std::collections::HashMap;
use std::path::Path;

use super::queries::{get_all_specs, get_blocked_specs, ListFilters, SpecInfo};
use super::DB_PATH;

#[derive(Debug, Serialize)]
pub(crate) struct Recommendation {
    pub id: String,
    pub status: String,
    pub reason: String,
    pub priority: u32,
    pub impact: usize,
}

/// Recommend the next spec to work on based on priority ranking.
///
/// Ranking: active > blocked-ready-to-resume > paused-with-age > impact > drafts
pub fn next_spec(json: bool) -> Result<()> {
    let recommendations = next_spec_value()?;

    if json {
        println!("{}", serde_json::to_string_pretty(&recommendations)?);
        return Ok(());
    }

    if recommendations.is_empty() {
        println!("No actionable specs found.");
        return Ok(());
    }

    // Top recommendation
    let top = &recommendations[0];
    println!("RECOMMENDED: {}", top.id);
    println!("  Status: {}", top.status);
    println!("  Reason: {}", top.reason);
    if top.impact > 0 {
        println!("  Impact: blocks {} other spec(s)", top.impact);
    }

    // Show the rest as alternatives
    if recommendations.len() > 1 {
        println!("\nOther specs:");
        for rec in &recommendations[1..] {
            let impact_str = if rec.impact > 0 {
                format!(" (blocks {})", rec.impact)
            } else {
                String::new()
            };
            println!(
                "  {:<28} {:<10} {}{}",
                rec.id, rec.status, rec.reason, impact_str
            );
        }
    }

    Ok(())
}

/// Recommend the next spec and return structured result (for MCP).
pub fn next_spec_value() -> Result<Vec<Recommendation>> {
    let all_specs = get_all_specs(&ListFilters::default())?;

    if all_specs.is_empty() {
        return Ok(Vec::new());
    }

    // Load blocked specs to check for unblocked ones
    let blocked_specs = get_blocked_specs().unwrap_or_default();

    // Load spec_deps for impact scoring
    let dep_counts = load_dep_counts();

    let mut recommendations: Vec<Recommendation> = Vec::new();

    for spec in &all_specs {
        let status = spec.status.as_deref().unwrap_or("unknown");
        let impact = dep_counts.get(&spec.id).copied().unwrap_or(0);

        match status {
            "active" => {
                recommendations.push(Recommendation {
                    id: spec.id.clone(),
                    status: status.to_string(),
                    reason: "Currently active — continue working".to_string(),
                    priority: 1,
                    impact,
                });
            }
            "blocked" => {
                // Check if blockers are now complete
                let spec_blocked = blocked_specs.iter().find(|b| b.id == spec.id);
                let all_blockers_done = spec_blocked
                    .map(|b| {
                        b.blocked_by.is_empty()
                            || b.blocked_by
                                .iter()
                                .all(|bl| bl.status == "complete" || bl.status == "done")
                    })
                    .unwrap_or(false);

                if all_blockers_done {
                    recommendations.push(Recommendation {
                        id: spec.id.clone(),
                        status: status.to_string(),
                        reason: "Blockers complete — ready to resume".to_string(),
                        priority: 2,
                        impact,
                    });
                }
            }
            "paused" => {
                let age = spec_age_days_from_list(spec);
                let reason = if age > 14 {
                    format!("Paused {} days — overdue for attention", age)
                } else {
                    format!("Paused {} days", age)
                };
                recommendations.push(Recommendation {
                    id: spec.id.clone(),
                    status: status.to_string(),
                    reason,
                    priority: if age > 14 { 3 } else { 4 },
                    impact,
                });
            }
            "ready" => {
                recommendations.push(Recommendation {
                    id: spec.id.clone(),
                    status: status.to_string(),
                    reason: if impact > 0 {
                        format!("Ready to start — blocks {} other spec(s)", impact)
                    } else {
                        "Ready to start".to_string()
                    },
                    priority: 5,
                    impact,
                });
            }
            "draft" => {
                recommendations.push(Recommendation {
                    id: spec.id.clone(),
                    status: status.to_string(),
                    reason: "Draft — promote to ready when prepared".to_string(),
                    priority: 6,
                    impact,
                });
            }
            _ => {}
        }
    }

    // Sort by priority (ascending), then by impact (descending)
    recommendations.sort_by(|a, b| a.priority.cmp(&b.priority).then(b.impact.cmp(&a.impact)));

    Ok(recommendations)
}

/// Compute age in days for a spec from the all-specs list.
/// Reads paused_date/blocked_date from SpecInfo (parsed during scan_disk_specs).
pub fn spec_age_days_from_list(spec: &SpecInfo) -> i64 {
    let date_str = spec.paused_date.as_deref().or(spec.blocked_date.as_deref());
    if let Some(date_str) = date_str {
        if let Ok(date) = chrono::NaiveDate::parse_from_str(date_str, "%Y-%m-%d") {
            let today = chrono::Utc::now().date_naive();
            return (today - date).num_days();
        }
    }
    0
}

/// Load dependency counts from spec_deps: how many specs depend on each spec.
pub fn load_dep_counts() -> HashMap<String, usize> {
    let db_path = Path::new(DB_PATH);
    if !db_path.exists() {
        return HashMap::new();
    }
    let conn = match Connection::open(db_path) {
        Ok(c) => c,
        Err(_) => return HashMap::new(),
    };
    let mut stmt =
        match conn.prepare("SELECT depends_on, COUNT(*) FROM spec_deps GROUP BY depends_on") {
            Ok(s) => s,
            Err(_) => return HashMap::new(),
        };
    stmt.query_map([], |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, usize>(1)?))
    })
    .ok()
    .map(|rows| rows.flatten().collect())
    .unwrap_or_default()
}

/// Check if a git tag exists (delegates to patina::git::tag_exists)
pub fn tag_exists(tag: &str) -> Result<bool> {
    patina::git::tag_exists(tag)
}

/// Check if working tree is clean for tracked files (delegates to patina::git::is_clean_tracked)
pub fn is_tree_clean() -> Result<bool> {
    patina::git::is_clean_tracked()
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_tag_name_format() {
        let id = "session-092-hardening";
        let tag = format!("spec/{}", id);
        assert_eq!(tag, "spec/session-092-hardening");
    }
}
