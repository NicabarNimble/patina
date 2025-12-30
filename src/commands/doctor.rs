use anyhow::{Context, Result};
use patina::environment::Environment;
use patina::project;
use patina::session::SessionManager;
use serde::{Deserialize, Serialize};
use std::fs;

#[derive(Serialize, Deserialize)]
struct HealthCheck {
    status: String, // "healthy", "warning", "critical"
    environment_changes: EnvironmentChanges,
    project_config: ProjectStatus,
    recommendations: Vec<String>,
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

pub fn execute(json_output: bool, audit_files: bool) -> Result<i32> {
    // Find project root first (needed for all subcommands)
    let project_root = SessionManager::find_project_root()
        .context("Not in a Patina project directory. Run 'patina init' first.")?;

    // If --audit flag is set, run file audit instead
    if audit_files {
        crate::commands::audit::execute(&project_root)?;
        return Ok(0);
    }

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
    let mut health_check = analyze_environment(&current_env, &stored_tools, &config.dev.dev_type)?;

    // Check project status - use frontends.default as the LLM
    let llm = &config.frontends.default;
    let adapter = patina::adapters::get_adapter(llm);
    let adapter_version = adapter
        .check_for_updates(&project_root)?
        .map(|(current, _)| current);

    // Count layer patterns
    let layer_path = project_root.join("layer");
    let pattern_count = count_patterns(&layer_path);

    // Count sessions
    let sessions_path = adapter.get_sessions_path(&project_root);
    let session_count = sessions_path
        .as_ref()
        .map(|path| count_sessions(path))
        .unwrap_or(0);

    health_check.project_config = ProjectStatus {
        llm: llm.to_string(),
        adapter_version,
        layer_patterns: pattern_count,
        sessions: session_count,
    };

    // Display results
    if json_output {
        println!("{}", serde_json::to_string_pretty(&health_check)?);
    } else {
        display_health_check(&health_check, &current_env)?;

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

fn analyze_environment(
    current: &Environment,
    stored_tools: &[String],
    dev_type: &str,
) -> Result<HealthCheck> {
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
            let required = is_tool_required(tool_name, dev_type);
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
        recommendations,
    })
}

fn is_tool_required(tool: &str, dev_type: &str) -> bool {
    // Check if tool is required based on project type and configuration
    match tool {
        "cargo" | "rust" => true, // Always required for Patina
        "docker" => dev_type == "docker",
        _ => false,
    }
}

fn get_install_command(tool: &str) -> &'static str {
    match tool {
        "cargo" | "rust" => "curl https://sh.rustup.rs -sSf | sh",
        "docker" => "Visit https://docker.com/get-started",
        "git" => "brew install git (macOS) or apt install git (Linux)",
        _ => "Check your package manager",
    }
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

fn display_health_check(health: &HealthCheck, _env: &Environment) -> Result<()> {
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

    if !health.recommendations.is_empty() {
        println!("\nRecommendations:");
        for (i, rec) in health.recommendations.iter().enumerate() {
            println!("  {}. {rec}", i + 1);
        }
    }

    Ok(())
}
