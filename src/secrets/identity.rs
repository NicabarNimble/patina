//! Identity resolution for secrets decryption.
//!
//! Resolution order:
//! 1. PATINA_IDENTITY env var (for CI/headless)
//! 2. macOS Keychain (Touch ID protected) - Phase 1: skipped, Phase 2: console only
//! 3. Encrypted file (machine-bound, works everywhere)

use crate::secrets::encrypted_file;
use crate::secrets::keychain;
use crate::secrets::recipients;
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
/// Dual storage strategy (Phase 1):
/// 1. PATINA_IDENTITY env var (escape hatch)
/// 2. Encrypted file (universal, works everywhere)
/// 3. Keychain (Phase 1: skipped, Phase 2: macOS console only)
pub fn get_identity_string() -> Result<Zeroizing<String>> {
    // 1. Check env first (CI/headless path, escape hatch)
    if let Ok(identity) = std::env::var(IDENTITY_ENV_VAR) {
        if !identity.is_empty() {
            log_debug("source = PATINA_IDENTITY (env var)");
            return Ok(Zeroizing::new(identity));
        }
        log_debug("PATINA_IDENTITY set but empty, falling back");
    }

    // 2. Try encrypted file (universal path)
    if encrypted_file::has_identity() {
        log_debug("source = encrypted_file");
        return Ok(Zeroizing::new(encrypted_file::get_identity()?));
    }

    // 3. Fall back to Keychain (legacy, pre-dual-storage users)
    #[cfg(target_os = "macos")]
    {
        if keychain::has_identity() {
            log_debug("source = Keychain (legacy, migrating)");
            let identity = keychain::get_identity()?;

            // Auto-migrate: Create encrypted file for future use
            if let Err(e) = encrypted_file::store_identity(&identity) {
                log_debug(&format!("Auto-migration failed: {} (continuing)", e));
            } else {
                log_debug("Auto-migration succeeded (encrypted file created)");
            }

            return Ok(Zeroizing::new(identity));
        }
    }

    // No identity found anywhere
    bail!(
        "No identity found.\n\
         \n\
         Setup:\n\
         1. Import existing identity: patina secrets --import-key\n\
         2. Or generate new: patina secrets setup-claude\n\
         \n\
         Temporary workaround:\n\
         export PATINA_IDENTITY='AGE-SECRET-KEY-1...'"
    )
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

/// Store an identity (dual storage strategy).
///
/// - macOS: Writes to BOTH Keychain and encrypted file
/// - Linux: Writes to encrypted file only
///
/// This ensures SSH works immediately after setup (no migration needed).
pub fn store_identity(identity: &str) -> Result<()> {
    // Validate before storing
    if !recipients::is_valid_age_identity(identity) {
        bail!("Invalid age identity format. Expected AGE-SECRET-KEY-1...");
    }

    #[cfg(target_os = "macos")]
    {
        // macOS: Write to both storages
        let keychain_result = keychain::store_identity(identity);
        let file_result = encrypted_file::store_identity(identity);

        // Require at least one to succeed
        match (keychain_result, file_result) {
            (Ok(()), Ok(())) => {
                log_debug("Stored in both Keychain and encrypted file");
                Ok(())
            }
            (Ok(()), Err(e)) => {
                log_debug(&format!("Keychain OK, encrypted file failed: {}", e));
                Ok(()) // Keychain works, acceptable
            }
            (Err(_), Ok(())) => {
                log_debug("Encrypted file OK, Keychain failed (acceptable)");
                Ok(()) // Encrypted file works, acceptable
            }
            (Err(e1), Err(e2)) => {
                bail!(
                    "Failed to store identity in both storages.\n\
                     Keychain error: {}\n\
                     Encrypted file error: {}",
                    e1,
                    e2
                )
            }
        }
    }

    #[cfg(not(target_os = "macos"))]
    {
        // Linux/other: Only encrypted file available
        encrypted_file::store_identity(identity)
    }
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

/// Export the identity from Keychain (zeroized on drop).
///
/// Returns the identity string for backup.
pub fn export_identity() -> Result<Zeroizing<String>> {
    Ok(Zeroizing::new(keychain::get_identity()?))
}

/// Check if an identity is available.
///
/// Checks env var, encrypted file, then Keychain (legacy).
pub fn has_identity() -> bool {
    // Check env var
    if let Ok(identity) = std::env::var(IDENTITY_ENV_VAR) {
        if !identity.is_empty() {
            return true;
        }
    }

    // Check encrypted file
    if encrypted_file::has_identity() {
        return true;
    }

    // Check Keychain (legacy, pre-dual-storage)
    #[cfg(target_os = "macos")]
    {
        return keychain::has_identity();
    }

    #[cfg(not(target_os = "macos"))]
    {
        false
    }
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

    // Check Keychain (legacy)
    #[cfg(target_os = "macos")]
    {
        if keychain::has_identity() {
            return Some(IdentitySource::Keychain);
        }
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
