//! Doctor command plugin — project health checks.
//!
//! Extracted from src/commands/doctor.rs into a WASM command plugin.
//! Uses patina:host/layer for read-only project data access instead
//! of direct library calls.
//!
//! Proves the command world: CLI subcommand that runs without Mother daemon.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use patina_sdk::command::{layer, measure, query};
use patina_sdk::{register_command, CommandPlugin};

/// Typed structs for doctor plugin.
///
/// Environment/ToolInfo: deserialized at the JSON boundary (EC3).
/// HealthCheck/ToolChange: serialized for --json output (EC2).
/// Wrapper structs preserve the nested JSON shape.
mod types {
    use super::*;

    // -- Input boundary: environment JSON → typed structs (EC3) --

    #[derive(Deserialize)]
    pub struct Environment {
        #[serde(default)]
        pub tools: HashMap<String, ToolInfo>,
    }

    #[derive(Deserialize)]
    pub struct ToolInfo {
        #[serde(default)]
        pub available: bool,
        pub version: Option<String>,
    }

    // -- Output: health check → JSON (EC2) --

    #[derive(Serialize)]
    pub struct HealthCheck {
        pub status: String,
        pub environment_changes: EnvironmentChanges,
        pub project_config: ProjectConfig,
        pub recommendations: Vec<String>,
    }

    #[derive(Serialize)]
    pub struct EnvironmentChanges {
        pub missing_tools: Vec<ToolChange>,
        pub new_tools: Vec<ToolChange>,
        pub version_changes: Vec<ToolChange>,
    }

    #[derive(Serialize)]
    pub struct ProjectConfig {
        pub llm: String,
        pub adapter_version: Option<String>,
        pub layer_patterns: u32,
        pub sessions: u32,
        pub beliefs: Option<u32>,
    }

    #[derive(Serialize)]
    pub struct ToolChange {
        pub name: String,
        pub old_version: Option<String>,
        pub new_version: Option<String>,
        pub required: bool,
    }
}

#[derive(Default)]
struct DoctorPlugin;

impl CommandPlugin for DoctorPlugin {
    fn name(&self) -> String {
        "doctor".into()
    }

    fn description(&self) -> String {
        "Check project health and environment".into()
    }

    fn run(&mut self, args: &[String]) -> i32 {
        let json_output = args.iter().any(|a| a == "--json" || a == "-j");

        // Find project root via host function
        if layer::find_project_root().is_none() {
            eprintln!("Error: Not in a Patina project directory. Run 'patina init' first.");
            return 1;
        }

        if !json_output {
            println!("Checking project health...");
        }

        // Get stored tools from project config via host
        let stored_tools = layer::get_stored_tools();

        // Get current environment via host — parse into typed struct at boundary (EC3)
        let current_env_json = match layer::detect_environment() {
            Ok(json) => json,
            Err(e) => {
                eprintln!("Error detecting environment: {}", e);
                return 1;
            }
        };
        let current_env: types::Environment = match serde_json::from_str(&current_env_json) {
            Ok(v) => v,
            Err(e) => {
                eprintln!("Error parsing environment: {}", e);
                return 1;
            }
        };

        // Analyze environment changes
        let (env_changes, recommendations, status) =
            analyze_environment(&current_env, &stored_tools);

        // Get project config for LLM info
        let config_json = match layer::read_config() {
            Ok(json) => json,
            Err(e) => {
                eprintln!("Error reading config: {}", e);
                return 1;
            }
        };
        let config: serde_json::Value = match serde_json::from_str(&config_json) {
            Ok(v) => v,
            Err(e) => {
                eprintln!("Error parsing config: {}", e);
                return 1;
            }
        };

        let llm = config
            .pointer("/adapters/default")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown")
            .to_string();

        // Check adapter version via host
        let adapter_version = layer::check_adapter_version(&llm).ok().flatten();

        // Count patterns and sessions via host
        let pattern_count: u32 = ["core", "topics", "projects"]
            .iter()
            .map(|dir| layer::count_layer_files(dir))
            .sum();
        let session_count = layer::count_layer_files("sessions");

        // Get UID via host
        let uid = layer::get_project_uid();

        // Query belief count via host query interface
        let beliefs = query::query("context", "{}")
            .ok()
            .and_then(|text| extract_belief_count(&text));

        let project_config = types::ProjectConfig {
            llm,
            adapter_version,
            layer_patterns: pattern_count,
            sessions: session_count,
            beliefs,
        };

        let health = types::HealthCheck {
            status: status.clone(),
            environment_changes: env_changes,
            project_config,
            recommendations,
        };

        // Emit measurement event (EC1: emit before any results are printed)
        let capture_metrics = serde_json::json!({
            "missing_tools": health.environment_changes.missing_tools.len(),
            "new_tools": health.environment_changes.new_tools.len(),
            "layer_patterns": health.project_config.layer_patterns,
            "sessions": health.project_config.sessions,
            "beliefs": health.project_config.beliefs.unwrap_or(0),
        });

        if let Err(e) = measure::record_measurement(
            "capture",
            "doctor",
            "health-check",
            &capture_metrics.to_string(),
        ) {
            eprintln!("Warning: failed to record measurement: {}", e);
        }

        // Output (EC4: probe framing — brief summary, not full dashboard)
        if json_output {
            println!(
                "{}",
                serde_json::to_string_pretty(&health).unwrap_or_default()
            );
        } else {
            display_probe_summary(&health, uid.as_deref());
        }

        // Exit code
        match status.as_str() {
            "healthy" => 0,
            "warning" => 2,
            "critical" => 3,
            _ => 1,
        }
    }
}

/// Analyze environment against stored tools. Returns (changes, recommendations, status).
///
/// Takes typed `Environment` — all JSON parsing happened at the boundary (EC3).
fn analyze_environment(
    env: &types::Environment,
    stored_tools: &[String],
) -> (types::EnvironmentChanges, Vec<String>, String) {
    let mut missing_tools = Vec::new();
    let mut new_tools = Vec::new();
    let mut recommendations = Vec::new();

    // Check for missing tools — typed field access, no get-chains
    for tool_name in stored_tools {
        let available = env
            .tools
            .get(tool_name)
            .map(|t| t.available)
            .unwrap_or(false);

        if !available {
            let required = is_tool_required(tool_name);
            missing_tools.push(types::ToolChange {
                name: tool_name.clone(),
                old_version: Some("detected".to_string()),
                new_version: None,
                required,
            });

            if required {
                recommendations.push(format!(
                    "Install {}: {}",
                    tool_name,
                    get_install_command(tool_name)
                ));
            }
        }
    }

    // Check for new tools
    for (name, info) in &env.tools {
        if info.available && !stored_tools.contains(name) {
            new_tools.push(types::ToolChange {
                name: name.clone(),
                old_version: None,
                new_version: info.version.clone(),
                required: false,
            });
        }
    }

    let status = if missing_tools.iter().any(|t| t.required) {
        "critical".to_string()
    } else if !missing_tools.is_empty() {
        "warning".to_string()
    } else {
        "healthy".to_string()
    };

    let env_changes = types::EnvironmentChanges {
        missing_tools,
        new_tools,
        version_changes: Vec::new(),
    };

    (env_changes, recommendations, status)
}

fn is_tool_required(tool: &str) -> bool {
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

/// Display probe summary — brief results + pointer to measure (EC4).
///
/// Doctor is a probe: "check complete, emitted to event stream."
/// Measure is the dashboard: "View full health: patina measure --full."
fn display_probe_summary(health: &types::HealthCheck, uid: Option<&str>) {
    println!("\n  Doctor check complete (emitted to event stream)\n");
    println!("  Environment: {}", health.status);

    // Tool summary
    if !health.environment_changes.missing_tools.is_empty() {
        for tool in &health.environment_changes.missing_tools {
            let marker = if tool.required { "!" } else { "-" };
            let required_msg = if tool.required { " (required)" } else { "" };
            println!("    [{}] missing: {}{}", marker, tool.name, required_msg);
        }
    }
    if !health.environment_changes.new_tools.is_empty() {
        for tool in &health.environment_changes.new_tools {
            let version = tool.new_version.as_deref().unwrap_or("detected");
            println!("    [+] new: {} {}", tool.name, version);
        }
    }
    if health.environment_changes.missing_tools.is_empty()
        && health.environment_changes.new_tools.is_empty()
    {
        println!("    tools: all present");
    }

    // Config summary
    let adapter = health
        .project_config
        .adapter_version
        .as_deref()
        .unwrap_or("unknown");
    println!(
        "    config: {} (adapter {})",
        health.project_config.llm, adapter
    );

    // Layer summary
    let belief_str = health
        .project_config
        .beliefs
        .map(|b| format!(", {} beliefs", b))
        .unwrap_or_default();
    println!(
        "    layer: {} patterns, {} sessions{}",
        health.project_config.layer_patterns, health.project_config.sessions, belief_str
    );

    // UID
    if uid.is_none() {
        println!("    uid: missing (will be created on next scrape)");
    }

    // Recommendations
    if !health.recommendations.is_empty() {
        println!();
        for rec in &health.recommendations {
            println!("    {}", rec);
        }
    }

    println!("\n  View full health: patina measure --full\n");
}

/// Extract belief count from context query output.
///
/// The context text includes lines like "Epistemic Beliefs: 105 total"
/// or similar patterns. Extracts the first number after "beliefs" (case-insensitive).
fn extract_belief_count(context_text: &str) -> Option<u32> {
    for line in context_text.lines() {
        let lower = line.to_lowercase();
        if lower.contains("belief") {
            // Find first number in the line
            for word in line.split_whitespace() {
                if let Ok(n) = word.parse::<u32>() {
                    return Some(n);
                }
            }
        }
    }
    None
}

register_command!(DoctorPlugin);
