// Dependable Rust: Black-box boundary for agent command
// Single public execute function hiding all implementation

use anyhow::Result;

/// Execute agent command with given subcommand
pub fn execute(subcommand: AgentSubcommand) -> Result<()> {
    implementation::execute_impl(subcommand)
}

/// Agent subcommands
#[derive(Debug, Clone)]
pub enum AgentSubcommand {
    Start,
    Stop,
    Status,
    List,
}

// Everything else is private
mod implementation {
    use super::*;
    use anyhow::{Context, Result};
    use std::process::{Command, Stdio};
    use std::thread;
    use std::time::Duration;

    pub(super) fn execute_impl(subcommand: AgentSubcommand) -> Result<()> {
        match subcommand {
            AgentSubcommand::Start => start(),
            AgentSubcommand::Stop => stop(),
            AgentSubcommand::Status => status(),
            AgentSubcommand::List => list(),
        }
    }

    fn start() -> Result<()> {
        // Check if already running
        if is_service_running(8080) {
            println!("✅ Agent environment service is already running on port 8080");
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

        // Check if workspace directory exists
        let workspace_dir = std::env::current_dir()?.join("workspace");
        if !workspace_dir.exists() {
            anyhow::bail!(
                "Agent environment service not found. Run 'patina init' in a Patina project."
            );
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
            .context("Failed to start agent environment service")?;

        // Wait for service to be ready
        println!("⏳ Waiting for service to be ready...");
        let mut retries = 0;
        while retries < 30 {
            if is_service_running(8080) {
                println!("✅ Agent environment service is running on port 8080");
                println!("   PID: {}", child.id());
                println!();
                println!("📝 You can now use:");
                println!("   - 'patina build' to build in a container");
                println!("   - 'patina test' to run tests in a container");
                println!("   - 'patina agent stop' to stop the service");
                
                // Detach the child process so it continues running
                let _ = child.try_wait();
                return Ok(());
            }
            thread::sleep(Duration::from_secs(1));
            retries += 1;
        }

        // If we get here, service didn't start in time
        let _ = child.kill();
        anyhow::bail!("Agent environment service failed to start within 30 seconds")
    }

    fn stop() -> Result<()> {
        if !is_service_running(8080) {
            println!("⚠️  Agent environment service is not running");
            return Ok(());
        }

        println!("🛑 Stopping agent environment service...");

        // Find and kill the process
        #[cfg(unix)]
        {
            use std::process::Command;
            
            // Find the process using lsof
            let output = Command::new("lsof")
                .args(&["-ti:8080"])
                .output()
                .context("Failed to find process on port 8080")?;

            if output.status.success() {
                let pid_str = String::from_utf8_lossy(&output.stdout);
                for pid in pid_str.lines() {
                    if let Ok(pid_num) = pid.trim().parse::<i32>() {
                        Command::new("kill")
                            .arg(pid_num.to_string())
                            .output()
                            .context("Failed to kill process")?;
                    }
                }
                println!("✅ Agent environment service stopped");
            } else {
                println!("⚠️  Could not find process on port 8080");
            }
        }

        #[cfg(not(unix))]
        {
            println!("⚠️  Manual stop required on this platform");
            println!("   Please manually stop the process listening on port 8080");
        }

        Ok(())
    }

    fn status() -> Result<()> {
        if is_service_running(8080) {
            println!("✅ Agent environment service is running on port 8080");
            
            // Try to get more info from the service
            let client = create_workspace_client()?;
            match client.health_check() {
                Ok(true) => println!("   Health: OK"),
                Ok(false) => println!("   Health: Unhealthy"),
                Err(_) => println!("   Health: Unknown"),
            }
        } else {
            println!("⚠️  Agent environment service is not running");
            println!("   Run 'patina agent start' to start the service");
        }
        Ok(())
    }

    fn list() -> Result<()> {
        if !is_service_running(8080) {
            println!("⚠️  Agent environment service is not running");
            println!("   Run 'patina agent start' to start the service");
            return Ok(());
        }

        let client = create_workspace_client()?;
        
        println!("📦 Active workspaces:");
        match client.list() {
            Ok(workspaces) => {
                if workspaces.is_empty() {
                    println!("   (none)");
                } else {
                    for ws in workspaces {
                        println!("   - {}", ws);
                    }
                }
            }
            Err(e) => {
                println!("   Error listing workspaces: {}", e);
            }
        }
        
        Ok(())
    }

    // Helper function to check if service is running (internal only)
    fn is_service_running(port: u16) -> bool {
        // Use the workspace_client's function but hide it internally
        // This avoids exposing workspace_client details
        if patina::config::use_refactored_workspace() {
            patina::workspace_client_refactored::is_service_running(port)
        } else {
            patina::workspace_client::is_service_running(port)
        }
    }

    // Helper to create workspace client (internal only)
    fn create_workspace_client() -> Result<Box<dyn WorkspaceOperations>> {
        if patina::config::use_refactored_workspace() {
            Ok(Box::new(WorkspaceClientWrapper::new()?))
        } else {
            Ok(Box::new(WorkspaceClientWrapper::new()?))
        }
    }

    // Internal trait to abstract workspace operations
    trait WorkspaceOperations {
        fn health_check(&self) -> Result<bool>;
        fn list(&self) -> Result<Vec<String>>;
    }

    // Wrapper to hide workspace_client details
    struct WorkspaceClientWrapper {
        #[allow(dead_code)]
        base_url: String,
    }

    impl WorkspaceClientWrapper {
        fn new() -> Result<Self> {
            Ok(Self {
                base_url: "http://localhost:8080".to_string(),
            })
        }
    }

    impl WorkspaceOperations for WorkspaceClientWrapper {
        fn health_check(&self) -> Result<bool> {
            // For now, just check if service is running
            // Later this can use the actual workspace client
            Ok(is_service_running(8080))
        }

        fn list(&self) -> Result<Vec<String>> {
            // Temporary implementation - will use actual client later
            Ok(vec![])
        }
    }
}