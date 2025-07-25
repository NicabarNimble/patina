use anyhow::{Context, Result};
use std::env;
use std::path::Path;
use std::process::Command;
use patina::session::SessionManager;

pub fn execute(command: Option<String>) -> Result<()> {
    // Check if we have a Dagger pipeline with agent support
    if !Path::new("pipelines/main.go").exists() {
        anyhow::bail!("No Dagger pipeline found. Run 'patina init' with --dev=dagger first.");
    }
    
    // Get or generate session ID
    let session_id = env::var("PATINA_SESSION_ID")
        .unwrap_or_else(|_| {
            // Try to get from current Patina session
            if let Ok(project_root) = SessionManager::find_project_root() {
                let session_manager = SessionManager::new(&project_root);
                if let Ok(session) = session_manager.get_or_create_session() {
                    session.id.clone()
                } else {
                    format!("agent-{}", chrono::Utc::now().timestamp())
                }
            } else {
                // Generate a simple ID based on timestamp
                format!("agent-{}", chrono::Utc::now().timestamp())
            }
        });
    
    println!("🤖 Starting agent workflow with session: {}", session_id);
    
    // Set environment variable for Dagger pipeline
    env::set_var("PATINA_SESSION_ID", &session_id);
    
    let subcommand = command.as_deref().unwrap_or("agent");
    
    match subcommand {
        "test" => {
            println!("🧪 Running tests in isolated agent environment...");
            run_dagger_command(&["test"])?;
        }
        "workspace" | "agent" => {
            println!("🔧 Creating isolated agent workspace...");
            run_dagger_command(&["agent"])?;
            
            println!("\n📝 Agent workspace created!");
            println!("   Session ID: {}", session_id);
            println!("   Branch: agent/{}", session_id);
            println!("\n💡 Next steps:");
            println!("   - The agent can now work in the isolated container");
            println!("   - Changes are tracked on the agent branch");
            println!("   - Use 'git checkout agent/{}' to review changes", session_id);
            println!("   - Run '/session-update' to capture insights");
        }
        "shell" => {
            println!("🐚 Starting interactive shell in agent container...");
            println!("   (This feature requires Dagger SDK support for interactive containers)");
            anyhow::bail!("Interactive shell not yet implemented");
        }
        _ => {
            anyhow::bail!("Unknown agent command: {}. Available: workspace, test, shell", subcommand);
        }
    }
    
    Ok(())
}

fn run_dagger_command(args: &[&str]) -> Result<()> {
    // Check if Go is available
    if which::which("go").is_err() {
        anyhow::bail!("Go is required for Dagger pipelines. Please install Go 1.21+");
    }
    
    let mut cmd = Command::new("go");
    cmd.current_dir("pipelines")
        .arg("run")
        .arg(".")
        .args(args);
    
    let status = cmd.status()
        .context("Failed to run Dagger pipeline")?;
    
    if !status.success() {
        anyhow::bail!("Dagger pipeline failed");
    }
    
    Ok(())
}