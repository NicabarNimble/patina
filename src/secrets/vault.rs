//! Age-encrypted vault operations.
//!
//! Handles encryption/decryption of the secrets vault.
//!
//! Vault format (decrypted TOML):
//! ```toml
//! [meta]
//! version = 1
//! created_at = "2024-12-24T12:00:00Z"
//! modified_at = "2024-12-24T14:30:00Z"
//!
//! [values]
//! my-api = "xxx"
//! db-pass = "xxx"
//! ```

use crate::secrets::identity;
use crate::secrets::recipients as recipients_mod;
use age::armor::{ArmoredReader, ArmoredWriter, Format};
use age::x25519;
use anyhow::{bail, Context, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::io::{Read, Write};
use std::path::Path;
use std::str::FromStr;

/// Vault format version.
const VAULT_VERSION: u32 = 1;

/// Vault metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VaultMeta {
    pub version: u32,
    pub created_at: DateTime<Utc>,
    pub modified_at: DateTime<Utc>,
}

/// The decrypted vault contents.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Vault {
    pub meta: VaultMeta,
    #[serde(default)]
    pub values: HashMap<String, String>,
}

impl Default for Vault {
    fn default() -> Self {
        let now = Utc::now();
        Self {
            meta: VaultMeta {
                version: VAULT_VERSION,
                created_at: now,
                modified_at: now,
            },
            values: HashMap::new(),
        }
    }
}

impl Vault {
    /// Create a new empty vault.
    pub fn new() -> Self {
        Self::default()
    }

    /// Insert a secret value.
    pub fn insert(&mut self, name: &str, value: &str) {
        self.values.insert(name.to_string(), value.to_string());
        self.meta.modified_at = Utc::now();
    }

    /// Remove a secret value.
    pub fn remove(&mut self, name: &str) -> Option<String> {
        let removed = self.values.remove(name);
        if removed.is_some() {
            self.meta.modified_at = Utc::now();
        }
        removed
    }
}

/// Vault status for display.
#[derive(Debug, Clone)]
pub struct VaultStatus {
    pub exists: bool,
    pub secret_count: usize,
    pub secret_names: Vec<String>,
    pub recipient_count: usize,
    pub recipients: Vec<String>,
}

/// Check vault status without decrypting.
///
/// Gets secret names from registry (no decryption needed).
/// Gets recipient keys from recipients file.
pub fn check_status(
    vault_path: &Path,
    recipients_path: &Path,
    registry_path: &Path,
) -> VaultStatus {
    let exists = vault_path.exists();

    // Load recipients
    let recipients = if recipients_path.exists() {
        recipients_mod::load_recipients(recipients_path).unwrap_or_default()
    } else {
        Vec::new()
    };
    let recipient_count = recipients.len();

    // Load secret names from registry (no decryption needed)
    let registry =
        crate::secrets::registry::SecretsRegistry::load_from(registry_path).unwrap_or_default();
    let secret_names: Vec<String> = registry.list().iter().map(|s| s.to_string()).collect();
    let secret_count = secret_names.len();

    VaultStatus {
        exists,
        secret_count,
        secret_names,
        recipient_count,
        recipients,
    }
}

/// Decrypt a vault file.
pub fn decrypt_vault(vault_path: &Path) -> Result<Vault> {
    if !vault_path.exists() {
        bail!("Vault not found: {:?}", vault_path);
    }

    // Get identity for decryption
    let identity = identity::get_identity().context("No identity available for decryption")?;

    // Read encrypted file
    let encrypted =
        fs::read(vault_path).with_context(|| format!("Failed to read vault: {:?}", vault_path))?;

    // Decrypt
    let decrypted = decrypt_bytes(&encrypted, &identity)?;

    // Parse TOML
    let vault: Vault = toml::from_str(&decrypted).context("Failed to parse decrypted vault")?;

    // Version check
    if vault.meta.version != VAULT_VERSION {
        bail!(
            "Unsupported vault version {}. Expected {}.",
            vault.meta.version,
            VAULT_VERSION
        );
    }

    Ok(vault)
}

/// Encrypt and save a vault.
pub fn encrypt_vault(vault: &Vault, vault_path: &Path, recipients_path: &Path) -> Result<()> {
    // Load recipients
    let recipient_strings = recipients_mod::load_recipients(recipients_path)?;
    if recipient_strings.is_empty() {
        bail!("No recipients found in {:?}", recipients_path);
    }

    // Parse recipients
    let recipients: Vec<x25519::Recipient> = recipient_strings
        .iter()
        .map(|r| {
            x25519::Recipient::from_str(r)
                .map_err(|e| anyhow::anyhow!("Invalid recipient '{}': {}", r, e))
        })
        .collect::<Result<Vec<_>>>()?;

    // Serialize vault to TOML
    let content = toml::to_string_pretty(vault)?;

    // Encrypt
    let encrypted = encrypt_bytes(content.as_bytes(), &recipients)?;

    // Ensure parent directory exists
    if let Some(parent) = vault_path.parent() {
        fs::create_dir_all(parent)?;
    }

    // Write
    fs::write(vault_path, encrypted)
        .with_context(|| format!("Failed to write vault: {:?}", vault_path))?;

    Ok(())
}

/// Encrypt bytes for multiple recipients.
fn encrypt_bytes(data: &[u8], recipients: &[x25519::Recipient]) -> Result<Vec<u8>> {
    let encryptor =
        age::Encryptor::with_recipients(recipients.iter().map(|r| r as &dyn age::Recipient))
            .expect("No recipients provided");

    let mut encrypted = Vec::new();
    {
        let armor_writer = ArmoredWriter::wrap_output(&mut encrypted, Format::AsciiArmor)?;
        let mut writer = encryptor.wrap_output(armor_writer)?;
        writer.write_all(data)?;
        writer.finish()?.finish()?;
    }

    Ok(encrypted)
}

/// Decrypt bytes with an identity.
fn decrypt_bytes(data: &[u8], identity: &x25519::Identity) -> Result<String> {
    let armor_reader = ArmoredReader::new(data);

    let decryptor = age::Decryptor::new(armor_reader)?;

    let mut decrypted = Vec::new();
    let mut reader = decryptor.decrypt(std::iter::once(identity as &dyn age::Identity))?;
    reader.read_to_end(&mut decrypted)?;

    String::from_utf8(decrypted).context("Decrypted content is not valid UTF-8")
}

/// Initialize a new vault with an identity.
///
/// Reuses existing identity from Keychain if available, otherwise generates new.
/// Returns the recipient (public key) for the vault.
pub fn init_vault(vault_path: &Path, recipients_path: &Path) -> Result<String> {
    // Check if identity already exists
    let recipient = if identity::has_identity() {
        // Reuse existing identity
        identity::get_recipient()?
    } else {
        // Generate new identity
        let (identity_str, recipient) = identity::generate_identity();

        // Store identity in Keychain
        identity::store_identity(&identity_str)?;

        println!("✓ Generated encryption key");
        println!("✓ Stored in macOS Keychain (Touch ID protected)");

        recipient
    };

    // Create empty vault
    let vault = Vault::new();

    // Save recipient to file
    recipients_mod::save_recipients(recipients_path, std::slice::from_ref(&recipient))?;

    // Encrypt and save vault
    encrypt_vault(&vault, vault_path, recipients_path)?;

    Ok(recipient)
}

/// Load all secrets from both global and project vaults.
///
/// Project vault overrides global vault on conflicts.
pub fn load_merged_secrets(
    global_vault_path: Option<&Path>,
    project_vault_path: Option<&Path>,
) -> Result<HashMap<String, String>> {
    let mut secrets = HashMap::new();

    // Load global first
    if let Some(path) = global_vault_path {
        if path.exists() {
            let vault = decrypt_vault(path)?;
            secrets.extend(vault.values);
        }
    }

    // Project overrides global
    if let Some(path) = project_vault_path {
        if path.exists() {
            let vault = decrypt_vault(path)?;
            secrets.extend(vault.values);
        }
    }

    Ok(secrets)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vault_default() {
        let vault = Vault::new();
        assert_eq!(vault.meta.version, VAULT_VERSION);
        assert!(vault.values.is_empty());
    }

    #[test]
    fn test_vault_operations() {
        let mut vault = Vault::new();

        vault.insert("test-secret", "secret-value");
        assert_eq!(
            vault.values.get("test-secret").map(|s| s.as_str()),
            Some("secret-value")
        );
        assert!(vault.values.contains_key("test-secret"));
        assert_eq!(vault.values.len(), 1);

        vault.remove("test-secret");
        assert!(vault.values.get("test-secret").is_none());
        assert!(vault.values.is_empty());
    }

    #[test]
    fn test_vault_serialization() {
        let mut vault = Vault::new();
        vault.insert("github-token", "ghp_test123");
        vault.insert("openai-key", "sk-test456");

        let toml = toml::to_string_pretty(&vault).unwrap();
        assert!(toml.contains("[meta]"));
        assert!(toml.contains("[values]"));
        assert!(toml.contains("github-token"));

        let parsed: Vault = toml::from_str(&toml).unwrap();
        assert_eq!(
            parsed.values.get("github-token").map(|s| s.as_str()),
            Some("ghp_test123")
        );
        assert_eq!(
            parsed.values.get("openai-key").map(|s| s.as_str()),
            Some("sk-test456")
        );
    }
}
