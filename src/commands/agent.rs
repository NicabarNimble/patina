use anyhow::{Context, Result};
use patina::workspace_client;
use std::process::{Command, Stdio};
use std::thread;
use std::time::Duration;

pub fn start() -> Result<()> {
    // Check if using modular system
    let use_modular = std::env::var("PATINA_USE_MODULAR").unwrap_or_default() == "true";
    let port = if use_modular { 8091 } else { 8080 };
    
    // Check if already running
    if workspace_client::is_service_running(port) {
        println!("✅ Agent environment service is already running on port {}", port);
        return Ok(());
    }

    println!("🚀 Starting agent environment service{}...", 
        if use_modular { " (modular)" } else { "" });

    // Check if Go is available
    let go_available = Command::new("go").arg("version").output().is_ok();

    if !go_available {
        anyhow::bail!(
            "Go is required to run the agent environment service. Please install Go first."
        );
    }

    let mut child = if use_modular {
        // Start modular gateway
        let modules_dir = std::env::current_dir()?.join("modules/api-gateway");
        if !modules_dir.exists() {
            anyhow::bail!(
                "Modular gateway not found. The modules/ directory is missing."
            );
        }
        
        Command::new("go")
            .arg("run")
            .arg("./cmd/server")
            .current_dir(&modules_dir)
            .env("PROJECT_ROOT", std::env::current_dir()?)
            .env("WORKTREE_ROOT", "/tmp/patina-worktrees")
            .env("PORT", port.to_string())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .context("Failed to start modular gateway")?
    } else {
        // Start old workspace service
        let workspace_dir = std::env::current_dir()?.join("workspace");
        if !workspace_dir.exists() {
            anyhow::bail!(
                "Agent environment service not found. Run 'patina init' in a Patina project."
            );
        }
        
        Command::new("go")
            .arg("run")
            .arg("./cmd/workspace-server")
            .current_dir(&workspace_dir)
            .env("PROJECT_ROOT", std::env::current_dir()?)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .context("Failed to start agent environment service")?
    };

    // Wait for service to be ready
    println!("⏳ Waiting for service to be ready...");
    let mut retries = 0;
    while retries < 30 {
        if workspace_client::is_service_running(port) {
            println!("✅ Agent environment service is running on port {}", port);
            println!("   PID: {}", child.id());
            println!("   Use 'patina agent stop' to stop the service");
            if use_modular {
                println!("   Using modular architecture");
            }

            // Detach the process so it continues running
            // In a real implementation, we'd save the PID to a file
            std::mem::forget(child);

            return Ok(());
        }
        thread::sleep(Duration::from_secs(1));
        retries += 1;
    }

    // If we get here, service failed to start
    let _ = child.kill();
    anyhow::bail!("Failed to start agent environment service after 30 seconds")
}

pub fn stop() -> Result<()> {
    let use_modular = std::env::var("PATINA_USE_MODULAR").unwrap_or_default() == "true";
    let port = if use_modular { 8091 } else { 8080 };
    
    if !workspace_client::is_service_running(port) {
        println!("ℹ️  Agent environment service is not running");
        return Ok(());
    }

    println!("🛑 Stopping agent environment service...");

    // In a real implementation, we'd read the PID from a file
    // For now, we'll use pkill
    let pattern = if use_modular {
        "api-gateway/server"
    } else {
        "workspace-server"
    };
    
    let output = Command::new("pkill")
        .arg("-f")
        .arg(pattern)
        .output()
        .context("Failed to stop agent environment service")?;

    if output.status.success() {
        println!("✅ Agent environment service stopped");
    } else {
        println!("⚠️  Failed to stop agent environment service");
        println!("   You may need to manually kill the process");
    }

    Ok(())
}

pub fn status() -> Result<()> {
    let use_modular = std::env::var("PATINA_USE_MODULAR").unwrap_or_default() == "true";
    let port = if use_modular { 8091 } else { 8080 };
    
    if workspace_client::is_service_running(port) {
        println!("✅ Workspace service is running on port {}", port);
        if use_modular {
            println!("   Using modular architecture");
        }

        // Try to get workspace list
        if let Ok(client) =
            patina::workspace_client::WorkspaceClient::new(format!("http://localhost:{}", port))
        {
            match client.list_workspaces() {
                Ok(workspaces) => {
                    println!("   Active environments: {}", workspaces.len());
                    for ws in workspaces {
                        println!("   - {} ({})", ws.name, ws.status);
                    }
                }
                Err(e) => {
                    println!("   Could not retrieve environment list: {}", e);
                }
            }
        }
    } else {
        println!("❌ Agent environment service is not running");
        println!("   Run 'patina agent start' to start the service");
    }

    Ok(())
}

pub fn list() -> Result<()> {
    if !workspace_client::is_service_running(8080) {
        anyhow::bail!("Agent environment service is not running. Run 'patina agent start' first.");
    }

    let client =
        patina::workspace_client::WorkspaceClient::new("http://localhost:8080".to_string())?;
    let workspaces = client.list_workspaces()?;

    if workspaces.is_empty() {
        println!("No active agent environments");
    } else {
        println!("Active agent environments:");
        for ws in workspaces {
            println!("  {} - {} ({})", ws.id, ws.name, ws.status);
            println!("    Branch: {}", ws.branch_name);
            println!("    Image: {}", ws.base_image);
        }
    }

    Ok(())
}
