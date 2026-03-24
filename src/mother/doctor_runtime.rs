use crate::environment::Environment;
use crate::eventlog;
use crate::project;
use crate::session::SessionManager;
use anyhow::{Context, Result};
use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;
use std::process::Command;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HealthStatus {
    Healthy,
    Warning,
    Critical,
}

#[derive(Serialize, Deserialize)]
struct HealthCheck {
    status: HealthStatus,
    environment_changes: EnvironmentChanges,
    project_config: ProjectStatus,
    data_integrity: DataIntegrity,
    recommendations: Vec<String>,
}

#[derive(Serialize, Deserialize, Default)]
struct DataIntegrity {
    events_db: EventsDbStatus,
    jsonl_replica: JsonlReplicaStatus,
    emission_coverage: EmissionCoverage,
    session_durability: SessionDurability,
}

#[derive(Serialize, Deserialize, Default)]
struct SessionDurability {
    uncommitted: bool,
    dirty_paths: Vec<String>,
}

#[derive(Serialize, Deserialize, Default)]
struct EmissionCoverage {
    types_checked: usize,
    types_with_data: usize,
    types_empty: Vec<String>,
}

#[derive(Serialize, Deserialize, Default)]
struct EventsDbStatus {
    exists: bool,
    integrity_ok: bool,
    event_count: i64,
    max_seq: i64,
    warnings: Vec<String>,
}

#[derive(Serialize, Deserialize, Default)]
struct JsonlReplicaStatus {
    exists: bool,
    max_seq: i64,
    gap: i64,
    warnings: Vec<String>,
}

#[derive(Serialize, Deserialize)]
struct EnvironmentChanges {
    missing_tools: Vec<ToolChange>,
    new_tools: Vec<ToolChange>,
    version_changes: Vec<ToolChange>,
}

#[derive(Serialize, Deserialize)]
struct ToolChange {
    name: String,
    old_version: Option<String>,
    new_version: Option<String>,
    required: bool,
}

#[derive(Serialize, Deserialize)]
struct ProjectStatus {
    llm: String,
    adapter_version: Option<String>,
    layer_patterns: usize,
    sessions: usize,
}

pub fn execute_value() -> Result<serde_json::Value> {
    let project_root = SessionManager::find_project_root()
        .context("Not in a Patina project directory. Run 'patina init' first.")?;

    let config = project::load_with_migration(&project_root)?;
    let current_env = Environment::detect()?;
    let stored_tools = config
        .environment
        .as_ref()
        .map(|e| e.detected_tools.clone())
        .unwrap_or_default();

    let mut health_check = analyze_environment(&current_env, &stored_tools)?;

    let llm = &config.adapters.default;
    let adapter = crate::interface::runtime::get_interface_provider(llm);
    let adapter_version = adapter
        .check_for_updates(&project_root)?
        .map(|(current, _)| current);

    let layer_path = project_root.join("layer");
    let pattern_count = count_patterns(&layer_path);
    let sessions_path = project_root.join("layer").join("sessions");
    let session_count = count_sessions(&sessions_path);

    health_check.project_config = ProjectStatus {
        llm: llm.to_string(),
        adapter_version,
        layer_patterns: pattern_count,
        sessions: session_count,
    };

    health_check.data_integrity = check_data_integrity(&mut health_check.recommendations);
    health_check.data_integrity.session_durability =
        check_session_durability(&mut health_check.recommendations);

    let has_data_warnings = !health_check.data_integrity.events_db.warnings.is_empty()
        || !health_check
            .data_integrity
            .jsonl_replica
            .warnings
            .is_empty();
    if has_data_warnings && health_check.status == HealthStatus::Healthy {
        health_check.status = HealthStatus::Warning;
    }
    if health_check.data_integrity.session_durability.uncommitted
        && health_check.status == HealthStatus::Healthy
    {
        health_check.status = HealthStatus::Warning;
    }
    if !health_check.data_integrity.events_db.integrity_ok
        && health_check.data_integrity.events_db.exists
    {
        health_check.status = HealthStatus::Critical;
    }

    let exit_code = match health_check.status {
        HealthStatus::Healthy => 0,
        HealthStatus::Warning => 2,
        HealthStatus::Critical => 3,
    };

    Ok(serde_json::json!({
        "health": health_check,
        "exit_code": exit_code,
    }))
}

fn analyze_environment(current: &Environment, stored_tools: &[String]) -> Result<HealthCheck> {
    let mut missing_tools = Vec::new();
    let mut new_tools = Vec::new();
    let version_changes = Vec::new();
    let mut recommendations = Vec::new();

    for tool_name in stored_tools {
        if !current
            .tools
            .get(tool_name)
            .is_some_and(|info| info.available)
        {
            let required = matches!(tool_name.as_str(), "cargo" | "rust" | "git");
            missing_tools.push(ToolChange {
                name: tool_name.clone(),
                old_version: Some("detected".to_string()),
                new_version: None,
                required,
            });

            if required {
                let install = match tool_name.as_str() {
                    "cargo" | "rust" => "curl https://sh.rustup.rs -sSf | sh",
                    "docker" => "Visit https://docker.com/get-started",
                    "git" => "brew install git (macOS) or apt install git (Linux)",
                    _ => "Check your package manager",
                };
                recommendations.push(format!("Install {tool_name}: {install}"));
            }
        }
    }

    for (name, info) in &current.tools {
        if info.available && !stored_tools.contains(name) {
            new_tools.push(ToolChange {
                name: name.clone(),
                old_version: None,
                new_version: info.version.clone(),
                required: false,
            });
        }
    }

    let status = if missing_tools.iter().any(|t| t.required) {
        HealthStatus::Critical
    } else if !missing_tools.is_empty() {
        HealthStatus::Warning
    } else {
        HealthStatus::Healthy
    };

    Ok(HealthCheck {
        status,
        environment_changes: EnvironmentChanges {
            missing_tools,
            new_tools,
            version_changes,
        },
        project_config: ProjectStatus {
            llm: String::new(),
            adapter_version: None,
            layer_patterns: 0,
            sessions: 0,
        },
        data_integrity: DataIntegrity::default(),
        recommendations,
    })
}

fn check_data_integrity(recommendations: &mut Vec<String>) -> DataIntegrity {
    let mut integrity = DataIntegrity::default();
    let events_path = Path::new(eventlog::EVENTS_DB);
    integrity.events_db.exists = events_path.exists();

    if !integrity.events_db.exists {
        integrity
            .events_db
            .warnings
            .push("events.db not found".to_string());
        recommendations.push("Run any command to initialize events.db".to_string());
    } else {
        match Connection::open(events_path) {
            Ok(conn) => {
                let quick_check: String = conn
                    .query_row("PRAGMA quick_check", [], |row| row.get(0))
                    .unwrap_or_else(|_| "error".to_string());
                integrity.events_db.integrity_ok = quick_check == "ok";
                if !integrity.events_db.integrity_ok {
                    integrity
                        .events_db
                        .warnings
                        .push(format!("PRAGMA quick_check: {}", quick_check));
                    recommendations.push(
                        "events.db may be corrupt. Import from JSONL: `patina events import layer/events.jsonl`".to_string(),
                    );
                }

                integrity.events_db.event_count = conn
                    .query_row("SELECT COUNT(*) FROM eventlog", [], |row| row.get(0))
                    .unwrap_or(0);
                integrity.events_db.max_seq = conn
                    .query_row("SELECT COALESCE(MAX(seq), 0) FROM eventlog", [], |row| {
                        row.get(0)
                    })
                    .unwrap_or(0);
            }
            Err(e) => {
                integrity
                    .events_db
                    .warnings
                    .push(format!("failed to open events.db: {e}"));
            }
        }
    }

    if integrity.events_db.exists && integrity.events_db.integrity_ok {
        integrity.emission_coverage = check_emission_coverage(recommendations);
    }

    let jsonl_path = Path::new("layer/events.jsonl");
    integrity.jsonl_replica.exists = jsonl_path.exists();
    if !integrity.jsonl_replica.exists {
        integrity
            .jsonl_replica
            .warnings
            .push("layer/events.jsonl not found — no durability replica".to_string());
        recommendations.push("Run `patina events export` to create JSONL replica".to_string());
    }

    integrity
}

fn check_session_durability(recommendations: &mut Vec<String>) -> SessionDurability {
    if !crate::git::is_git_repo().unwrap_or(false) {
        return SessionDurability::default();
    }
    let output = match Command::new("git")
        .args([
            "status",
            "--porcelain",
            "--",
            "layer/sessions",
            "layer/events.jsonl",
        ])
        .output()
    {
        Ok(output) if output.status.success() => output,
        _ => return SessionDurability::default(),
    };

    let dirty_paths: Vec<String> = String::from_utf8_lossy(&output.stdout)
        .lines()
        .filter_map(parse_porcelain_path)
        .collect();

    if !dirty_paths.is_empty() {
        recommendations.push(
            "Session artifacts are not fully committed. Run `git add layer/sessions layer/events.jsonl && git commit -m \"session: preserve artifacts\"` to preserve recoverability.".to_string(),
        );
    }

    SessionDurability {
        uncommitted: !dirty_paths.is_empty(),
        dirty_paths,
    }
}

const CORE_EVENT_TYPES: &[&str] = &[
    "measure.capture",
    "measure.search",
    "measure.index",
    "measure.believe",
    "measure.evolve",
    "scry.query",
    "scry.use",
    "scry.feedback",
    "context.query",
    "assay.query",
];

fn check_emission_coverage(recommendations: &mut Vec<String>) -> EmissionCoverage {
    let mut event_types: Vec<String> = CORE_EVENT_TYPES.iter().map(|s| s.to_string()).collect();
    event_types.extend(load_schema_event_types());
    event_types.sort();
    event_types.dedup();

    let mut coverage = EmissionCoverage {
        types_checked: event_types.len(),
        ..Default::default()
    };
    let conn = match Connection::open(eventlog::EVENTS_DB) {
        Ok(c) => c,
        Err(_) => return coverage,
    };

    for event_type in &event_types {
        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM eventlog WHERE event_type = ?1",
                [event_type.as_str()],
                |row| row.get(0),
            )
            .unwrap_or(0);
        if count > 0 {
            coverage.types_with_data += 1;
        } else {
            coverage.types_empty.push(event_type.to_string());
        }
    }

    if !coverage.types_empty.is_empty() {
        recommendations.push(format!(
            "No events for: {}. Run the corresponding commands to populate.",
            coverage.types_empty.join(", ")
        ));
    }

    coverage
}

fn load_schema_event_types() -> Vec<String> {
    let mut out = Vec::new();
    let project_root = match SessionManager::find_project_root() {
        Ok(root) => root,
        Err(_) => return out,
    };
    let schemas_dir = crate::paths::project::schemas_dir(&project_root);
    let entries = match std::fs::read_dir(&schemas_dir) {
        Ok(entries) => entries,
        Err(_) => return out,
    };

    for entry in entries.flatten() {
        let schema_path = entry.path().join("schema.toml");
        let content = match std::fs::read_to_string(schema_path) {
            Ok(c) => c,
            Err(_) => continue,
        };
        let parsed: toml::Value = match toml::from_str(&content) {
            Ok(v) => v,
            Err(_) => continue,
        };
        if let Some(facts) = parsed.get("facts").and_then(|v| v.as_array()) {
            for fact in facts {
                if let Some(event_type) = fact.get("event_type").and_then(|v| v.as_str()) {
                    out.push(event_type.to_string());
                }
            }
        }
    }

    out
}

fn count_patterns(layer_path: &Path) -> usize {
    let mut count = 0;
    if layer_path.exists() {
        for dir in ["core", "topics", "projects"] {
            let path = layer_path.join(dir);
            if let Ok(entries) = fs::read_dir(path) {
                count += entries
                    .filter_map(Result::ok)
                    .filter(|e| e.path().extension().is_some_and(|ext| ext == "md"))
                    .count();
            }
        }
    }
    count
}

fn count_sessions(sessions_path: &Path) -> usize {
    if let Ok(entries) = fs::read_dir(sessions_path) {
        entries
            .filter_map(Result::ok)
            .filter(|e| e.path().extension().is_some_and(|ext| ext == "md"))
            .count()
    } else {
        0
    }
}

fn parse_porcelain_path(line: &str) -> Option<String> {
    if line.len() < 4 {
        return None;
    }
    let raw = line[3..].trim();
    if raw.is_empty() {
        return None;
    }
    if let Some((_, to)) = raw.split_once(" -> ") {
        return Some(to.to_string());
    }
    Some(raw.to_string())
}
