// Dependable Rust: Black-box boundary for workspace_client
// Minimal public interface - all structs and implementation hidden

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Client for workspace operations
pub struct WorkspaceClient {
    inner: Box<implementation::WorkspaceClientImpl>,
}

impl WorkspaceClient {
    /// Create a new workspace client
    pub fn new(base_url: String) -> Result<Self> {
        Ok(Self {
            inner: Box::new(implementation::WorkspaceClientImpl::new(base_url)?),
        })
    }

    /// Create a new workspace
    pub fn create_workspace(&self, name: String) -> Result<String> {
        self.inner.create_workspace(name)
    }

    /// Execute a command in a workspace
    pub fn exec(&self, workspace_id: &str, command: Vec<String>) -> Result<String> {
        self.inner.exec(workspace_id, command)
    }

    /// List all workspaces
    pub fn list(&self) -> Result<Vec<String>> {
        self.inner.list()
    }

    /// Delete a workspace
    pub fn delete(&self, workspace_id: &str) -> Result<()> {
        self.inner.delete(workspace_id)
    }

    /// Check if service is healthy
    pub fn health_check(&self) -> Result<bool> {
        self.inner.health_check()
    }
}

// TEMPORARY EXPORTS - These will be removed once commands are black-boxed
// TODO: Remove these exports after black-boxing agent.rs and dev_env/dagger.rs

/// Request to create a workspace (DEPRECATED - will be removed)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[deprecated(note = "Will be removed after black-boxing. Use WorkspaceClient methods instead")]
pub struct CreateWorkspaceRequest {
    pub name: String,
    pub image: String,
    pub command: Vec<String>,
    pub env: HashMap<String, String>,
    pub mounts: Vec<String>,
}

/// Request to execute command in workspace (DEPRECATED - will be removed)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[deprecated(note = "Will be removed after black-boxing. Use WorkspaceClient::exec instead")]
pub struct ExecRequest {
    pub command: Vec<String>,
    pub env: HashMap<String, String>,
    pub working_dir: Option<String>,
}

/// Check if the workspace service is running (DEPRECATED - will be removed)
#[deprecated(note = "Will be removed after black-boxing. Use WorkspaceClient::health_check instead")]
pub fn is_service_running(port: u16) -> bool {
    implementation::is_service_running(port)
}

// Everything else is private
mod implementation;