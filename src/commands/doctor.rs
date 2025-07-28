use anyhow::{Context, Result};
use patina::session::SessionManager;
use patina::environment::Environment;
use std::fs;
use serde::{Serialize, Deserialize};

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

pub fn execute(check_only: bool, auto_fix: bool, json_output: bool) -> Result<i32> {
    // Find project root
    let project_root = SessionManager::find_project_root()
        .context("Not in a Patina project directory. Run 'patina init' first.")?;
    
    let non_interactive = auto_fix || json_output ||
        std::env::var("PATINA_NONINTERACTIVE").is_ok();
    
    if !json_output {
        println!("🏥 Checking project health...");
    }
    
    // Read project config
    let config_path = project_root.join(".patina").join("config.json");
    let config_content = fs::read_to_string(&config_path)
        .context("Failed to read project config")?;
    let config: serde_json::Value = serde_json::from_str(&config_content)?;
    
    // Get current environment
    let current_env = Environment::detect()?;
    
    // Get stored environment snapshot
    let stored_tools = config.get("environment_snapshot")
        .and_then(|s| s.get("detected_tools"))
        .and_then(|t| t.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str())
                .map(|s| s.to_string())
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    
    // Compare environments
    let mut health_check = analyze_environment(&current_env, &stored_tools, &config)?;
    
    // Check project status
    let llm = config.get("llm").and_then(|l| l.as_str()).unwrap_or("unknown");
    let adapter = patina::adapters::get_adapter(llm);
    let adapter_version = adapter.check_for_updates(&project_root)?
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
        
        if !health_check.environment_changes.missing_tools.is_empty() && !check_only {
            if auto_fix {
                if !json_output {
                    println!("\n🔧 Auto-fixing environment...");
                }
                update_environment_snapshot(&config_path, &current_env)?;
            } else if !non_interactive {
                print!("\nUpdate environment snapshot? [Y/n] ");
                use std::io::{self, Write};
                io::stdout().flush()?;
                let mut response = String::new();
                io::stdin().read_line(&mut response)?;
                
                if response.trim().is_empty() || response.trim().eq_ignore_ascii_case("y") {
                    update_environment_snapshot(&config_path, &current_env)?;
                    if !json_output {
                        println!("✓ Environment snapshot updated");
                    }
                }
            }
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

fn analyze_environment(current: &Environment, stored_tools: &[String], config: &serde_json::Value) -> Result<HealthCheck> {
    let mut missing_tools = Vec::new();
    let mut new_tools = Vec::new();
    let version_changes = Vec::new();
    let mut recommendations = Vec::new();
    
    // Check for missing tools
    for tool_name in stored_tools {
        if !current.tools.get(tool_name).map_or(false, |info| info.available) {
            let required = is_tool_required(tool_name, config);
            missing_tools.push(ToolChange {
                name: tool_name.clone(),
                old_version: Some("detected".to_string()),
                new_version: None,
                required,
            });
            
            if required {
                recommendations.push(format!("Install {}: {}", tool_name, get_install_command(tool_name)));
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

fn is_tool_required(tool: &str, config: &serde_json::Value) -> bool {
    // Check if tool is required based on project type and configuration
    match tool {
        "cargo" | "rust" => true, // Always required for Patina
        "docker" => {
            // Required if dev environment is docker or dagger
            config.get("dev")
                .and_then(|d| d.as_str())
                .map(|d| d == "docker" || d == "dagger")
                .unwrap_or(false)
        }
        "dagger" => {
            config.get("dev")
                .and_then(|d| d.as_str())
                .map(|d| d == "dagger")
                .unwrap_or(false)
        }
        _ => false,
    }
}

fn get_install_command(tool: &str) -> &'static str {
    match tool {
        "cargo" | "rust" => "curl https://sh.rustup.rs -sSf | sh",
        "docker" => "Visit https://docker.com/get-started",
        "dagger" => "curl -L https://dl.dagger.io/install.sh | sh",
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
                count += entries.filter_map(Result::ok)
                    .filter(|e| e.path().extension().map_or(false, |ext| ext == "md"))
                    .count();
            }
        }
    }
    count
}

fn count_sessions(sessions_path: &std::path::Path) -> usize {
    if let Ok(entries) = fs::read_dir(sessions_path) {
        entries.filter_map(Result::ok)
            .filter(|e| e.path().extension().map_or(false, |ext| ext == "md"))
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
        println!("  {} {}: {} → NOT FOUND{}", 
            marker, 
            tool.name, 
            tool.old_version.as_ref().unwrap_or(&"unknown".to_string()),
            if tool.required { " (required!)" } else { "" }
        );
    }
    
    // Display new tools
    for tool in &health.environment_changes.new_tools {
        println!("  ✓ New tool: {} {}", 
            tool.name, 
            tool.new_version.as_ref().unwrap_or(&"detected".to_string())
        );
    }
    
    println!("\nProject Configuration:");
    println!("  ✓ LLM: {} (adapter {})", 
        health.project_config.llm,
        health.project_config.adapter_version.as_ref().unwrap_or(&"unknown".to_string())
    );
    println!("  ✓ Layer: {} patterns stored", health.project_config.layer_patterns);
    println!("  ✓ Sessions: {} recorded", health.project_config.sessions);
    
    if !health.recommendations.is_empty() {
        println!("\nRecommendations:");
        for (i, rec) in health.recommendations.iter().enumerate() {
            println!("  {}. {}", i + 1, rec);
        }
    }
    
    Ok(())
}

fn update_environment_snapshot(config_path: &std::path::Path, env: &Environment) -> Result<()> {
    // Read current config
    let mut config: serde_json::Value = serde_json::from_str(&fs::read_to_string(config_path)?)?;
    
    // Update environment snapshot
    if let Some(snapshot) = config.get_mut("environment_snapshot") {
        snapshot["os"] = serde_json::Value::String(env.os.clone());
        snapshot["arch"] = serde_json::Value::String(env.arch.clone());
        snapshot["detected_tools"] = serde_json::Value::Array(
            env.tools.iter()
                .filter(|(_, info)| info.available)
                .map(|(name, _)| serde_json::Value::String(name.clone()))
                .collect()
        );
    }
    
    // Write back
    fs::write(config_path, serde_json::to_string_pretty(&config)?)?;
    Ok(())
}