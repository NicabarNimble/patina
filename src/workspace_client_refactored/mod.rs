// Dependable Rust: Black-box boundary for workspace_client
// Minimal public interface - all structs and implementation hidden

use anyhow::Result;
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

// Everything else is private
mod implementation;