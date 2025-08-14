use anyhow::{Context, Result};
use patina::workspace_client;
use std::fs;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::thread;
use std::time::Duration;

fn get_pid_file_path() -> Result<PathBuf> {
    let project_root = std::env::current_dir()?;
    let pid_dir = project_root.join(".patina");
    
    // Ensure .patina directory exists
    if !pid_dir.exists() {
        fs::create_dir_all(&pid_dir)?;
    }
    
    Ok(pid_dir.join("agent.pid"))
}

fn write_pid(pid: u32) -> Result<()> {
    let pid_file = get_pid_file_path()?;
    fs::write(&pid_file, pid.to_string())
        .with_context(|| format!("Failed to write PID file: {:?}", pid_file))
}

fn read_pid() -> Result<Option<u32>> {
    let pid_file = get_pid_file_path()?;
    
    if !pid_file.exists() {
        return Ok(None);
    }
    
    let pid_str = fs::read_to_string(&pid_file)?;
    let pid = pid_str.trim().parse::<u32>()
        .with_context(|| format!("Invalid PID in file: {}", pid_str))?;
    
    Ok(Some(pid))
}

fn remove_pid_file() -> Result<()> {
    let pid_file = get_pid_file_path()?;
    if pid_file.exists() {
        fs::remove_file(&pid_file)?;
    }
    Ok(())
}

fn is_process_running(pid: u32) -> bool {
    // On Unix, we can check if process exists by sending signal 0
    Command::new("kill")
        .arg("-0")
        .arg(pid.to_string())
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false)
}

pub fn start() -> Result<()> {
    const PORT: u16 = 8091;
    
    // Check if already running via PID file
    if let Some(pid) = read_pid()? {
        if is_process_running(pid) {
            println!("✅ Agent environment service is already running on port {}", PORT);
            println!("   PID: {}", pid);
            return Ok(());
        } else {
            // Process died but PID file remains, clean it up
            println!("⚠️  Found stale PID file, cleaning up...");
            remove_pid_file()?;
        }
    }
    
    // Also check if service is running on the port (fallback)
    if workspace_client::is_service_running(PORT) {
        println!("✅ Agent environment service is already running on port {}", PORT);
        println!("   (Started outside of patina agent)");
        return Ok(());
    }

    println!("🚀 Starting agent environment service...");

    // Check if Go is available
    let go_available = Command::new("go").arg("version").output().is_ok();

    if !go_available {
        anyhow::bail!(
            "Go is required to run the agent environment service. Please install Go first."
        );
    }

    // Start modular gateway
    let modules_dir = std::env::current_dir()?.join("modules/api-gateway");
    if !modules_dir.exists() {
        anyhow::bail!(
            "Agent environment service not found. The modules/ directory is missing.\n\
             Run 'patina init' in a Patina project."
        );
    }
    
    let mut child = Command::new("go")
        .arg("run")
        .arg("./cmd/server")
        .current_dir(&modules_dir)
        .env("PROJECT_ROOT", std::env::current_dir()?)
        .env("WORKTREE_ROOT", "/tmp/patina-worktrees")
        .env("PORT", PORT.to_string())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .context("Failed to start agent environment service")?;

    let pid = child.id();

    // Wait for service to be ready
    println!("⏳ Waiting for service to be ready...");
    let mut retries = 0;
    while retries < 30 {
        if workspace_client::is_service_running(PORT) {
            // Save PID to file
            write_pid(pid)?;
            
            println!("✅ Agent environment service is running on port {}", PORT);
            println!("   PID: {}", pid);
            println!("   Use 'patina agent stop' to stop the service");

            // Detach the process so it continues running
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
    const PORT: u16 = 8091;
    
    // Try to stop via PID file first
    if let Some(pid) = read_pid()? {
        if is_process_running(pid) {
            println!("🛑 Stopping agent environment service (PID: {})...", pid);
            
            // Try graceful shutdown with SIGTERM
            let output = Command::new("kill")
                .arg(pid.to_string())
                .output()
                .context("Failed to send stop signal")?;
            
            if output.status.success() {
                // Wait a bit for graceful shutdown
                thread::sleep(Duration::from_secs(2));
                
                // Check if it stopped
                if !is_process_running(pid) {
                    println!("✅ Agent environment service stopped");
                    remove_pid_file()?;
                    return Ok(());
                }
                
                // Force kill if still running
                println!("⚠️  Service didn't stop gracefully, forcing...");
                Command::new("kill")
                    .arg("-9")
                    .arg(pid.to_string())
                    .output()?;
                
                println!("✅ Agent environment service stopped (forced)");
                remove_pid_file()?;
                return Ok(());
            }
        } else {
            println!("ℹ️  PID file exists but process is not running");
            remove_pid_file()?;
        }
    }
    
    // Fallback: Check if service is running without our PID
    if !workspace_client::is_service_running(PORT) {
        println!("ℹ️  Agent environment service is not running");
        return Ok(());
    }

    println!("⚠️  Service is running but not managed by patina agent");
    println!("   You may need to manually kill the process");
    
    // Last resort: try pkill
    println!("   Attempting to stop via pkill...");
    let output = Command::new("pkill")
        .arg("-f")
        .arg("api-gateway/server")
        .output()
        .context("Failed to run pkill")?;

    if output.status.success() {
        println!("✅ Agent environment service stopped");
    } else {
        println!("⚠️  Failed to stop agent environment service");
    }

    Ok(())
}

pub fn status() -> Result<()> {
    const PORT: u16 = 8091;
    
    // Check PID file first
    if let Some(pid) = read_pid()? {
        if is_process_running(pid) {
            println!("✅ Workspace service is running on port {}", PORT);
            println!("   PID: {}", pid);
        } else {
            println!("⚠️  PID file exists but process {} is not running", pid);
            println!("   Cleaning up stale PID file...");
            remove_pid_file()?;
        }
    }
    
    // Also check if service is accessible
    if workspace_client::is_service_running(PORT) {
        if read_pid()?.is_none() {
            println!("✅ Workspace service is running on port {}", PORT);
            println!("   (Started outside of patina agent)");
        }

        // Try to get workspace list
        if let Ok(client) =
            patina::workspace_client::WorkspaceClient::new(format!("http://localhost:{}", PORT))
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
    const PORT: u16 = 8091;
    
    if !workspace_client::is_service_running(PORT) {
        println!("❌ Agent environment service is not running");
        println!("   Run 'patina agent start' to start the service");
        return Ok(());
    }
    
    // Get workspace list
    let client = patina::workspace_client::WorkspaceClient::new(format!("http://localhost:{}", PORT))?;
    
    match client.list_workspaces() {
        Ok(workspaces) => {
            if workspaces.is_empty() {
                println!("No active environments");
            } else {
                println!("Active environments ({}):", workspaces.len());
                for ws in workspaces {
                    println!("  {} - {} ({})", ws.id, ws.name, ws.status);
                    if let Some(path) = ws.worktree_path {
                        println!("    Worktree: {}", path);
                    }
                    println!("    Branch: {}", ws.branch_name);
                }
            }
        }
        Err(e) => {
            println!("❌ Failed to retrieve environment list: {}", e);
        }
    }
    
    Ok(())
}