use anyhow::{Context, Result};
use patina::environment::Environment;
use patina::eventlog;
use patina::project;
use patina::session::SessionManager;
use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

#[derive(Serialize, Deserialize)]
struct HealthCheck {
    status: String, // "healthy", "warning", "critical"
    environment_changes: EnvironmentChanges,
    project_config: ProjectStatus,
    data_integrity: DataIntegrity,
    recommendations: Vec<String>,
}

#[derive(Serialize, Deserialize, Default)]
struct DataIntegrity {
    events_db: EventsDbStatus,
    jsonl_replica: JsonlReplicaStatus,
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

pub fn execute(json_output: bool) -> Result<i32> {
    // Find project root first
    let project_root = SessionManager::find_project_root()
        .context("Not in a Patina project directory. Run 'patina init' first.")?;

    let _non_interactive = json_output || std::env::var("PATINA_NONINTERACTIVE").is_ok();

    if !json_output {
        println!("🏥 Checking project health...");
    }

    // Load unified project config (with migration if needed)
    let config = project::load_with_migration(&project_root)?;

    // Get current environment
    let current_env = Environment::detect()?;

    // Get stored environment snapshot
    let stored_tools = config
        .environment
        .as_ref()
        .map(|e| e.detected_tools.clone())
        .unwrap_or_default();

    // Compare environments
    let mut health_check = analyze_environment(&current_env, &stored_tools)?;

    // Check project status - use adapters.default as the LLM
    let llm = &config.adapters.default;
    let adapter = patina::adapters::get_adapter(llm);
    let adapter_version = adapter
        .check_for_updates(&project_root)?
        .map(|(current, _)| current);

    // Count layer patterns
    let layer_path = project_root.join("layer");
    let pattern_count = count_patterns(&layer_path);

    // Count sessions from canonical location (layer/sessions/)
    let sessions_path = project_root.join("layer").join("sessions");
    let session_count = count_sessions(&sessions_path);

    health_check.project_config = ProjectStatus {
        llm: llm.to_string(),
        adapter_version,
        layer_patterns: pattern_count,
        sessions: session_count,
    };

    // Check data integrity (events.db + JSONL replica)
    health_check.data_integrity = check_data_integrity(&mut health_check.recommendations);

    // Escalate status if data integrity has warnings
    let has_data_warnings = !health_check.data_integrity.events_db.warnings.is_empty()
        || !health_check
            .data_integrity
            .jsonl_replica
            .warnings
            .is_empty();
    if has_data_warnings && health_check.status == "healthy" {
        health_check.status = "warning".to_string();
    }
    if !health_check.data_integrity.events_db.integrity_ok
        && health_check.data_integrity.events_db.exists
    {
        health_check.status = "critical".to_string();
    }

    // Display results
    if json_output {
        println!("{}", serde_json::to_string_pretty(&health_check)?);
    } else {
        display_health_check(&health_check, &current_env, &project_root)?;

        // Only provide recommendations, no auto-fixing
        if !health_check.environment_changes.missing_tools.is_empty()
            && !json_output
            && !health_check.recommendations.is_empty()
        {
            println!("\n💡 Run 'patina init .' to refresh your environment snapshot");
        }
    }

    // Determine exit code
    let exit_code = match health_check.status.as_str() {
        "healthy" => 0,
        "warning" => 2,
        "critical" => 3,
        _ => 1,
    };

    Ok(exit_code)
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
        "critical".to_string()
    } else if !missing_tools.is_empty() {
        "warning".to_string()
    } else {
        "healthy".to_string()
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

fn display_health_check(
    health: &HealthCheck,
    _env: &Environment,
    project_root: &std::path::Path,
) -> Result<()> {
    println!("\nEnvironment Changes Since Init:");

    // Display missing tools
    for tool in &health.environment_changes.missing_tools {
        let marker = if tool.required { "⚠️ " } else { "  " };
        let old_version = tool.old_version.as_deref().unwrap_or("unknown");
        let required_msg = if tool.required { " (required!)" } else { "" };
        println!(
            "  {marker} {}: {old_version} → NOT FOUND{required_msg}",
            tool.name
        );
    }

    // Display new tools
    for tool in &health.environment_changes.new_tools {
        let version = tool.new_version.as_deref().unwrap_or("detected");
        println!("  ✓ New tool: {} {version}", tool.name);
    }

    println!("\nProject Configuration:");
    // Display UID
    if let Some(uid) = project::get_uid(project_root) {
        println!("  ✓ UID: {}", uid);
    } else {
        println!("  ⚠ UID: missing (will be created on next scrape)");
    }
    let adapter_version = health
        .project_config
        .adapter_version
        .as_deref()
        .unwrap_or("unknown");
    println!(
        "  ✓ LLM: {} (adapter {adapter_version})",
        health.project_config.llm
    );
    println!(
        "  ✓ Layer: {} patterns stored",
        health.project_config.layer_patterns
    );
    println!("  ✓ Sessions: {} recorded", health.project_config.sessions);

    // Data integrity section
    println!("\nData Integrity:");
    let db = &health.data_integrity.events_db;
    if !db.exists {
        println!("  ⚠ events.db: not found");
    } else if !db.integrity_ok {
        println!("  ⚠ events.db: INTEGRITY CHECK FAILED");
    } else {
        println!(
            "  ✓ events.db: {} events, max seq {}, integrity ok",
            db.event_count, db.max_seq
        );
    }

    let jsonl = &health.data_integrity.jsonl_replica;
    if !jsonl.exists {
        println!("  ⚠ JSONL replica: not found");
    } else if jsonl.gap > 0 {
        println!(
            "  ⚠ JSONL replica: {} events behind (max seq {})",
            jsonl.gap, jsonl.max_seq
        );
    } else {
        println!("  ✓ JSONL replica: up to date (max seq {})", jsonl.max_seq);
    }

    if !health.recommendations.is_empty() {
        println!("\nRecommendations:");
        for (i, rec) in health.recommendations.iter().enumerate() {
            println!("  {}. {rec}", i + 1);
        }
    }

    Ok(())
}
