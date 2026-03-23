use super::identity;
use super::recipients as recipients_mod;
use super::registry;
use age::armor::{ArmoredReader, ArmoredWriter, Format};
use age::x25519;
use anyhow::{bail, Context, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::io::{Read, Write};
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use std::str::FromStr;

const VAULT_VERSION: u32 = 1;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VaultMeta {
    pub version: u32,
    pub created_at: DateTime<Utc>,
    pub modified_at: DateTime<Utc>,
}

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
    pub fn new() -> Self {
        Self::default()
    }

    pub fn insert(&mut self, name: &str, value: &str) {
        self.values.insert(name.to_string(), value.to_string());
        self.meta.modified_at = Utc::now();
    }

    pub fn remove(&mut self, name: &str) -> Option<String> {
        let removed = self.values.remove(name);
        if removed.is_some() {
            self.meta.modified_at = Utc::now();
        }
        removed
    }
}

#[derive(Debug, Clone)]
pub struct VaultStatus {
    pub exists: bool,
    pub secret_count: usize,
    pub secret_names: Vec<String>,
    pub recipient_count: usize,
    pub recipients: Vec<String>,
}

pub fn check_status(
    vault_path: &Path,
    recipients_path: &Path,
    registry_path: &Path,
) -> VaultStatus {
    let exists = vault_path.exists();

    let recipients = if recipients_path.exists() {
        recipients_mod::load_recipients(recipients_path).unwrap_or_default()
    } else {
        Vec::new()
    };
    let recipient_count = recipients.len();

    let registry = registry::SecretsRegistry::load_from(registry_path).unwrap_or_default();
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

pub fn decrypt_vault(vault_path: &Path) -> Result<Vault> {
    if !vault_path.exists() {
        bail!("Vault not found: {:?}", vault_path);
    }

    let identity = identity::get_identity().context("No identity available for decryption")?;
    let encrypted =
        fs::read(vault_path).with_context(|| format!("Failed to read vault: {:?}", vault_path))?;

    let decrypted = decrypt_bytes(&encrypted, &identity)?;
    let vault: Vault = toml::from_str(&decrypted).context("Failed to parse decrypted vault")?;

    if vault.meta.version != VAULT_VERSION {
        bail!(
            "Unsupported vault version {}. Expected {}.",
            vault.meta.version,
            VAULT_VERSION
        );
    }

    Ok(vault)
}

pub fn encrypt_vault(vault: &Vault, vault_path: &Path, recipients_path: &Path) -> Result<()> {
    let recipient_strings = recipients_mod::load_recipients(recipients_path)?;
    if recipient_strings.is_empty() {
        bail!("No recipients found in {:?}", recipients_path);
    }

    let recipients: Vec<x25519::Recipient> = recipient_strings
        .iter()
        .map(|r| {
            x25519::Recipient::from_str(r)
                .map_err(|e| anyhow::anyhow!("Invalid recipient '{}': {}", r, e))
        })
        .collect::<Result<Vec<_>>>()?;

    let content = toml::to_string_pretty(vault)?;
    let encrypted = encrypt_bytes(content.as_bytes(), &recipients)?;

    if let Some(parent) = vault_path.parent() {
        fs::create_dir_all(parent)?;
    }

    fs::write(vault_path, encrypted)
        .with_context(|| format!("Failed to write vault: {:?}", vault_path))?;

    #[cfg(unix)]
    fs::set_permissions(vault_path, fs::Permissions::from_mode(0o600))?;

    Ok(())
}

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

fn decrypt_bytes(data: &[u8], identity: &x25519::Identity) -> Result<String> {
    let armor_reader = ArmoredReader::new(data);
    let decryptor = age::Decryptor::new(armor_reader)?;

    let mut decrypted = Vec::new();
    let mut reader = decryptor.decrypt(std::iter::once(identity as &dyn age::Identity))?;
    reader.read_to_end(&mut decrypted)?;

    String::from_utf8(decrypted).context("Decrypted content is not valid UTF-8")
}

pub fn init_vault(vault_path: &Path, recipients_path: &Path) -> Result<String> {
    let recipient = if identity::has_identity() {
        identity::get_recipient()?
    } else {
        let (identity_str, recipient) = identity::generate_identity();
        identity::store_identity(&identity_str)?;
        recipient
    };

    let vault = Vault::new();
    recipients_mod::save_recipients(recipients_path, std::slice::from_ref(&recipient))?;
    encrypt_vault(&vault, vault_path, recipients_path)?;

    Ok(recipient)
}
