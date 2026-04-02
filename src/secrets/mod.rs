//! Secrets management for Patina (v2 - Local Vault)
//!
//! Local-first secrets management using age encryption and macOS Keychain.
//! No cloud accounts, no SaaS dependencies.
//!
//! # Architecture
//!
//! ```text
//! Identity (your private key)
//! ├── PATINA_IDENTITY env var (CI/headless)
//! └── macOS Keychain "Patina Secrets" (Touch ID protected)
//!           │
//!           │ decrypt
//!           ▼
//! Global Vault (personal)        Project Vault (shared)
//! ~/.patina/                     .patina/
//! ├── secrets.toml               ├── secrets.toml
//! ├── recipient.txt              ├── recipients.txt
//! └── vault.age                  └── vault.age
//!           │
//!           │ merge (project overrides global)
//!           ▼
//! patina secrets run -- cargo test
//! ```
//!
//! # Example
//!
//! ```ignore
//! use patina::secrets;
//!
//! // Check status
//! let status = secrets::check_status(Some(project_root))?;
//!
//! // Add a secret
//! secrets::add_secret("github-token", "ghp_xxx", Some("GITHUB_TOKEN"), false)?;
//!
//! // Run with secrets
//! secrets::run_with_secrets(Some(project_root), &["cargo", "test"])?;
//! ```

mod encrypted_file;
mod identity;
mod keychain;
mod recipients;
mod registry;
mod session;
mod storage;
mod vault;

// Public exports
pub use self::identity::IdentitySource;
pub use self::registry::{infer_env_name, is_valid_env_name, is_valid_secret_name};
pub use self::vault::VaultStatus;

use crate::paths;
use anyhow::{bail, Result};
use patina_protocol::{
    BuiltinChild, BuiltinChildAction, BuiltinChildRequest, BuiltinChildResult,
    SecretsAuthorityOperation, SecretsDispatchRequest,
};
use std::collections::HashMap;
use std::io::Write;
use std::path::Path;
use std::process::{Command, Stdio};

// =============================================================================
// Status
// =============================================================================

/// Combined status of global and project vaults.
#[derive(Debug)]
pub struct SecretsStatus {
    pub global: VaultStatus,
    pub project: Option<VaultStatus>,
    pub identity_source: Option<IdentitySource>,
    /// Your public key (for sharing with others)
    pub recipient_key: Option<String>,
}

/// Check vault status (both global and project).
pub fn check_status(project_root: Option<&Path>) -> Result<SecretsStatus> {
    let global_vault = paths::secrets::vault_path();
    let global_recipients = paths::secrets::recipient_path();
    let global_registry = paths::secrets::registry_path();
    let global = vault::check_status(&global_vault, &global_recipients, &global_registry);

    let project = project_root.map(|root| {
        let project_vault = paths::secrets::project_vault_path(root);
        let project_recipients = paths::secrets::project_recipients_path(root);
        let project_registry = paths::secrets::project_registry_path(root);
        vault::check_status(&project_vault, &project_recipients, &project_registry)
    });

    let identity_source = identity::get_identity_source();

    // Get recipient key (public key) if identity exists - uses has_identity() to avoid Touch ID
    let recipient_key = if identity::has_identity() {
        identity::get_recipient().ok()
    } else {
        None
    };

    Ok(SecretsStatus {
        global,
        project,
        identity_source,
        recipient_key,
    })
}

// =============================================================================
// Secret Management
// =============================================================================

/// Result of adding a secret, for callers to display as they see fit.
pub struct AddResult {
    /// The env var name mapped to this secret.
    pub env_var: String,
    /// Whether a new vault was created (first secret).
    pub created_vault: bool,
}

/// Add a secret to the vault.
///
/// - `global = true`: add to global vault (~/.patina/)
/// - `global = false`: add to project vault (.patina/) if exists, else global
pub fn add_secret(
    name: &str,
    value: &str,
    env: Option<&str>,
    global: bool,
    project_root: Option<&Path>,
) -> Result<AddResult> {
    // Validate name
    if !registry::is_valid_secret_name(name) {
        bail!(
            "Invalid secret name '{}'. Use lowercase letters, digits, and hyphens (e.g., 'github-token')",
            name
        );
    }

    // Determine env var name
    let env_var = env
        .map(|e| e.to_string())
        .unwrap_or_else(|| registry::infer_env_name(name));

    // Validate env
    if !registry::is_valid_env_name(&env_var) {
        bail!(
            "Invalid env name '{}'. Use uppercase letters, digits, and underscores (e.g., 'GITHUB_TOKEN')",
            env_var
        );
    }

    // Determine vault paths
    let (vault_path, recipients_path, registry_path) = if global {
        (
            paths::secrets::vault_path(),
            paths::secrets::recipient_path(),
            paths::secrets::registry_path(),
        )
    } else if let Some(root) = project_root {
        (
            paths::secrets::project_vault_path(root),
            paths::secrets::project_recipients_path(root),
            paths::secrets::project_registry_path(root),
        )
    } else {
        // No project, use global
        (
            paths::secrets::vault_path(),
            paths::secrets::recipient_path(),
            paths::secrets::registry_path(),
        )
    };

    // Check if vault exists, init if not
    let created_vault = if !vault_path.exists() {
        let _recipient = vault::init_vault(&vault_path, &recipients_path)?;
        true
    } else {
        false
    };

    // Load and update vault (requires decrypt → Touch ID)
    let mut vault_data = match vault::decrypt_vault(&vault_path) {
        Ok(data) => data,
        Err(e) => {
            // Identity/vault mismatch: if registry has no secrets, safe to re-init
            let reg = registry::SecretsRegistry::load_from(&registry_path).unwrap_or_default();
            if reg.list().is_empty() {
                eprintln!("⚠ Vault identity mismatch with empty registry, re-initializing");
                let _recipient = vault::init_vault(&vault_path, &recipients_path)?;
                vault::decrypt_vault(&vault_path)?
            } else {
                return Err(e.context(
                    "Vault identity mismatch. The vault was encrypted with a different key.\n\
                     Recovery: patina secrets --import-key (re-import your identity)",
                ));
            }
        }
    };
    vault_data.insert(name, value);
    vault::encrypt_vault(&vault_data, &vault_path, &recipients_path)?;

    // Update registry
    let mut reg = registry::SecretsRegistry::load_from(&registry_path)?;
    reg.insert(name, &env_var);
    reg.save_to(&registry_path)?;

    Ok(AddResult {
        env_var,
        created_vault,
    })
}

/// Remove a secret from the vault.
pub fn remove_secret(name: &str, global: bool, project_root: Option<&Path>) -> Result<()> {
    // Determine vault paths
    let (vault_path, recipients_path, registry_path) = if global {
        (
            paths::secrets::vault_path(),
            paths::secrets::recipient_path(),
            paths::secrets::registry_path(),
        )
    } else if let Some(root) = project_root {
        (
            paths::secrets::project_vault_path(root),
            paths::secrets::project_recipients_path(root),
            paths::secrets::project_registry_path(root),
        )
    } else {
        bail!("No project root provided and --global not specified");
    };

    if !vault_path.exists() {
        bail!("Vault not found");
    }

    // Load and update vault
    let mut vault_data = vault::decrypt_vault(&vault_path)?;
    if vault_data.remove(name).is_none() {
        bail!("Secret '{}' not found in vault", name);
    }
    vault::encrypt_vault(&vault_data, &vault_path, &recipients_path)?;

    // Update registry
    let mut reg = registry::SecretsRegistry::load_from(&registry_path)?;
    reg.remove(name);
    reg.save_to(&registry_path)?;

    println!("✓ Removed {}", name);

    Ok(())
}

// =============================================================================
// Execution
// =============================================================================

/// Run a command with secrets injected as environment variables.
pub fn run_with_secrets(project_root: Option<&Path>, command: &[String]) -> Result<i32> {
    if command.is_empty() {
        bail!("No command provided");
    }

    // Load secrets with session caching
    let secrets = session::get_secrets_with_cache(|| load_secrets_via_ipc(project_root))?;

    if secrets.is_empty() {
        println!("No secrets to inject.");
    } else {
        println!("✓ Injecting {} secrets", secrets.len());

        // Build environment with secrets
        let mut cmd = Command::new(&command[0]);
        cmd.args(&command[1..]);

        // Inherit current environment
        cmd.envs(std::env::vars());

        // Add secrets as env vars
        for (env_var, value) in &secrets {
            cmd.env(env_var, value);
        }

        cmd.stdin(Stdio::inherit());
        cmd.stdout(Stdio::inherit());
        cmd.stderr(Stdio::inherit());

        let status = cmd.status()?;
        return Ok(status.code().unwrap_or(1));
    }

    // No secrets, just run command
    let status = Command::new(&command[0]).args(&command[1..]).status()?;

    Ok(status.code().unwrap_or(1))
}

/// Run a command on a remote host via SSH with secrets injected via stdin.
///
/// Secrets are piped as `export` statements to a remote `bash -s` shell,
/// so they never appear in process arguments (invisible to `ps auxe`).
pub fn run_with_secrets_ssh(
    project_root: Option<&Path>,
    host: &str,
    command: &[String],
) -> Result<i32> {
    if command.is_empty() {
        bail!("No command provided");
    }

    // Load secrets with session caching
    let secrets = session::get_secrets_with_cache(|| load_secrets_via_ipc(project_root))?;

    // Build stdin script: export secrets then exec the user's command.
    // exec replaces the shell so secrets don't linger in the process tree.
    let mut stdin_script = String::new();
    for (env_var, value) in &secrets {
        let escaped_value = value.replace('\'', "'\\''");
        stdin_script.push_str(&format!("export {}='{}'\n", env_var, escaped_value));
    }
    stdin_script.push_str(&format!("exec {}\n", shell_join(command)));

    println!("✓ Injecting {} secrets via SSH (stdin)", secrets.len());

    // Pipe secrets via stdin to bash -s on the remote host.
    // bash -s reads commands from stdin — secrets never touch argv.
    let mut child = Command::new("ssh")
        .arg(host)
        .arg("bash")
        .arg("-s")
        .stdin(Stdio::piped())
        .spawn()?;

    if let Some(mut stdin) = child.stdin.take() {
        stdin.write_all(stdin_script.as_bytes())?;
        // Drop closes the pipe, sending EOF to remote bash
    }

    let status = child.wait()?;

    Ok(status.code().unwrap_or(1))
}

/// Join command arguments with proper shell quoting.
/// Arguments containing shell metacharacters are single-quoted.
fn shell_join(args: &[String]) -> String {
    args.iter()
        .map(|arg| {
            if arg.is_empty()
                || arg
                    .contains(|c: char| c.is_whitespace() || "\"'\\$`!#&|;(){}[]<>?*~".contains(c))
            {
                format!("'{}'", arg.replace('\'', "'\\''"))
            } else {
                arg.clone()
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

// =============================================================================
// Session Management
// =============================================================================

/// Clear the session cache (lock).
pub fn lock_session() -> Result<()> {
    if session::clear_cache()? {
        println!("✓ Session cache cleared");
    } else {
        println!("Session cache not active (serve not running)");
    }
    Ok(())
}

// =============================================================================
// Identity Management
// =============================================================================

/// Export the identity (for backup, zeroized on drop).
pub fn export_identity() -> Result<zeroize::Zeroizing<String>> {
    identity::export_identity()
}

/// Import an identity (for new machine setup).
pub fn import_identity(identity_str: &str) -> Result<String> {
    identity::import_identity(identity_str)
}

/// Reset identity (remove from Keychain).
///
/// Warning: This deletes your private key. Make sure you have a backup!
pub fn reset_identity() -> Result<()> {
    keychain::delete_identity()
}

// =============================================================================
// Recipient Management
// =============================================================================

/// Add a recipient to the project vault.
pub fn add_recipient(project_root: &Path, recipient_key: &str) -> Result<()> {
    if !recipients::is_valid_age_recipient(recipient_key) {
        bail!("Invalid age recipient. Expected age1...");
    }

    let recipients_path = paths::secrets::project_recipients_path(project_root);
    let vault_path = paths::secrets::project_vault_path(project_root);

    if !vault_path.exists() {
        bail!("Project vault not found. Add a secret first.");
    }

    // Load existing recipients
    let mut recipient_list = recipients::load_recipients(&recipients_path)?;

    // Check for duplicates
    if recipient_list.contains(&recipient_key.to_string()) {
        bail!("Recipient already exists");
    }

    recipient_list.push(recipient_key.to_string());

    // Save updated recipients
    recipients::save_recipients(&recipients_path, &recipient_list)?;

    // Re-encrypt vault for all recipients
    let vault_data = vault::decrypt_vault(&vault_path)?;
    vault::encrypt_vault(&vault_data, &vault_path, &recipients_path)?;

    println!(
        "✓ Re-encrypted vault for {} recipients",
        recipient_list.len()
    );

    Ok(())
}

/// Remove a recipient from the project vault.
pub fn remove_recipient(project_root: &Path, recipient_key: &str) -> Result<()> {
    let recipients_path = paths::secrets::project_recipients_path(project_root);
    let vault_path = paths::secrets::project_vault_path(project_root);

    if !vault_path.exists() {
        bail!("Project vault not found");
    }

    // Load existing recipients
    let mut recipient_list = recipients::load_recipients(&recipients_path)?;

    // Find and remove
    let original_len = recipient_list.len();
    recipient_list.retain(|r| r != recipient_key);

    if recipient_list.len() == original_len {
        bail!("Recipient not found");
    }

    if recipient_list.is_empty() {
        bail!("Cannot remove last recipient");
    }

    // Save updated recipients
    recipients::save_recipients(&recipients_path, &recipient_list)?;

    // Re-encrypt vault for remaining recipients
    let vault_data = vault::decrypt_vault(&vault_path)?;
    vault::encrypt_vault(&vault_data, &vault_path, &recipients_path)?;

    println!(
        "✓ Re-encrypted vault for {} recipients",
        recipient_list.len()
    );

    Ok(())
}

/// List recipients for the project vault.
pub fn list_recipients(project_root: &Path) -> Result<Vec<String>> {
    let recipients_path = paths::secrets::project_recipients_path(project_root);
    recipients::load_recipients(&recipients_path)
}

// =============================================================================
// Single-Secret Accessor
// =============================================================================

/// Get a single secret from the global vault only.
///
/// Decrypts ONLY `~/.patina/vault.age` — never touches project vaults.
/// Checks session cache first (via `patina serve` if running) to avoid
/// redundant Touch ID prompts.
///
/// Returns `Ok(None)` if the secret doesn't exist or the vault doesn't exist.
/// Returns `Err` only on actual decryption failure (missing identity, corrupted vault).
pub fn get_global_secret(name: &str) -> Result<Option<String>> {
    // 1. Check session cache first (no Touch ID)
    if let Some(cached) = session::get_cached_secrets() {
        return Ok(cached.get(name).cloned());
    }

    // 2. Cache miss — decrypt global vault only
    let global_path = paths::secrets::vault_path();
    if !global_path.exists() {
        return Ok(None);
    }

    let vault_data = vault::decrypt_vault(&global_path)?;
    Ok(vault_data.values.get(name).cloned())
}

// =============================================================================
// Helper Functions
// =============================================================================

/// Load env-var-to-value secrets map from Mother secrets authority.
fn load_secrets_via_ipc(project_root: Option<&Path>) -> Result<HashMap<String, String>> {
    let client = crate::mother::control_plane_client();
    let request = BuiltinChildRequest::new(
        BuiltinChild::SecretsAuthority,
        BuiltinChildAction::SecretsDispatch(SecretsDispatchRequest {
            operation: SecretsAuthorityOperation::LoadSecretsEnvMap {
                project_root: project_root.map(|root| root.display().to_string()),
            },
        }),
    );
    let response = client.child_action_typed(&request).map_err(|error| {
        anyhow::anyhow!(
            "secrets-authority unavailable via Mother (start with `patina mother start`): {}",
            error
        )
    })?;
    let payload = match response.result {
        BuiltinChildResult::Dispatch { payload } => payload,
        other => bail!(
            "Unexpected typed response from secrets-authority: {:?}",
            other
        ),
    };

    let secrets = payload
        .get("secrets")
        .cloned()
        .unwrap_or_else(|| serde_json::json!({}));
    Ok(serde_json::from_value(secrets).unwrap_or_default())
}

/// Prompt for a secret value interactively (masked, no echo).
pub fn prompt_for_value(name: &str) -> Result<String> {
    let term = console::Term::stderr();
    let value = term
        .read_secure_line()
        .map_err(|e| anyhow::anyhow!("Failed to read secret for '{}': {}", name, e))?;

    Ok(value.trim().to_string())
}

#[cfg(test)]
mod tests {
    use crate::paths;

    #[test]
    fn test_registry_path() {
        let _guard = crate::test_support::env_test_mutex()
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        let temp = tempfile::TempDir::new().unwrap();
        let patina_home = temp.path().join("patina-home");
        std::fs::create_dir_all(&patina_home).unwrap();
        let old = std::env::var_os("PATINA_HOME");
        unsafe {
            std::env::set_var("PATINA_HOME", &patina_home);
        }

        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let path = paths::secrets::registry_path();
            assert_eq!(path, patina_home.join("secrets.toml"));
        }));

        match old {
            Some(value) => unsafe {
                std::env::set_var("PATINA_HOME", value);
            },
            None => unsafe {
                std::env::remove_var("PATINA_HOME");
            },
        }

        if let Err(panic) = result {
            std::panic::resume_unwind(panic);
        }
    }

    #[test]
    fn test_get_global_secret_no_vault() {
        // When no vault exists, get_global_secret returns Ok(None)
        // This test relies on the test environment not having ~/.patina/vault.age
        // or having a session cache running. In CI, neither exists.
        // If a real vault exists on the dev machine, this still passes because
        // "nonexistent-secret-name" won't be in it.
        let result = super::get_global_secret("nonexistent-test-secret-xyzzy");
        // Either Ok(None) — secret not found — or Err from vault issues,
        // but never panics
        match result {
            Ok(None) => {} // expected in most environments
            Ok(Some(_)) => panic!("unexpected secret found for random test name"),
            Err(_) => {} // acceptable: vault exists but identity unavailable in CI
        }
    }
}
