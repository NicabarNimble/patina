//! Identity resolution for secrets decryption.
//!
//! Resolution order:
//! 1. PATINA_IDENTITY env var (for CI/headless)
//! 2. macOS Keychain (Touch ID protected)

use crate::secrets::keychain;
use crate::secrets::recipients;
use age::secrecy::ExposeSecret;
use age::x25519;
use anyhow::{bail, Context, Result};
use std::str::FromStr;

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
    let identity_str = get_identity_string()?;

    x25519::Identity::from_str(&identity_str)
        .map_err(|e| anyhow::anyhow!("Invalid age identity: {}", e))
}

/// Get the identity as a string.
///
/// Useful for export operations.
pub fn get_identity_string() -> Result<String> {
    // 1. Check env first (CI/headless path)
    if let Ok(identity) = std::env::var(IDENTITY_ENV_VAR) {
        if !identity.is_empty() {
            log_debug("source = PATINA_IDENTITY (env var)");
            return Ok(identity);
        }
        log_debug("PATINA_IDENTITY set but empty, falling back to Keychain");
    }

    // 2. Fall back to Keychain (Mac with Touch ID)
    log_debug("source = Keychain");
    keychain::get_identity()
}

/// Get the public key (recipient) for the current identity.
pub fn get_recipient() -> Result<String> {
    let identity = get_identity()?;
    Ok(identity.to_public().to_string())
}

/// Generate a new age identity.
///
/// Returns (identity_string, recipient_string).
pub fn generate_identity() -> (String, String) {
    let identity = x25519::Identity::generate();
    let recipient = identity.to_public();
    (
        identity.to_string().expose_secret().to_string(),
        recipient.to_string(),
    )
}

/// Store an identity in the Keychain.
pub fn store_identity(identity: &str) -> Result<()> {
    // Validate before storing
    if !recipients::is_valid_age_identity(identity) {
        bail!("Invalid age identity format. Expected AGE-SECRET-KEY-1...");
    }

    keychain::store_identity(identity)
}

/// Import an identity from a string.
///
/// Validates and stores in Keychain.
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

    // Store in Keychain
    keychain::store_identity(identity).context("Failed to store identity in Keychain")?;

    Ok(recipient)
}

/// Export the identity from Keychain.
///
/// Returns the identity string for backup.
pub fn export_identity() -> Result<String> {
    keychain::get_identity()
}

/// Check if an identity is available.
///
/// Checks env var first, then Keychain.
pub fn has_identity() -> bool {
    // Check env var
    if let Ok(identity) = std::env::var(IDENTITY_ENV_VAR) {
        if !identity.is_empty() {
            return true;
        }
    }

    // Check Keychain
    keychain::has_identity()
}

/// Identity source for display/debugging.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IdentitySource {
    /// From PATINA_IDENTITY env var
    Environment,
    /// From macOS Keychain
    Keychain,
}

impl std::fmt::Display for IdentitySource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            IdentitySource::Environment => write!(f, "PATINA_IDENTITY"),
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

    // Check Keychain
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
