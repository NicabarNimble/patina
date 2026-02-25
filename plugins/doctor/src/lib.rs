//! Doctor command plugin — project health checks.
//!
//! Extracted from src/commands/doctor.rs into a WASM command plugin.
//! Uses patina:host/layer for read-only project data access instead
//! of direct library calls.
//!
//! Proves the command world: CLI subcommand that runs without Mother daemon.

use patina_sdk::command::{layer, measure, query};
use patina_sdk::{register_command, CommandPlugin};

/// JSON structures for health check output.
/// Mirrors the original compiled-in doctor types.
mod types {
    use serde_json::Value;

    pub struct HealthCheck {
        pub status: String,
        pub missing_tools: Vec<ToolChange>,
        pub new_tools: Vec<ToolChange>,
        pub llm: String,
        pub adapter_version: Option<String>,
        pub layer_patterns: u32,
        pub sessions: u32,
        pub uid: Option<String>,
        pub beliefs: Option<u32>,
        pub recommendations: Vec<String>,
    }

    pub struct ToolChange {
        pub name: String,
        pub old_version: Option<String>,
        pub new_version: Option<String>,
        pub required: bool,
    }

    impl HealthCheck {
        pub fn to_json(&self) -> Value {
            serde_json::json!({
                "status": self.status,
                "environment_changes": {
                    "missing_tools": self.missing_tools.iter().map(|t| serde_json::json!({
                        "name": t.name,
                        "old_version": t.old_version,
                        "new_version": t.new_version,
                        "required": t.required,
                    })).collect::<Vec<_>>(),
                    "new_tools": self.new_tools.iter().map(|t| serde_json::json!({
                        "name": t.name,
                        "old_version": t.old_version,
                        "new_version": t.new_version,
                        "required": t.required,
                    })).collect::<Vec<_>>(),
                    "version_changes": [],
                },
                "project_config": {
                    "llm": self.llm,
                    "adapter_version": self.adapter_version,
                    "layer_patterns": self.layer_patterns,
                    "sessions": self.sessions,
                    "beliefs": self.beliefs,
                },
                "recommendations": self.recommendations,
            })
        }
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
            println!("\u{1f3e5} Checking project health...");
        }

        // Get stored tools from project config via host
        let stored_tools = layer::get_stored_tools();

        // Get current environment via host
        let current_env_json = match layer::detect_environment() {
            Ok(json) => json,
            Err(e) => {
                eprintln!("Error detecting environment: {}", e);
                return 1;
            }
        };

        // Parse the environment JSON to extract tool info
        let current_env: serde_json::Value = match serde_json::from_str(&current_env_json) {
            Ok(v) => v,
            Err(e) => {
                eprintln!("Error parsing environment: {}", e);
                return 1;
            }
        };

        // Analyze environment changes
        let mut health = analyze_environment(&current_env, &stored_tools);

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
        let pattern_count = ["core", "topics", "projects"]
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

        health.llm = llm;
        health.adapter_version = adapter_version;
        health.layer_patterns = pattern_count;
        health.sessions = session_count;
        health.uid = uid;
        health.beliefs = beliefs;

        // Emit measurement event
        let capture_metrics = serde_json::json!({
            "missing_tools": health.missing_tools.len(),
            "new_tools": health.new_tools.len(),
            "layer_patterns": health.layer_patterns,
            "sessions": health.sessions,
            "beliefs": health.beliefs.unwrap_or(0),
        });

        if let Err(e) = measure::record_measurement(
            "capture",
            "doctor",
            "health-check",
            &capture_metrics.to_string(),
        ) {
            eprintln!("Warning: failed to record measurement: {}", e);
        }

        // Output
        if json_output {
            println!(
                "{}",
                serde_json::to_string_pretty(&health.to_json()).unwrap_or_default()
            );
        } else {
            display_health_check(&health);
        }

        // Exit code
        match health.status.as_str() {
            "healthy" => 0,
            "warning" => 2,
            "critical" => 3,
            _ => 1,
        }
    }
}

fn analyze_environment(
    current_env: &serde_json::Value,
    stored_tools: &[String],
) -> types::HealthCheck {
    let tools = current_env
        .get("tools")
        .and_then(|v| v.as_object())
        .cloned()
        .unwrap_or_default();

    let mut missing_tools = Vec::new();
    let mut new_tools = Vec::new();
    let mut recommendations = Vec::new();

    // Check for missing tools
    for tool_name in stored_tools {
        let available = tools
            .get(tool_name)
            .and_then(|t| t.get("available"))
            .and_then(|v| v.as_bool())
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
    for (name, info) in &tools {
        let available = info
            .get("available")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        if available && !stored_tools.contains(name) {
            let version = info
                .get("version")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());
            new_tools.push(types::ToolChange {
                name: name.clone(),
                old_version: None,
                new_version: version,
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

    types::HealthCheck {
        status,
        missing_tools,
        new_tools,
        llm: String::new(),
        adapter_version: None,
        layer_patterns: 0,
        sessions: 0,
        uid: None,
        beliefs: None,
        recommendations,
    }
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

fn display_health_check(health: &types::HealthCheck) {
    println!("\nEnvironment Changes Since Init:");

    for tool in &health.missing_tools {
        let marker = if tool.required {
            "\u{26a0}\u{fe0f} "
        } else {
            "  "
        };
        let old_version = tool.old_version.as_deref().unwrap_or("unknown");
        let required_msg = if tool.required { " (required!)" } else { "" };
        println!(
            "  {} {}: {} \u{2192} NOT FOUND{}",
            marker, tool.name, old_version, required_msg
        );
    }

    for tool in &health.new_tools {
        let version = tool.new_version.as_deref().unwrap_or("detected");
        println!("  \u{2713} New tool: {} {}", tool.name, version);
    }

    println!("\nProject Configuration:");
    if let Some(uid) = &health.uid {
        println!("  \u{2713} UID: {}", uid);
    } else {
        println!("  \u{26a0} UID: missing (will be created on next scrape)");
    }
    let adapter_version = health.adapter_version.as_deref().unwrap_or("unknown");
    println!(
        "  \u{2713} LLM: {} (adapter {})",
        health.llm, adapter_version
    );
    println!(
        "  \u{2713} Layer: {} patterns stored",
        health.layer_patterns
    );
    println!("  \u{2713} Sessions: {} recorded", health.sessions);
    if let Some(count) = health.beliefs {
        println!("  \u{2713} Beliefs: {} epistemic", count);
    }

    if !health.recommendations.is_empty() {
        println!("\nRecommendations:");
        for (i, rec) in health.recommendations.iter().enumerate() {
            println!("  {}. {}", i + 1, rec);
        }

        println!("\n\u{1f4a1} Run 'patina init .' to refresh your environment snapshot");
    }
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
