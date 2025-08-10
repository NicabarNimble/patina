// Dependable Rust: Black-box boundary for Dagger development environment
// Hides all workspace_client implementation details

use super::DevEnvironment;
use anyhow::Result;
use std::path::Path;

pub const DAGGER_VERSION: &str = "2.0.0";

/// Dagger development environment
pub struct DaggerEnvironment;

impl DevEnvironment for DaggerEnvironment {
    fn name(&self) -> &'static str {
        implementation::name()
    }

    fn version(&self) -> &'static str {
        DAGGER_VERSION
    }

    fn init_project(
        &self,
        project_path: &Path,
        project_name: &str,
        project_type: &str,
    ) -> Result<()> {
        implementation::init_project(project_path, project_name, project_type)
    }

    fn build(&self, project_path: &Path) -> Result<()> {
        implementation::build(project_path)
    }

    fn test(&self, project_path: &Path) -> Result<()> {
        implementation::test(project_path)
    }

    fn is_available(&self) -> bool {
        implementation::is_available()
    }

    fn fallback(&self) -> Option<&'static str> {
        Some("docker")
    }
}

// Everything else is private
mod implementation {
    use anyhow::{Context, Result};
    use std::path::Path;
    use std::process::Command;
    use uuid::Uuid;

    pub(super) fn name() -> &'static str {
        "dagger"
    }

    pub(super) fn init_project(
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

    pub(super) fn build(project_path: &Path) -> Result<()> {
        // Check if workspace service is running
        if !is_workspace_service_running() {
            anyhow::bail!(
                "Workspace service is not running. Please run 'patina agent start' first."
            );
        }

        let project_name = project_path
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("build");

        let workspace_name = format!("{}-build-{}", project_name, Uuid::new_v4());

        println!("📦 Creating Dagger workspace: {workspace_name}");

        // Create and execute build in workspace
        // This now hides all the CreateWorkspaceRequest details
        let workspace_id = create_build_workspace(&workspace_name)?;
        
        println!("✅ Workspace created: {}", workspace_id);

        // Wait for workspace to be ready
        wait_for_workspace_ready(&workspace_id)?;

        println!("🔨 Building project in Dagger container...");

        // Execute build command
        let output = execute_in_workspace(
            &workspace_id,
            vec!["cargo", "build", "--release"],
            Some("/workspace/project"),
        )?;

        // Print output
        println!("{}", output);

        // Clean up workspace
        cleanup_workspace(&workspace_id)?;

        println!("✅ Build complete!");
        Ok(())
    }

    pub(super) fn test(project_path: &Path) -> Result<()> {
        // Check if workspace service is running
        if !is_workspace_service_running() {
            anyhow::bail!(
                "Workspace service is not running. Please run 'patina agent start' first."
            );
        }

        let project_name = project_path
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("test");

        let workspace_name = format!("{}-test-{}", project_name, Uuid::new_v4());

        println!("🧪 Creating Dagger test workspace: {workspace_name}");

        // Create and execute tests in workspace
        let workspace_id = create_test_workspace(&workspace_name)?;
        
        println!("✅ Workspace created: {}", workspace_id);

        // Wait for workspace to be ready
        wait_for_workspace_ready(&workspace_id)?;

        println!("🧪 Running tests in Dagger container...");

        // Execute test command
        let output = execute_in_workspace(
            &workspace_id,
            vec!["cargo", "test"],
            Some("/workspace/project"),
        )?;

        // Print output
        println!("{}", output);

        // Clean up workspace
        cleanup_workspace(&workspace_id)?;

        println!("✅ Tests complete!");
        Ok(())
    }

    pub(super) fn is_available() -> bool {
        // Check if Go is available (required for workspace service)
        Command::new("go").arg("version").output().is_ok()
    }

    // Helper functions that hide workspace_client details

    fn is_workspace_service_running() -> bool {
        // This abstracts away the is_service_running call
        if crate::config::use_refactored_workspace() {
            crate::workspace_client_refactored::is_service_running(8080)
        } else {
            crate::workspace_client::is_service_running(8080)
        }
    }

    fn create_build_workspace(name: &str) -> Result<String> {
        // This hides the CreateWorkspaceRequest struct
        use std::collections::HashMap;
        
        if crate::config::use_refactored_workspace() {
            use crate::workspace_client_refactored::{CreateWorkspaceRequest, WorkspaceClient};
            let client = WorkspaceClient::new("http://localhost:8080".to_string())?;
            
            // For refactored version, we'll use the simplified API when it's ready
            // For now, still use the deprecated exports
            #[allow(deprecated)]
            let request = CreateWorkspaceRequest {
                name: name.to_string(),
                image: "rust:latest".to_string(),
                command: vec![],
                env: HashMap::new(),
                mounts: vec![],
            };
            
            // This is a temporary hack - the refactored version needs a better API
            // that doesn't expose these details
            client.create_workspace(name.to_string())
        } else {
            use crate::workspace_client::{CreateWorkspaceRequest, WorkspaceClient};
            let client = WorkspaceClient::new("http://localhost:8080".to_string())?;
            
            let request = CreateWorkspaceRequest {
                name: name.to_string(),
                base_image: Some("rust:latest".to_string()),
                env: None,
            };
            
            let workspace = client.create_workspace(request)?;
            Ok(workspace.id)
        }
    }

    fn create_test_workspace(name: &str) -> Result<String> {
        // Similar to create_build_workspace but for testing
        create_build_workspace(name) // For now, same as build
    }

    fn wait_for_workspace_ready(_workspace_id: &str) -> Result<()> {
        // Simplified waiting logic
        std::thread::sleep(std::time::Duration::from_secs(2));
        Ok(())
    }

    fn execute_in_workspace(
        workspace_id: &str,
        command: Vec<&str>,
        work_dir: Option<&str>,
    ) -> Result<String> {
        // This hides the ExecRequest struct
        use std::collections::HashMap;
        
        if crate::config::use_refactored_workspace() {
            use crate::workspace_client_refactored::{ExecRequest, WorkspaceClient};
            let client = WorkspaceClient::new("http://localhost:8080".to_string())?;
            
            // Use the simplified exec API
            let command_strings: Vec<String> = command.iter().map(|s| s.to_string()).collect();
            client.exec(workspace_id, command_strings)
        } else {
            use crate::workspace_client::{ExecRequest, WorkspaceClient};
            let client = WorkspaceClient::new("http://localhost:8080".to_string())?;
            
            #[allow(deprecated)]
            let request = ExecRequest {
                command: command.iter().map(|s| s.to_string()).collect(),
                work_dir: work_dir.map(|s| s.to_string()),
                env: None,
            };
            
            let response = client.execute(workspace_id, request)?;
            
            // Combine stdout and stderr
            Ok(format!("{}\n{}", response.stdout, response.stderr))
        }
    }

    fn cleanup_workspace(workspace_id: &str) -> Result<()> {
        // Clean up the workspace
        if crate::config::use_refactored_workspace() {
            use crate::workspace_client_refactored::WorkspaceClient;
            let client = WorkspaceClient::new("http://localhost:8080".to_string())?;
            client.delete(workspace_id)
        } else {
            use crate::workspace_client::WorkspaceClient;
            let client = WorkspaceClient::new("http://localhost:8080".to_string())?;
            client.delete_workspace(workspace_id)
        }
    }
}