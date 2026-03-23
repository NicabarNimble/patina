use anyhow::{Context, Result};
use patina::environment::Environment;
use patina::eventlog;
use patina::project;
use patina::session::SessionManager;
use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use std::process::Command;

/// Typed health status — the doctor check outcome.
///
/// Follows [[enum-not-string-for-finite-states]]: 3 variants for project health.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HealthStatus {
    Healthy,
    Warning,
    Critical,
}

impl HealthStatus {
    /// Canonical string form.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Healthy => "healthy",
            Self::Warning => "warning",
            Self::Critical => "critical",
        }
    }
}

impl std::fmt::Display for HealthStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}
use std::fs;
use std::path::Path;

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

pub(crate) fn execute_value() -> Result<serde_json::Value> {
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
    let adapter = patina::interface::runtime::get_interface_provider(llm);
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

pub fn execute_cli(json_output: bool) -> Result<()> {
    let payload = serde_json::json!({ "json": json_output });
    let client = patina::mother::Client::new("localhost:50051".to_string());
    let response = client
        .child_action("doctor", "run", &payload)
        .map_err(|e| {
            anyhow::anyhow!(
                "doctor child unavailable via Mother (start with `patina mother start`): {}",
                e
            )
        })?;

    if json_output {
        if let Some(data) = response.get("data") {
            println!("{}", serde_json::to_string_pretty(data)?);
        } else {
            println!("{}", serde_json::to_string_pretty(&response)?);
        }
        if let Some(code) = response.get("exit_code").and_then(|v| v.as_i64()) {
            if code != 0 {
                std::process::exit(code as i32);
            }
        }
        return Ok(());
    }

    if let Some(text) = response.get("text").and_then(|v| v.as_str()) {
        if !text.is_empty() {
            println!("{}", text);
        }
    }
    if let Some(data) = response.get("data") {
        let status = data
            .get("health")
            .and_then(|h| h.get("status"))
            .and_then(|s| s.as_str())
            .unwrap_or("unknown");
        println!("Doctor status: {}", status);
    }
    if let Some(code) = response.get("exit_code").and_then(|v| v.as_i64()) {
        if code != 0 {
            std::process::exit(code as i32);
        }
    }
    Ok(())
}

fn analyze_environment(current: &Environment, stored_tools: &[String]) -> Result<HealthCheck> {
    let mut missing_tools = Vec::new();
    let mut new_tools = Vec::new();
    let version_changes = Vec::new();
    let mut recommendations = Vec::new();

    // Check for missing tools
    for tool_name in stored_tools {
        if !current
            .tools
            .get(tool_name)
            .is_some_and(|info| info.available)
        {
            let required = is_tool_required(tool_name);
            missing_tools.push(ToolChange {
                name: tool_name.clone(),
                old_version: Some("detected".to_string()),
                new_version: None,
                required,
            });

            if required {
                recommendations.push(format!(
                    "Install {tool_name}: {}",
                    get_install_command(tool_name)
                ));
            }
        }
    }

    // Check for new tools
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

    // Determine overall status
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

fn is_tool_required(tool: &str) -> bool {
    // Core tools required for Patina projects
    // Docker is optional (detected but not required)
    matches!(tool, "cargo" | "rust" | "git")
}

fn get_install_command(tool: &str) -> &'static str {
    match tool {
        "cargo" | "rust" => "curl https://sh.rustup.rs -sSf | sh",
        "docker" => "Visit https://docker.com/get-started",
        "git" => "brew install git (macOS) or apt install git (Linux)",
        _ => "Check your package manager",
    }
}

/// Check events.db integrity and JSONL replica staleness.
fn check_data_integrity(recommendations: &mut Vec<String>) -> DataIntegrity {
    let mut integrity = DataIntegrity::default();

    // --- events.db checks ---
    let events_path = Path::new(eventlog::EVENTS_DB);
    integrity.events_db.exists = events_path.exists();

    if !integrity.events_db.exists {
        integrity
            .events_db
            .warnings
            .push("events.db not found".to_string());
        recommendations.push("Run any command to initialize events.db".to_string());
    } else {
        // PRAGMA quick_check (fast, sufficient for routine checks)
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

                // Row count + max seq
                integrity.events_db.event_count = conn
                    .query_row("SELECT COUNT(*) FROM eventlog", [], |row| row.get(0))
                    .unwrap_or(0);
                integrity.events_db.max_seq = conn
                    .query_row("SELECT COALESCE(MAX(seq), 0) FROM eventlog", [], |row| {
                        row.get(0)
                    })
                    .unwrap_or(0);

                if integrity.events_db.event_count == 0 {
                    integrity
                        .events_db
                        .warnings
                        .push("events.db is empty (0 events)".to_string());
                }
            }
            Err(e) => {
                integrity
                    .events_db
                    .warnings
                    .push(format!("failed to open events.db: {e}"));
            }
        }
    }

    // --- Emission coverage checks ---
    if integrity.events_db.exists && integrity.events_db.integrity_ok {
        integrity.emission_coverage = check_emission_coverage(recommendations);
    }

    // --- JSONL replica checks ---
    let jsonl_path = Path::new("layer/events.jsonl");
    integrity.jsonl_replica.exists = jsonl_path.exists();

    if !integrity.jsonl_replica.exists {
        integrity
            .jsonl_replica
            .warnings
            .push("layer/events.jsonl not found — no durability replica".to_string());
        recommendations.push("Run `patina events export` to create JSONL replica".to_string());
    } else {
        // Read last line to get max seq
        integrity.jsonl_replica.max_seq = read_jsonl_max_seq(jsonl_path);

        // Compare with events.db max seq
        if integrity.events_db.exists && integrity.events_db.max_seq > 0 {
            integrity.jsonl_replica.gap =
                integrity.events_db.max_seq - integrity.jsonl_replica.max_seq;
            if integrity.jsonl_replica.gap > 0 {
                integrity.jsonl_replica.warnings.push(format!(
                    "JSONL is {} events behind (db max seq: {}, JSONL max seq: {})",
                    integrity.jsonl_replica.gap,
                    integrity.events_db.max_seq,
                    integrity.jsonl_replica.max_seq
                ));
                recommendations
                    .push("Run `patina events export` to sync JSONL replica".to_string());
            }
        }
    }

    integrity
}

fn check_session_durability(recommendations: &mut Vec<String>) -> SessionDurability {
    if !patina::git::is_git_repo().unwrap_or(false) {
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

    if dirty_paths.is_empty() {
        return SessionDurability {
            uncommitted: false,
            dirty_paths,
        };
    }

    recommendations.push(
        "Session artifacts are not fully committed. Run `git add layer/sessions layer/events.jsonl && git commit -m \"session: preserve artifacts\"` to preserve recoverability."
            .to_string(),
    );

    SessionDurability {
        uncommitted: true,
        dirty_paths,
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

/// Non-schema event types that should have emitters wired in code.
/// Schema-driven event types (github.issue, github.pr, etc.) are
/// resolved at runtime from installed schemas.
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

/// Build the full list of active event types: core (hardcoded) + schema-driven.
///
/// Schema event types are loaded from installed schemas at runtime, so new
/// connectors automatically appear in doctor checks without code changes.
fn build_active_event_types() -> Vec<String> {
    let mut types: Vec<String> = CORE_EVENT_TYPES.iter().map(|s| s.to_string()).collect();

    // Add event types from installed schemas
    if let Ok(schemas) = crate::commands::schema::load_all_installed() {
        for schema in &schemas {
            for fact in &schema.facts {
                if !types.contains(&fact.event_type) {
                    types.push(fact.event_type.clone());
                }
            }
        }
    }

    types
}

/// Check emission coverage: which registered event types have data in events.db.
fn check_emission_coverage(recommendations: &mut Vec<String>) -> EmissionCoverage {
    let active_types = build_active_event_types();

    let mut coverage = EmissionCoverage {
        types_checked: active_types.len(),
        ..Default::default()
    };

    let conn = match Connection::open(eventlog::EVENTS_DB) {
        Ok(c) => c,
        Err(_) => return coverage,
    };

    for event_type in &active_types {
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

/// Read the max seq from a JSONL file by scanning the last non-empty line.
fn read_jsonl_max_seq(path: &Path) -> i64 {
    let content = match fs::read_to_string(path) {
        Ok(c) => c,
        Err(_) => return 0,
    };

    // Find last non-empty line
    for line in content.lines().rev() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        // Parse seq from JSON line
        if let Ok(value) = serde_json::from_str::<serde_json::Value>(line) {
            if let Some(seq) = value.get("seq").and_then(|v| v.as_i64()) {
                return seq;
            }
        }
    }
    0
}

fn count_patterns(layer_path: &std::path::Path) -> usize {
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

fn count_sessions(sessions_path: &std::path::Path) -> usize {
    if let Ok(entries) = fs::read_dir(sessions_path) {
        entries
            .filter_map(Result::ok)
            .filter(|e| e.path().extension().is_some_and(|ext| ext == "md"))
            .count()
    } else {
        0
    }
}

#[cfg(test)]
mod tests {
    use super::parse_porcelain_path;

    #[test]
    fn parse_porcelain_path_handles_simple_entries() {
        assert_eq!(
            parse_porcelain_path(" M layer/sessions/20260316-abc.md").as_deref(),
            Some("layer/sessions/20260316-abc.md")
        );
    }

    #[test]
    fn parse_porcelain_path_handles_rename_entries() {
        assert_eq!(
            parse_porcelain_path("R  old/path.md -> layer/sessions/new.md").as_deref(),
            Some("layer/sessions/new.md")
        );
    }
}
