use super::{encrypted_file, keychain, recipients, storage};
use age::secrecy::ExposeSecret;
use age::x25519;
use anyhow::{bail, Context, Result};
use std::str::FromStr;
use zeroize::Zeroizing;

pub const IDENTITY_ENV_VAR: &str = "PATINA_IDENTITY";

fn log_debug(msg: &str) {
    if std::env::var("PATINA_LOG").is_ok() {
        eprintln!("[DEBUG secrets::identity] {}", msg);
    }
}

pub fn get_identity() -> Result<x25519::Identity> {
    let identity_str = get_identity_string()?;

    x25519::Identity::from_str(&identity_str)
        .map_err(|e| anyhow::anyhow!("Invalid age identity: {}", e))
}

pub fn get_identity_string() -> Result<Zeroizing<String>> {
    if let Ok(identity) = std::env::var(IDENTITY_ENV_VAR) {
        if !identity.is_empty() {
            log_debug("source = PATINA_IDENTITY (env var)");
            return Ok(Zeroizing::new(identity));
        }
        log_debug("PATINA_IDENTITY set but empty, falling back to storage");
    }

    log_debug("delegating to storage orchestrator");
    Ok(Zeroizing::new(storage::get_identity()?))
}

pub fn get_recipient() -> Result<String> {
    let identity = get_identity()?;
    Ok(identity.to_public().to_string())
}

pub fn generate_identity() -> (Zeroizing<String>, String) {
    let identity = x25519::Identity::generate();
    let recipient = identity.to_public();
    (
        Zeroizing::new(identity.to_string().expose_secret().to_string()),
        recipient.to_string(),
    )
}

pub fn store_identity(identity: &str) -> Result<()> {
    if !recipients::is_valid_age_identity(identity) {
        bail!("Invalid age identity format. Expected AGE-SECRET-KEY-1...");
    }
    storage::store_identity(identity)
}

pub fn import_identity(identity: &str) -> Result<String> {
    let identity = identity.trim();
    if !recipients::is_valid_age_identity(identity) {
        bail!("Invalid age identity format. Expected AGE-SECRET-KEY-1...");
    }

    let parsed = x25519::Identity::from_str(identity)
        .map_err(|e| anyhow::anyhow!("Invalid age identity: {}", e))?;
    let recipient = parsed.to_public().to_string();

    store_identity(identity).context("Failed to store identity")?;
    Ok(recipient)
}

pub fn export_identity() -> Result<Zeroizing<String>> {
    get_identity_string()
}

pub fn has_identity() -> bool {
    if let Ok(identity) = std::env::var(IDENTITY_ENV_VAR) {
        if !identity.is_empty() {
            return true;
        }
    }
    storage::has_identity()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IdentitySource {
    Environment,
    EncryptedFile,
    Keychain,
}

impl std::fmt::Display for IdentitySource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            IdentitySource::Environment => write!(f, "PATINA_IDENTITY"),
            IdentitySource::EncryptedFile => write!(f, "Encrypted File"),
            IdentitySource::Keychain => write!(f, "macOS Keychain"),
        }
    }
}

pub fn get_identity_source() -> Option<IdentitySource> {
    if let Ok(identity) = std::env::var(IDENTITY_ENV_VAR) {
        if !identity.is_empty() {
            return Some(IdentitySource::Environment);
        }
    }

    if encrypted_file::has_identity() {
        return Some(IdentitySource::EncryptedFile);
    }
    if keychain::has_identity() {
        return Some(IdentitySource::Keychain);
    }

    None
}
