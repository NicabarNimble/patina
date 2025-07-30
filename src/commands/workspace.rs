use anyhow::{Context, Result};
use std::process::{Command, Stdio};
use std::thread;
use std::time::Duration;
use patina::workspace_client;

pub fn start() -> Result<()> {
    // Check if already running
    if workspace_client::is_service_running(8080) {
        println!("✅ Workspace service is already running on port 8080");
        return Ok(());
    }

    println!("🚀 Starting workspace service...");
    
    // Check if Go is available
    let go_available = Command::new("go")
        .arg("version")
        .output()
        .is_ok();
        
    if !go_available {
        anyhow::bail!("Go is required to run the workspace service. Please install Go first.");
    }
    
    // Check if workspace directory exists
    let workspace_dir = std::env::current_dir()?.join("workspace");
    if !workspace_dir.exists() {
        anyhow::bail!("Workspace service not found. Run 'patina init' in a Patina project.");
    }
    
    // Start the workspace service in the background
    let mut child = Command::new("go")
        .arg("run")
        .arg("./cmd/workspace-server")
        .current_dir(&workspace_dir)
        .env("PROJECT_ROOT", std::env::current_dir()?)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .context("Failed to start workspace service")?;
    
    // Wait for service to be ready
    println!("⏳ Waiting for service to be ready...");
    let mut retries = 0;
    while retries < 30 {
        if workspace_client::is_service_running(8080) {
            println!("✅ Workspace service is running on port 8080");
            println!("   PID: {}", child.id());
            println!("   Use 'patina workspace stop' to stop the service");
            
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
    anyhow::bail!("Failed to start workspace service after 30 seconds")
}

pub fn stop() -> Result<()> {
    if !workspace_client::is_service_running(8080) {
        println!("ℹ️  Workspace service is not running");
        return Ok(());
    }
    
    println!("🛑 Stopping workspace service...");
    
    // In a real implementation, we'd read the PID from a file
    // For now, we'll use pkill
    let output = Command::new("pkill")
        .arg("-f")
        .arg("workspace-server")
        .output()
        .context("Failed to stop workspace service")?;
        
    if output.status.success() {
        println!("✅ Workspace service stopped");
    } else {
        println!("⚠️  Failed to stop workspace service");
        println!("   You may need to manually kill the process");
    }
    
    Ok(())
}

pub fn status() -> Result<()> {
    if workspace_client::is_service_running(8080) {
        println!("✅ Workspace service is running on port 8080");
        
        // Try to get workspace list
        match patina::workspace_client::WorkspaceClient::new("http://localhost:8080".to_string()) {
            Ok(client) => {
                match client.list_workspaces() {
                    Ok(workspaces) => {
                        println!("   Active workspaces: {}", workspaces.len());
                        for ws in workspaces {
                            println!("   - {} ({})", ws.name, ws.status);
                        }
                    }
                    Err(_) => {
                        println!("   Could not retrieve workspace list");
                    }
                }
            }
            Err(_) => {}
        }
    } else {
        println!("❌ Workspace service is not running");
        println!("   Run 'patina workspace start' to start the service");
    }
    
    Ok(())
}

pub fn list() -> Result<()> {
    if !workspace_client::is_service_running(8080) {
        anyhow::bail!("Workspace service is not running. Run 'patina workspace start' first.");
    }
    
    let client = patina::workspace_client::WorkspaceClient::new("http://localhost:8080".to_string())?;
    let workspaces = client.list_workspaces()?;
    
    if workspaces.is_empty() {
        println!("No active workspaces");
    } else {
        println!("Active workspaces:");
        for ws in workspaces {
            println!("  {} - {} ({})", ws.id, ws.name, ws.status);
            println!("    Branch: {}", ws.branch_name);
            println!("    Image: {}", ws.base_image);
        }
    }
    
    Ok(())
}