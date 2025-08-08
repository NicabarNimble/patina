use super::DevEnvironment;
use crate::workspace_client::{self, CreateWorkspaceRequest, ExecRequest, WorkspaceClient};
use anyhow::{Context, Result};
use std::path::Path;
use uuid::Uuid;
use which;

pub const DAGGER_VERSION: &str = "2.0.0"; // New workspace-based version

pub struct DaggerEnvironment;

impl DevEnvironment for DaggerEnvironment {
    fn name(&self) -> &'static str {
        "dagger"
    }

    fn version(&self) -> &'static str {
        DAGGER_VERSION
    }

    fn init_project(
        &self,
        _project_path: &Path,
        project_name: &str,
        _project_type: &str,
    ) -> Result<()> {
        // The new Dagger approach doesn't need template files
        // Everything runs through the workspace service
        println!("🚀 Dagger environment ready for {project_name}");
        println!("   Build and test will use isolated workspace containers");
        Ok(())
    }

    fn build(&self, project_path: &Path) -> Result<()> {
        // Check if workspace service is running
        if !workspace_client::is_service_running(8080) {
            anyhow::bail!(
                "Workspace service is not running. Please run 'patina agent start' first."
            );
        }

        let client = WorkspaceClient::new("http://localhost:8080".to_string())?;

        // Create a workspace for this build
        let project_name = project_path
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("build");

        let workspace_name = format!("{}-build-{}", project_name, Uuid::new_v4());

        println!("📦 Creating Dagger workspace: {workspace_name}");

        let request = CreateWorkspaceRequest {
            name: workspace_name.clone(),
            base_image: Some("rust:latest".to_string()),
            env: None,
        };

        let workspace = client
            .create_workspace(request)
            .context("Failed to create workspace")?;

        println!("✅ Workspace created: {}", workspace.id);

        // Wait for workspace to be ready
        let mut retries = 0;
        loop {
            let ws = client.get_workspace(&workspace.id)?;
            if ws.status == "ready" {
                break;
            }
            if ws.status == "error" {
                anyhow::bail!("Workspace failed to initialize");
            }
            if retries > 30 {
                anyhow::bail!("Timeout waiting for workspace to be ready");
            }
            std::thread::sleep(std::time::Duration::from_secs(1));
            retries += 1;
        }

        println!("🔨 Building project in Dagger container...");

        // Run cargo build in the workspace
        let exec_request = ExecRequest {
            command: vec![
                "cargo".to_string(),
                "build".to_string(),
                "--release".to_string(),
            ],
            work_dir: Some("/workspace/project".to_string()),
            env: None,
        };

        let result = client
            .execute(&workspace.id, exec_request)
            .context("Failed to execute build command")?;

        // Print output
        if !result.stdout.is_empty() {
            println!("{}", result.stdout);
        }
        if !result.stderr.is_empty() {
            eprintln!("{}", result.stderr);
        }

        // Check exit code
        if result.exit_code != 0 {
            anyhow::bail!("Build failed with exit code {}", result.exit_code);
        }

        println!("✅ Build completed successfully");

        // Clean up workspace
        println!("🧹 Cleaning up workspace...");
        client.delete_workspace(&workspace.id)?;

        Ok(())
    }

    fn test(&self, project_path: &Path) -> Result<()> {
        // Check if workspace service is running
        if !workspace_client::is_service_running(8080) {
            anyhow::bail!(
                "Workspace service is not running. Please run 'patina agent start' first."
            );
        }

        let client = WorkspaceClient::new("http://localhost:8080".to_string())?;

        let project_name = project_path
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("test");

        let workspace_name = format!("{}-test-{}", project_name, Uuid::new_v4());

        println!("📦 Creating Dagger workspace: {workspace_name}");

        let request = CreateWorkspaceRequest {
            name: workspace_name.clone(),
            base_image: Some("rust:latest".to_string()),
            env: None,
        };

        let workspace = client.create_workspace(request)?;

        // Wait for ready
        let mut retries = 0;
        loop {
            let ws = client.get_workspace(&workspace.id)?;
            if ws.status == "ready" {
                break;
            }
            if ws.status == "error" {
                anyhow::bail!("Workspace failed to initialize");
            }
            if retries > 30 {
                anyhow::bail!("Timeout waiting for workspace to be ready");
            }
            std::thread::sleep(std::time::Duration::from_secs(1));
            retries += 1;
        }

        println!("🧪 Running tests in Dagger container...");

        let exec_request = ExecRequest {
            command: vec!["cargo".to_string(), "test".to_string()],
            work_dir: Some("/workspace/project".to_string()),
            env: None,
        };

        let result = client.execute(&workspace.id, exec_request)?;

        // Print output
        if !result.stdout.is_empty() {
            println!("{}", result.stdout);
        }
        if !result.stderr.is_empty() {
            eprintln!("{}", result.stderr);
        }

        // Clean up
        client.delete_workspace(&workspace.id)?;

        if result.exit_code != 0 {
            anyhow::bail!("Tests failed with exit code {}", result.exit_code);
        }

        println!("✅ All tests passed");
        Ok(())
    }

    fn is_available(&self) -> bool {
        // Dagger is available if we have Go (to run the workspace service)
        which::which("go").is_ok()
    }

    fn fallback(&self) -> Option<&'static str> {
        Some("docker")
    }
}
