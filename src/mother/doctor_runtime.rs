use crate::environment::Environment;
use crate::eventlog;
use crate::project;
use crate::session::SessionManager;
use anyhow::{Context, Result};
use patina_core::doctor::{
    DataIntegrity, DoctorRunResult, EmissionCoverage, EnvironmentPort, EventStorePort,
    HealthStatus, ProjectRepoPort, ProjectSnapshot, SessionDurability, ToolSnapshot,
};
use rusqlite::Connection;
use std::fs;
use std::path::Path;
use std::process::Command;

pub fn execute_value() -> Result<serde_json::Value> {
    let project_root = SessionManager::find_project_root()
        .context("Not in a Patina project directory. Run 'patina init' first.")?;
    let environment_port = FsEnvironmentPort;
    let project_port = FsProjectRepoPort {
        project_root: project_root.clone(),
    };
    let event_store_port = FsEventStorePort { project_root };

    let result =
        patina_core::doctor::run_doctor(&environment_port, &project_port, &event_store_port)
            .map_err(|err| anyhow::anyhow!(err.to_string()))?;

    Ok(doctor_result_to_value(&result))
}

fn doctor_result_to_value(result: &DoctorRunResult) -> serde_json::Value {
    let status = match result.health.status {
        HealthStatus::Healthy => "healthy",
        HealthStatus::Warning => "warning",
        HealthStatus::Critical => "critical",
    };
    serde_json::json!({
        "health": {
            "status": status,
            "environment_changes": {
                "missing_tools": result.health.environment_changes.missing_tools.iter().map(|tool| serde_json::json!({
                    "name": tool.name,
                    "old_version": tool.old_version,
                    "new_version": tool.new_version,
                    "required": tool.required,
                })).collect::<Vec<_>>(),
                "new_tools": result.health.environment_changes.new_tools.iter().map(|tool| serde_json::json!({
                    "name": tool.name,
                    "old_version": tool.old_version,
                    "new_version": tool.new_version,
                    "required": tool.required,
                })).collect::<Vec<_>>(),
                "version_changes": result.health.environment_changes.version_changes.iter().map(|tool| serde_json::json!({
                    "name": tool.name,
                    "old_version": tool.old_version,
                    "new_version": tool.new_version,
                    "required": tool.required,
                })).collect::<Vec<_>>(),
            },
            "project_config": {
                "llm": result.health.project_config.llm,
                "adapter_version": result.health.project_config.adapter_version,
                "layer_patterns": result.health.project_config.layer_patterns,
                "sessions": result.health.project_config.sessions,
            },
            "data_integrity": {
                "events_db": {
                    "exists": result.health.data_integrity.events_db.exists,
                    "integrity_ok": result.health.data_integrity.events_db.integrity_ok,
                    "event_count": result.health.data_integrity.events_db.event_count,
                    "max_seq": result.health.data_integrity.events_db.max_seq,
                    "warnings": result.health.data_integrity.events_db.warnings,
                },
                "jsonl_replica": {
                    "exists": result.health.data_integrity.jsonl_replica.exists,
                    "max_seq": result.health.data_integrity.jsonl_replica.max_seq,
                    "gap": result.health.data_integrity.jsonl_replica.gap,
                    "warnings": result.health.data_integrity.jsonl_replica.warnings,
                },
                "emission_coverage": {
                    "types_checked": result.health.data_integrity.emission_coverage.types_checked,
                    "types_with_data": result.health.data_integrity.emission_coverage.types_with_data,
                    "types_empty": result.health.data_integrity.emission_coverage.types_empty,
                },
                "session_durability": {
                    "uncommitted": result.health.data_integrity.session_durability.uncommitted,
                    "dirty_paths": result.health.data_integrity.session_durability.dirty_paths,
                },
            },
            "recommendations": result.health.recommendations,
        },
        "exit_code": result.exit_code,
    })
}

struct FsEnvironmentPort;

impl EnvironmentPort for FsEnvironmentPort {
    fn detect_tools(&self) -> std::result::Result<Vec<ToolSnapshot>, String> {
        let current = Environment::detect().map_err(|e| e.to_string())?;
        Ok(current
            .tools
            .into_iter()
            .map(|(name, info)| ToolSnapshot {
                name,
                available: info.available,
                version: info.version,
            })
            .collect())
    }
}

struct FsProjectRepoPort {
    project_root: std::path::PathBuf,
}

impl ProjectRepoPort for FsProjectRepoPort {
    fn snapshot(&self) -> std::result::Result<ProjectSnapshot, String> {
        let config = project::load_with_migration(&self.project_root).map_err(|e| e.to_string())?;
        let llm = config.adapters.default.clone();
        let adapter = crate::interface::runtime::get_interface_provider(&llm);
        let adapter_version = adapter
            .check_for_updates(&self.project_root)
            .map_err(|e| e.to_string())?
            .map(|(current, _)| current);
        let stored_tools = config
            .environment
            .as_ref()
            .map(|e| e.detected_tools.clone())
            .unwrap_or_default();

        Ok(ProjectSnapshot {
            llm,
            adapter_version,
            layer_patterns: count_patterns(&self.project_root.join("layer")),
            sessions: count_sessions(&self.project_root.join("layer").join("sessions")),
            stored_tools,
        })
    }
}

struct FsEventStorePort {
    project_root: std::path::PathBuf,
}

impl EventStorePort for FsEventStorePort {
    fn data_integrity(&self) -> std::result::Result<DataIntegrity, String> {
        Ok(check_data_integrity(&self.project_root))
    }

    fn data_recommendations(&self) -> std::result::Result<Vec<String>, String> {
        Ok(data_recommendations(&check_data_integrity(
            &self.project_root,
        )))
    }

    fn session_recommendations(&self) -> std::result::Result<Vec<String>, String> {
        Ok(session_recommendations(&check_session_durability()))
    }
}

fn check_data_integrity(project_root: &Path) -> DataIntegrity {
    let mut integrity = DataIntegrity::default();
    let events_path = eventlog::resolve_events_db_path(project_root);
    integrity.events_db.exists = events_path.exists();

    if !integrity.events_db.exists {
        integrity
            .events_db
            .warnings
            .push("events.db not found".to_string());
    } else {
        match Connection::open(&events_path) {
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
        integrity.emission_coverage = check_emission_coverage(project_root);
    }

    let jsonl_path = Path::new("layer/events.jsonl");
    integrity.jsonl_replica.exists = jsonl_path.exists();
    if !integrity.jsonl_replica.exists {
        integrity
            .jsonl_replica
            .warnings
            .push("layer/events.jsonl not found — no durability replica".to_string());
    }

    integrity
}

fn check_session_durability() -> SessionDurability {
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

fn check_emission_coverage(project_root: &Path) -> EmissionCoverage {
    let mut event_types: Vec<String> = CORE_EVENT_TYPES.iter().map(|s| s.to_string()).collect();
    event_types.extend(load_schema_event_types(project_root));
    event_types.sort();
    event_types.dedup();

    let mut coverage = EmissionCoverage {
        types_checked: event_types.len(),
        ..Default::default()
    };
    let events_path = eventlog::resolve_events_db_path(project_root);
    let conn = match Connection::open(events_path) {
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

    coverage
}

fn load_schema_event_types(project_root: &Path) -> Vec<String> {
    let mut out = Vec::new();
    let schemas_dir = crate::paths::project::schemas_dir(project_root);
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

fn data_recommendations(data_integrity: &DataIntegrity) -> Vec<String> {
    let mut recommendations = Vec::new();
    if !data_integrity.events_db.exists {
        recommendations.push("Run any command to initialize events.db".to_string());
    }
    if data_integrity.events_db.exists && !data_integrity.events_db.integrity_ok {
        recommendations.push(
            "events.db may be corrupt. Import from JSONL: `patina events import layer/events.jsonl`"
                .to_string(),
        );
    }
    if !data_integrity.jsonl_replica.exists {
        recommendations.push("Run `patina events export` to create JSONL replica".to_string());
    }
    recommendations
}

fn session_recommendations(session_durability: &SessionDurability) -> Vec<String> {
    if session_durability.uncommitted {
        return vec![
            "Session artifacts are not fully committed. Run `git add layer/sessions layer/events.jsonl && git commit -m \"session: preserve artifacts\"` to preserve recoverability.".to_string(),
        ];
    }
    Vec::new()
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
