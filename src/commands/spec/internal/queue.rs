use anyhow::Result;
use rusqlite::Connection;
use serde::Serialize;
use std::collections::HashMap;

use patina::spec::SpecStatus;

use super::db_path;
use super::queries::{get_all_specs, get_blocked_specs, ListFilters, SpecInfo};

#[derive(Debug, Serialize)]
pub(crate) struct Recommendation {
    pub id: String,
    pub status: String,
    pub reason: String,
    pub priority: u32,
    pub impact: usize,
    /// Queue position from `target` field. None = unqueued, sorts last.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub queue_position: Option<u32>,
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
        let status = match spec.status {
            Some(s) => s,
            None => continue,
        };
        let impact = dep_counts.get(&spec.id).copied().unwrap_or(0);
        let queue_position = parse_queue_position(spec.target.as_deref());

        match status {
            SpecStatus::Active => {
                recommendations.push(Recommendation {
                    id: spec.id.clone(),
                    status: status.to_string(),
                    reason: "Currently active — continue working".to_string(),
                    priority: 1,
                    impact,
                    queue_position,
                });
            }
            SpecStatus::Blocked => {
                // Check if blockers are now complete
                let spec_blocked = blocked_specs.iter().find(|b| b.id == spec.id);
                let all_blockers_done = spec_blocked
                    .map(|b| {
                        b.blocked_by.is_empty()
                            || b.blocked_by.iter().all(|bl| bl.status.is_terminal())
                    })
                    .unwrap_or(false);

                if all_blockers_done {
                    recommendations.push(Recommendation {
                        id: spec.id.clone(),
                        status: status.to_string(),
                        reason: "Blockers complete — ready to resume".to_string(),
                        priority: 2,
                        impact,
                        queue_position,
                    });
                }
            }
            SpecStatus::Paused => {
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
                    queue_position,
                });
            }
            SpecStatus::Ready => {
                let reason = match queue_position {
                    Some(pos) => format!("Queue position #{}", pos),
                    None => {
                        if impact > 0 {
                            format!("Ready — blocks {} other spec(s)", impact)
                        } else {
                            "Ready to start".to_string()
                        }
                    }
                };
                recommendations.push(Recommendation {
                    id: spec.id.clone(),
                    status: status.to_string(),
                    reason,
                    priority: 5,
                    impact,
                    queue_position,
                });
            }
            SpecStatus::Draft => {
                let reason = match queue_position {
                    Some(pos) => format!("Queue position #{} — needs audit", pos),
                    None => "Draft — unqueued".to_string(),
                };
                recommendations.push(Recommendation {
                    id: spec.id.clone(),
                    status: status.to_string(),
                    reason,
                    priority: 6,
                    impact,
                    queue_position,
                });
            }
            _ => {}
        }
    }

    // Sort: status tier (priority), then queue position (queued before unqueued,
    // lower position first), then impact as final tiebreaker.
    recommendations.sort_by(|a, b| {
        a.priority
            .cmp(&b.priority)
            .then_with(|| match (a.queue_position, b.queue_position) {
                (Some(pa), Some(pb)) => pa.cmp(&pb),
                (Some(_), None) => std::cmp::Ordering::Less,
                (None, Some(_)) => std::cmp::Ordering::Greater,
                (None, None) => std::cmp::Ordering::Equal,
            })
            .then_with(|| b.impact.cmp(&a.impact))
    });

    Ok(recommendations)
}

/// Parse `target` field as a queue position number.
///
/// Accepts bare integers ("1", "2") or any string — non-numeric targets
/// are ignored (returns None). This lets `target` serve as queue position
/// without breaking if someone sets a non-numeric value.
fn parse_queue_position(target: Option<&str>) -> Option<u32> {
    target.and_then(|t| t.trim().parse::<u32>().ok())
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
    let db_path = match db_path() {
        Ok(path) => path,
        Err(_) => return HashMap::new(),
    };
    if !db_path.exists() {
        return HashMap::new();
    }
    let conn = match Connection::open(&db_path) {
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
