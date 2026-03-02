//! Identity resolution for secrets decryption.
//!
//! Resolution order:
//! 1. PATINA_IDENTITY env var (for CI/headless)
//! 2. Storage orchestrator (encrypted file → Keychain with auto-migration)

use crate::secrets::{encrypted_file, keychain, recipients, storage};
use age::secrecy::ExposeSecret;
use age::x25519;
use anyhow::{bail, Context, Result};
use std::str::FromStr;
use zeroize::Zeroizing;

/// Environment variable for identity (CI/headless path).
pub const IDENTITY_ENV_VAR: &str = "PATINA_IDENTITY";

/// Debug logging for secrets module (Phase 0 observability)
fn log_debug(msg: &str) {
    if std::env::var("PATINA_LOG").is_ok() {
        eprintln!("[DEBUG secrets::identity] {}", msg);
    }
}

/// Get the age identity for decryption.
///
/// Checks env var first (CI/headless), then Keychain (Mac with Touch ID).
pub fn get_identity() -> Result<x25519::Identity> {
    let identity_str = get_identity_string()?; // Zeroizing<String> — zeroed on drop

    x25519::Identity::from_str(&identity_str)
        .map_err(|e| anyhow::anyhow!("Invalid age identity: {}", e))
}

/// Get the identity as a string (zeroized on drop).
///
/// Resolution order:
/// 1. PATINA_IDENTITY env var (escape hatch for CI/headless)
/// 2. Storage orchestrator (handles encrypted file → Keychain with auto-migration)
pub fn get_identity_string() -> Result<Zeroizing<String>> {
    // 1. Check env first (CI/headless path, escape hatch)
    if let Ok(identity) = std::env::var(IDENTITY_ENV_VAR) {
        if !identity.is_empty() {
            log_debug("source = PATINA_IDENTITY (env var)");
            return Ok(Zeroizing::new(identity));
        }
        log_debug("PATINA_IDENTITY set but empty, falling back to storage");
    }

    // 2. Delegate to storage orchestrator
    // Storage handles: encrypted file → Keychain with auto-migration
    log_debug("delegating to storage orchestrator");
    Ok(Zeroizing::new(storage::get_identity()?))
}

/// Get the public key (recipient) for the current identity.
pub fn get_recipient() -> Result<String> {
    let identity = get_identity()?;
    Ok(identity.to_public().to_string())
}

/// Generate a new age identity.
///
/// Returns (identity_string, recipient_string). Identity is zeroized on drop.
pub fn generate_identity() -> (Zeroizing<String>, String) {
    let identity = x25519::Identity::generate();
    let recipient = identity.to_public();
    (
        Zeroizing::new(identity.to_string().expose_secret().to_string()),
        recipient.to_string(),
    )
}

/// Store an identity using dual-storage strategy.
///
/// Validates format, then delegates to storage orchestrator.
/// - macOS: Writes to BOTH Keychain and encrypted file
/// - Linux: Writes to encrypted file only
pub fn store_identity(identity: &str) -> Result<()> {
    // Validate before storing
    if !recipients::is_valid_age_identity(identity) {
        bail!("Invalid age identity format. Expected AGE-SECRET-KEY-1...");
    }

    // Delegate to storage orchestrator
    storage::store_identity(identity)
}

/// Import an identity from a string.
///
/// Validates and stores using dual-storage strategy.
pub fn import_identity(identity: &str) -> Result<String> {
    let identity = identity.trim();

    // Validate format
    if !recipients::is_valid_age_identity(identity) {
        bail!("Invalid age identity format. Expected AGE-SECRET-KEY-1...");
    }

    // Parse to validate it's a real identity and get recipient
    let parsed = x25519::Identity::from_str(identity)
        .map_err(|e| anyhow::anyhow!("Invalid age identity: {}", e))?;

    let recipient = parsed.to_public().to_string();

    // Store using dual-storage strategy (delegates to storage orchestrator)
    store_identity(identity).context("Failed to store identity")?;

    Ok(recipient)
}

/// Export the identity from storage (zeroized on drop).
///
/// Returns the identity string for backup.
pub fn export_identity() -> Result<Zeroizing<String>> {
    get_identity_string()
}

/// Check if an identity is available.
///
/// Checks env var first, then delegates to storage orchestrator.
pub fn has_identity() -> bool {
    // Check env var
    if let Ok(identity) = std::env::var(IDENTITY_ENV_VAR) {
        if !identity.is_empty() {
            return true;
        }
    }

    // Delegate to storage orchestrator
    storage::has_identity()
}

/// Identity source for display/debugging.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IdentitySource {
    /// From PATINA_IDENTITY env var
    Environment,
    /// From encrypted file (machine-bound)
    EncryptedFile,
    /// From macOS Keychain (legacy or dual-storage)
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

/// Get identity source for display.
pub fn get_identity_source() -> Option<IdentitySource> {
    // Check env var first
    if let Ok(identity) = std::env::var(IDENTITY_ENV_VAR) {
        if !identity.is_empty() {
            return Some(IdentitySource::Environment);
        }
    }

    // Check encrypted file
    if encrypted_file::has_identity() {
        return Some(IdentitySource::EncryptedFile);
    }

    // Check Keychain (legacy — stubs return false on non-macOS)
    if keychain::has_identity() {
        return Some(IdentitySource::Keychain);
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_identity() {
        let (identity, recipient) = generate_identity();
        assert!(identity.starts_with("AGE-SECRET-KEY-1"));
        assert!(recipient.starts_with("age1"));
    }

    #[test]
    fn test_identity_source_display() {
        assert_eq!(
            format!("{}", IdentitySource::Environment),
            "PATINA_IDENTITY"
        );
        assert_eq!(format!("{}", IdentitySource::Keychain), "macOS Keychain");
    }
}
