use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Duration;

/// Client for interacting with the workspace service
pub struct WorkspaceClient {
    base_url: String,
    client: reqwest::blocking::Client,
}

/// Workspace represents an isolated development environment
#[derive(Debug, Serialize, Deserialize)]
pub struct Workspace {
    pub id: String,
    pub name: String,
    pub container_id: String,
    pub branch_name: String,
    pub base_image: String,
    pub status: String,
}

/// Request to create a new workspace
#[derive(Debug, Serialize)]
pub struct CreateWorkspaceRequest {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub base_image: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub env: Option<HashMap<String, String>>,
}

/// Response from creating a workspace
#[derive(Debug, Deserialize)]
pub struct CreateWorkspaceResponse {
    pub workspace: Workspace,
}

/// Request to execute a command
#[derive(Debug, Serialize)]
pub struct ExecRequest {
    pub command: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub work_dir: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub env: Option<HashMap<String, String>>,
}

/// Response from executing a command
#[derive(Debug, Deserialize)]
pub struct ExecResponse {
    pub exit_code: i32,
    pub stdout: String,
    pub stderr: String,
}

/// Git status information
#[derive(Debug, Deserialize)]
pub struct GitStatus {
    pub branch: String,
    pub clean: bool,
    pub modified: Vec<String>,
    pub untracked: Vec<String>,
    pub current_commit: String,
}

/// Error response from the API
#[derive(Debug, Deserialize)]
pub struct ErrorResponse {
    pub error: String,
    pub code: Option<String>,
}

impl WorkspaceClient {
    /// Create a new workspace client
    pub fn new(base_url: String) -> Result<Self> {
        let client = reqwest::blocking::Client::builder()
            .timeout(Duration::from_secs(300)) // 5 minute timeout for long operations
            .build()
            .context("Failed to create HTTP client")?;

        Ok(Self { base_url, client })
    }

    /// Check if the workspace service is healthy
    pub fn health_check(&self) -> Result<bool> {
        let url = format!("{}/health", self.base_url);
        let response = self.client.get(&url).send()?;
        Ok(response.status().is_success())
    }

    /// Create a new workspace
    pub fn create_workspace(&self, request: CreateWorkspaceRequest) -> Result<Workspace> {
        let url = format!("{}/workspaces", self.base_url);
        let response = self
            .client
            .post(&url)
            .json(&request)
            .send()
            .context("Failed to create workspace")?;

        if response.status().is_success() {
            let resp: CreateWorkspaceResponse = response.json()?;
            Ok(resp.workspace)
        } else {
            let error: ErrorResponse = response.json()?;
            anyhow::bail!("Failed to create workspace: {}", error.error)
        }
    }

    /// Get a workspace by ID
    pub fn get_workspace(&self, id: &str) -> Result<Workspace> {
        let url = format!("{}/workspaces/{}", self.base_url, id);
        let response = self.client.get(&url).send()?;

        if response.status().is_success() {
            Ok(response.json()?)
        } else if response.status() == 404 {
            anyhow::bail!("Workspace not found: {}", id)
        } else {
            let error: ErrorResponse = response.json()?;
            anyhow::bail!("Failed to get workspace: {}", error.error)
        }
    }

    /// List all workspaces
    pub fn list_workspaces(&self) -> Result<Vec<Workspace>> {
        let url = format!("{}/workspaces", self.base_url);
        let response = self.client.get(&url).send()?;

        if response.status().is_success() {
            #[derive(Deserialize)]
            struct ListResponse {
                workspaces: Vec<Workspace>,
            }
            let resp: ListResponse = response.json()?;
            Ok(resp.workspaces)
        } else {
            let error: ErrorResponse = response.json()?;
            anyhow::bail!("Failed to list workspaces: {}", error.error)
        }
    }

    /// Delete a workspace
    pub fn delete_workspace(&self, id: &str) -> Result<()> {
        let url = format!("{}/workspaces/{}", self.base_url, id);
        let response = self.client.delete(&url).send()?;

        if response.status().is_success() || response.status() == 204 {
            Ok(())
        } else {
            let error: ErrorResponse = response.json()?;
            anyhow::bail!("Failed to delete workspace: {}", error.error)
        }
    }

    /// Execute a command in a workspace
    pub fn execute(&self, workspace_id: &str, request: ExecRequest) -> Result<ExecResponse> {
        let url = format!("{}/workspaces/{}/exec", self.base_url, workspace_id);
        let response = self
            .client
            .post(&url)
            .json(&request)
            .send()
            .context("Failed to execute command")?;

        if response.status().is_success() {
            Ok(response.json()?)
        } else {
            let error: ErrorResponse = response.json()?;
            anyhow::bail!("Failed to execute command: {}", error.error)
        }
    }

    /// Get git status of a workspace
    pub fn get_git_status(&self, workspace_id: &str) -> Result<GitStatus> {
        let url = format!("{}/workspaces/{}/git", self.base_url, workspace_id);
        let response = self.client.get(&url).send()?;

        if response.status().is_success() {
            Ok(response.json()?)
        } else {
            let error: ErrorResponse = response.json()?;
            anyhow::bail!("Failed to get git status: {}", error.error)
        }
    }

    /// Create a git branch in a workspace
    pub fn create_branch(&self, workspace_id: &str, branch_name: &str) -> Result<()> {
        let url = format!("{}/workspaces/{}/git/branch", self.base_url, workspace_id);

        #[derive(Serialize)]
        struct BranchRequest {
            branch_name: String,
        }

        let request = BranchRequest {
            branch_name: branch_name.to_string(),
        };

        let response = self.client.post(&url).json(&request).send()?;

        if response.status().is_success() || response.status() == 204 {
            Ok(())
        } else {
            let error: ErrorResponse = response.json()?;
            anyhow::bail!("Failed to create branch: {}", error.error)
        }
    }

    /// Commit changes in a workspace
    pub fn commit_changes(
        &self,
        workspace_id: &str,
        message: &str,
        author: Option<&str>,
        email: Option<&str>,
    ) -> Result<()> {
        let url = format!("{}/workspaces/{}/git/commit", self.base_url, workspace_id);

        #[derive(Serialize)]
        struct CommitRequest {
            message: String,
            #[serde(skip_serializing_if = "Option::is_none")]
            author: Option<String>,
            #[serde(skip_serializing_if = "Option::is_none")]
            email: Option<String>,
        }

        let request = CommitRequest {
            message: message.to_string(),
            author: author.map(|s| s.to_string()),
            email: email.map(|s| s.to_string()),
        };

        let response = self.client.post(&url).json(&request).send()?;

        if response.status().is_success() || response.status() == 204 {
            Ok(())
        } else {
            let error: ErrorResponse = response.json()?;
            anyhow::bail!("Failed to commit changes: {}", error.error)
        }
    }

    /// Push branch to origin
    pub fn push_branch(&self, workspace_id: &str) -> Result<()> {
        let url = format!("{}/workspaces/{}/git/push", self.base_url, workspace_id);
        let response = self.client.post(&url).send()?;

        if response.status().is_success() || response.status() == 204 {
            Ok(())
        } else {
            let error: ErrorResponse = response.json()?;
            anyhow::bail!("Failed to push branch: {}", error.error)
        }
    }
}

/// Check if the workspace service is running
pub fn is_service_running(port: u16) -> bool {
    let client = match WorkspaceClient::new(format!("http://localhost:{port}")) {
        Ok(c) => c,
        Err(_) => return false,
    };

    client.health_check().unwrap_or(false)
}
