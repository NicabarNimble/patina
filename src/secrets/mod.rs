//! Secrets management for Patina
//!
//! Integrates with 1Password to provide secure secret handling.
//! LLMs never see secret values - only names and references.
//!
//! # Design
//!
//! Follows the model pattern:
//! - Mothership registry (`~/.patina/secrets.toml`) holds secret definitions
//! - 1Password "Patina" vault holds actual values
//! - Project config declares requirements
//! - This module resolves and validates
//!
//! ```text
//! 1Password "Patina" vault  →  Actual secret values (external)
//!      ↓
//! secrets.toml (mothership) →  Secret definitions (names, items, env vars)
//!      ↓
//! config.toml (project)     →  What secrets this project needs
//!      ↓
//! this module               →  Resolves and validates at runtime
//! ```
//!
//! # Example
//!
//! ```ignore
//! use patina::secrets::{SecretsRegistry, check_op_status};
//!
//! // Check 1Password status
//! let status = check_op_status()?;
//! if !status.signed_in {
//!     println!("Please sign in: op signin");
//! }
//!
//! // Load registry
//! let registry = SecretsRegistry::load()?;
//!
//! // Check if a secret is registered
//! if let Some(secret) = registry.get("github-token") {
//!     println!("Maps to env: {}", secret.env);
//! }
//! ```

mod internal;

pub use internal::{OpRef, OpStatus, SecretDef, SecretsRegistry, ValidationReport};

use anyhow::Result;
use std::path::Path;

/// Check 1Password CLI status.
///
/// Returns information about:
/// - Whether `op` CLI is installed
/// - Whether user is signed in
/// - Whether "Patina" vault exists
pub fn check_op_status() -> Result<OpStatus> {
    internal::check_op_status()
}

/// Initialize the Patina vault in 1Password.
///
/// Creates the vault if it doesn't exist, creates empty secrets.toml.
pub fn init_vault() -> Result<()> {
    internal::init_vault()
}

/// Load the mothership secrets registry.
///
/// Returns empty registry if file doesn't exist.
pub fn load_registry() -> Result<SecretsRegistry> {
    SecretsRegistry::load()
}

/// Save the mothership secrets registry.
pub fn save_registry(registry: &SecretsRegistry) -> Result<()> {
    registry.save()
}

/// Load project secret requirements from config.
///
/// Reads `[secrets] requires = [...]` from `.patina/config.toml`.
pub fn load_project_requirements(project_root: &Path) -> Result<Vec<String>> {
    internal::load_project_requirements(project_root)
}

/// Validate secrets against 1Password.
///
/// Checks that each secret:
/// 1. Is registered in the mothership
/// 2. Has a corresponding item in 1Password
pub fn validate_secrets(names: &[String], registry: &SecretsRegistry) -> Result<ValidationReport> {
    internal::validate_secrets(names, registry)
}

/// Generate op:// references for secrets.
///
/// Returns the env var name and op:// URI for each secret.
pub fn generate_op_refs(names: &[String], registry: &SecretsRegistry) -> Result<Vec<OpRef>> {
    internal::generate_op_refs(names, registry)
}

/// Add a secret to the mothership registry.
///
/// Non-interactive: assumes 1Password item already exists.
/// Returns error if item not found in vault.
pub fn add_secret(
    name: &str,
    item: Option<&str>,
    field: Option<&str>,
    env: &str,
) -> Result<()> {
    internal::add_secret(name, item, field, env)
}

/// Execute a command with secrets injected.
///
/// 1. Loads project requirements
/// 2. Resolves against registry
/// 3. Validates against 1Password
/// 4. Runs command via `op run`
pub fn run_with_secrets(project_root: &Path, command: &[String]) -> Result<i32> {
    internal::run_with_secrets(project_root, command)
}

/// Execute a command on a remote host with secrets injected.
///
/// 1. Resolves secrets locally via `op read`
/// 2. Constructs SSH command with env prefix
/// 3. Secrets travel encrypted, never on disk
pub fn run_with_secrets_ssh(project_root: &Path, host: &str, command: &[String]) -> Result<i32> {
    internal::run_with_secrets_ssh(project_root, host, command)
}

#[cfg(test)]
mod tests {
    use crate::paths;

    #[test]
    fn test_registry_path() {
        let path = paths::secrets::registry_path();
        assert!(path.to_string_lossy().ends_with("secrets.toml"));
        assert!(path.to_string_lossy().contains(".patina"));
    }
}
