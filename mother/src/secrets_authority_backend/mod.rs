mod encrypted_file;
mod identity;
mod keychain;
mod recipients;
mod registry;
mod session;
mod storage;
mod vault;

pub use self::identity::IdentitySource;
pub use self::vault::VaultStatus;

use crate::secrets_authority_api as api;
use crate::secrets_paths as paths;
use anyhow::{bail, Result};
use std::collections::HashMap;
use std::path::Path;

#[derive(Debug)]
pub struct SecretsStatus {
    pub global: VaultStatus,
    pub project: Option<VaultStatus>,
    pub identity_source: Option<IdentitySource>,
    pub recipient_key: Option<String>,
}

pub struct AddResult {
    pub env_var: String,
    pub created_vault: bool,
}

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

pub fn add_secret(
    name: &str,
    value: &str,
    env: Option<&str>,
    global: bool,
    project_root: Option<&Path>,
) -> Result<AddResult> {
    if !registry::is_valid_secret_name(name) {
        bail!(
            "Invalid secret name '{}'. Use lowercase letters, digits, and hyphens (e.g., 'github-token')",
            name
        );
    }

    let env_var = env
        .map(|e| e.to_string())
        .unwrap_or_else(|| registry::infer_env_name(name));

    if !registry::is_valid_env_name(&env_var) {
        bail!(
            "Invalid env name '{}'. Use uppercase letters, digits, and underscores (e.g., 'GITHUB_TOKEN')",
            env_var
        );
    }

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
        (
            paths::secrets::vault_path(),
            paths::secrets::recipient_path(),
            paths::secrets::registry_path(),
        )
    };

    let created_vault = if !vault_path.exists() {
        let _recipient = vault::init_vault(&vault_path, &recipients_path)?;
        true
    } else {
        false
    };

    let mut vault_data = match vault::decrypt_vault(&vault_path) {
        Ok(data) => data,
        Err(e) => {
            // Identity/vault mismatch: if registry has no secrets, safe to re-init
            let reg = registry::SecretsRegistry::load_from(&registry_path).unwrap_or_default();
            if reg.list().is_empty() {
                tracing::warn!("Vault decrypt failed with empty registry, re-initializing");
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

    let mut reg = registry::SecretsRegistry::load_from(&registry_path)?;
    reg.insert(name, &env_var);
    reg.save_to(&registry_path)?;

    Ok(AddResult {
        env_var,
        created_vault,
    })
}

pub fn remove_secret(name: &str, global: bool, project_root: Option<&Path>) -> Result<()> {
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

    let mut vault_data = vault::decrypt_vault(&vault_path)?;
    if vault_data.remove(name).is_none() {
        bail!("Secret '{}' not found in vault", name);
    }
    vault::encrypt_vault(&vault_data, &vault_path, &recipients_path)?;

    let mut reg = registry::SecretsRegistry::load_from(&registry_path)?;
    reg.remove(name);
    reg.save_to(&registry_path)?;

    Ok(())
}

pub fn lock_session() -> Result<()> {
    let _ = session::clear_cache()?;
    Ok(())
}

pub fn export_identity() -> Result<zeroize::Zeroizing<String>> {
    identity::export_identity()
}

pub fn import_identity(identity_str: &str) -> Result<String> {
    identity::import_identity(identity_str)
}

pub fn reset_identity() -> Result<()> {
    keychain::delete_identity()
}

pub fn add_recipient(project_root: &Path, recipient_key: &str) -> Result<()> {
    if !recipients::is_valid_age_recipient(recipient_key) {
        bail!("Invalid age recipient. Expected age1...");
    }

    let recipients_path = paths::secrets::project_recipients_path(project_root);
    let vault_path = paths::secrets::project_vault_path(project_root);
    if !vault_path.exists() {
        bail!("Project vault not found. Add a secret first.");
    }

    let mut recipient_list = recipients::load_recipients(&recipients_path)?;
    if recipient_list.contains(&recipient_key.to_string()) {
        bail!("Recipient already exists");
    }
    recipient_list.push(recipient_key.to_string());
    recipients::save_recipients(&recipients_path, &recipient_list)?;

    let vault_data = vault::decrypt_vault(&vault_path)?;
    vault::encrypt_vault(&vault_data, &vault_path, &recipients_path)?;
    Ok(())
}

pub fn remove_recipient(project_root: &Path, recipient_key: &str) -> Result<()> {
    let recipients_path = paths::secrets::project_recipients_path(project_root);
    let vault_path = paths::secrets::project_vault_path(project_root);
    if !vault_path.exists() {
        bail!("Project vault not found");
    }

    let mut recipient_list = recipients::load_recipients(&recipients_path)?;
    let original_len = recipient_list.len();
    recipient_list.retain(|r| r != recipient_key);

    if recipient_list.len() == original_len {
        bail!("Recipient not found");
    }
    if recipient_list.is_empty() {
        bail!("Cannot remove last recipient");
    }

    recipients::save_recipients(&recipients_path, &recipient_list)?;
    let vault_data = vault::decrypt_vault(&vault_path)?;
    vault::encrypt_vault(&vault_data, &vault_path, &recipients_path)?;
    Ok(())
}

pub fn list_recipients(project_root: &Path) -> Result<Vec<String>> {
    let recipients_path = paths::secrets::project_recipients_path(project_root);
    recipients::load_recipients(&recipients_path)
}

pub fn get_global_secret(name: &str) -> Result<Option<String>> {
    if let Some(cached) = session::get_cached_secrets() {
        return Ok(cached.get(name).cloned());
    }

    let global_path = paths::secrets::vault_path();
    if !global_path.exists() {
        return Ok(None);
    }

    let vault_data = vault::decrypt_vault(&global_path)?;
    Ok(vault_data.values.get(name).cloned())
}

pub fn load_secrets_env_map(project_root: Option<&Path>) -> Result<HashMap<String, String>> {
    let mut env_map = HashMap::new();

    let global_vault_path = paths::secrets::vault_path();
    let global_registry_path = paths::secrets::registry_path();
    if global_vault_path.exists() {
        let vault_data = vault::decrypt_vault(&global_vault_path)?;
        let registry =
            registry::SecretsRegistry::load_from(&global_registry_path).unwrap_or_default();
        for (name, value) in &vault_data.values {
            if let Some(env_var) = registry.get_env(name) {
                env_map.insert(env_var.to_string(), value.clone());
            }
        }
    }

    if let Some(root) = project_root {
        let project_vault_path = paths::secrets::project_vault_path(root);
        let project_registry_path = paths::secrets::project_registry_path(root);
        if project_vault_path.exists() {
            let vault_data = vault::decrypt_vault(&project_vault_path)?;
            let registry =
                registry::SecretsRegistry::load_from(&project_registry_path).unwrap_or_default();
            for (name, value) in &vault_data.values {
                if let Some(env_var) = registry.get_env(name) {
                    env_map.insert(env_var.to_string(), value.clone());
                }
            }
        }
    }

    Ok(env_map)
}

pub struct MotherSecretsAuthorityBackend;

impl api::SecretsAuthorityBackend for MotherSecretsAuthorityBackend {
    fn get_global_secret(&self, name: String) -> anyhow::Result<Option<String>> {
        get_global_secret(&name)
    }

    fn add_secret(&self, input: api::AddSecretInput) -> anyhow::Result<api::AddSecretResult> {
        let result = add_secret(
            &input.name,
            &input.value,
            input.env_name.as_deref(),
            input.global,
            input.project_root.as_deref(),
        )?;
        Ok(api::AddSecretResult {
            env_var: result.env_var,
            created_vault: result.created_vault,
        })
    }

    fn remove_secret(&self, input: api::RemoveSecretInput) -> anyhow::Result<()> {
        remove_secret(&input.name, input.global, input.project_root.as_deref())
    }

    fn add_recipient(&self, input: api::RecipientInput) -> anyhow::Result<()> {
        add_recipient(&input.project_root, &input.key)
    }

    fn remove_recipient(&self, input: api::RecipientInput) -> anyhow::Result<()> {
        remove_recipient(&input.project_root, &input.key)
    }

    fn list_recipients(&self, project_root: std::path::PathBuf) -> anyhow::Result<Vec<String>> {
        list_recipients(&project_root)
    }

    fn status(
        &self,
        project_root: Option<std::path::PathBuf>,
    ) -> anyhow::Result<api::StatusResponse> {
        let status = check_status(project_root.as_deref())?;
        Ok(api::StatusResponse {
            identity_source: status.identity_source.map(|s| s.to_string()),
            recipient_key: status.recipient_key,
            global: api::StatusVault {
                exists: status.global.exists,
                secret_count: status.global.secret_count,
                recipient_count: status.global.recipient_count,
                secret_names: status.global.secret_names,
            },
            project: status.project.map(|p| api::StatusVault {
                exists: p.exists,
                secret_count: p.secret_count,
                recipient_count: p.recipient_count,
                secret_names: p.secret_names,
            }),
        })
    }

    fn lock_session(&self) -> anyhow::Result<()> {
        lock_session()
    }

    fn export_identity(&self) -> anyhow::Result<String> {
        Ok(export_identity()?.to_string())
    }

    fn import_identity(&self, identity: String) -> anyhow::Result<String> {
        import_identity(&identity)
    }

    fn reset_identity(&self) -> anyhow::Result<()> {
        reset_identity()
    }

    fn setup_claude_token(
        &self,
        token: String,
        project_root: Option<std::path::PathBuf>,
    ) -> anyhow::Result<api::SetupClaudeTokenResult> {
        let replacing = matches!(get_global_secret("claude-oauth"), Ok(Some(_)));
        let result = add_secret(
            "claude-oauth",
            &token,
            Some("CLAUDE_CODE_OAUTH_TOKEN"),
            true,
            project_root.as_deref(),
        )?;
        if let Ok(key) = export_identity() {
            let _ = import_identity(&key);
        }
        Ok(api::SetupClaudeTokenResult {
            replacing,
            created_vault: result.created_vault,
            env_var: result.env_var,
        })
    }

    fn load_secrets_env_map(
        &self,
        project_root: Option<std::path::PathBuf>,
    ) -> anyhow::Result<HashMap<String, String>> {
        load_secrets_env_map(project_root.as_deref())
    }
}
