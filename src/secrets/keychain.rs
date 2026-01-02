//! Keychain integration for age identity storage.
//!
//! Platform support:
//! - macOS: Uses Keychain with Touch ID protection
//! - Linux/Windows: Stubs that return errors (use PATINA_IDENTITY env var)
//!
//! This mirrors the embeddings test design: the workflow (get identity → encrypt/decrypt)
//! works everywhere, but the underlying mechanism differs per platform.

use anyhow::Result;

#[cfg(not(target_os = "macos"))]
use anyhow::bail;

/// Debug logging for secrets module (Phase 0 observability)
fn log_debug(msg: &str) {
    if std::env::var("PATINA_LOG").is_ok() {
        eprintln!("[DEBUG secrets::keychain] {}", msg);
    }
}

// macOS-only constants (used by security-framework)
#[cfg(target_os = "macos")]
const KEYCHAIN_SERVICE: &str = "patina";
#[cfg(target_os = "macos")]
const KEYCHAIN_ACCOUNT: &str = "Patina Secrets";

// =============================================================================
// macOS Implementation - Real Keychain with Touch ID
// =============================================================================

#[cfg(target_os = "macos")]
mod platform {
    use super::*;
    use anyhow::Context;

    /// Store an age identity in the macOS Keychain.
    ///
    /// The identity is stored as a generic password with Touch ID protection.
    pub fn store_identity(identity: &str) -> Result<()> {
        use security_framework::passwords::set_generic_password;

        log_debug("set_generic_password: attempting");
        let result = set_generic_password(KEYCHAIN_SERVICE, KEYCHAIN_ACCOUNT, identity.as_bytes());

        match &result {
            Ok(()) => log_debug("set_generic_password: success"),
            Err(e) => log_debug(&format!("set_generic_password: error: {}", e)),
        }

        result.context("Failed to store identity in Keychain")?;
        Ok(())
    }

    /// Retrieve the age identity from the macOS Keychain.
    ///
    /// This will trigger Touch ID if the item is protected.
    pub fn get_identity() -> Result<String> {
        use security_framework::passwords::get_generic_password;

        log_debug("get_generic_password: attempting (may trigger Touch ID)");
        let result = get_generic_password(KEYCHAIN_SERVICE, KEYCHAIN_ACCOUNT);

        match &result {
            Ok(_) => log_debug("get_generic_password: success"),
            Err(e) => log_debug(&format!("get_generic_password: error: {}", e)),
        }

        let password = result.context(
            "Failed to retrieve identity from Keychain. Run: patina secrets --import-key",
        )?;

        String::from_utf8(password).context("Keychain identity is not valid UTF-8")
    }

    /// Delete the age identity from the macOS Keychain.
    pub fn delete_identity() -> Result<()> {
        use security_framework::passwords::delete_generic_password;

        log_debug("delete_generic_password: attempting");
        let result = delete_generic_password(KEYCHAIN_SERVICE, KEYCHAIN_ACCOUNT);

        match &result {
            Ok(()) => log_debug("delete_generic_password: success"),
            Err(e) => log_debug(&format!("delete_generic_password: error: {}", e)),
        }

        result.context("Failed to delete identity from Keychain")?;
        Ok(())
    }

    /// Check if an identity exists in the Keychain.
    ///
    /// Does not trigger Touch ID - just checks existence.
    pub fn has_identity() -> bool {
        use security_framework::passwords::get_generic_password;

        log_debug("has_identity: checking existence (no Touch ID)");
        let exists = get_generic_password(KEYCHAIN_SERVICE, KEYCHAIN_ACCOUNT).is_ok();
        log_debug(&format!("has_identity: {}", exists));
        exists
    }
}

// =============================================================================
// Non-macOS Stubs - Graceful degradation to env var
// =============================================================================

#[cfg(not(target_os = "macos"))]
mod platform {
    use super::*;

    /// Keychain not available on this platform.
    ///
    /// Use PATINA_IDENTITY environment variable instead.
    pub fn store_identity(_identity: &str) -> Result<()> {
        bail!(
            "Keychain storage is only available on macOS.\n\
             On Linux/Windows, set the PATINA_IDENTITY environment variable:\n\
             \n\
             export PATINA_IDENTITY='AGE-SECRET-KEY-1...'"
        )
    }

    /// Keychain not available on this platform.
    ///
    /// Use PATINA_IDENTITY environment variable instead.
    pub fn get_identity() -> Result<String> {
        bail!(
            "Keychain is only available on macOS.\n\
             Set the PATINA_IDENTITY environment variable:\n\
             \n\
             export PATINA_IDENTITY='AGE-SECRET-KEY-1...'"
        )
    }

    /// Keychain not available on this platform.
    pub fn delete_identity() -> Result<()> {
        bail!("Keychain is only available on macOS")
    }

    /// Keychain not available on this platform.
    ///
    /// Always returns false - use PATINA_IDENTITY env var instead.
    pub fn has_identity() -> bool {
        false
    }
}

// =============================================================================
// Public API - delegates to platform-specific implementation
// =============================================================================

/// Store an age identity in the system keychain.
///
/// - macOS: Uses Keychain with Touch ID protection
/// - Linux/Windows: Returns error with guidance to use PATINA_IDENTITY env var
pub fn store_identity(identity: &str) -> Result<()> {
    platform::store_identity(identity)
}

/// Retrieve the age identity from the system keychain.
///
/// - macOS: Retrieves from Keychain (may trigger Touch ID)
/// - Linux/Windows: Returns error with guidance to use PATINA_IDENTITY env var
pub fn get_identity() -> Result<String> {
    platform::get_identity()
}

/// Delete the age identity from the system keychain.
///
/// - macOS: Deletes from Keychain
/// - Linux/Windows: Returns error (no keychain to delete from)
pub fn delete_identity() -> Result<()> {
    platform::delete_identity()
}

/// Check if an identity exists in the system keychain.
///
/// - macOS: Checks Keychain
/// - Linux/Windows: Always returns false (use PATINA_IDENTITY env var)
pub fn has_identity() -> bool {
    platform::has_identity()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[cfg(target_os = "macos")]
    fn test_keychain_constants() {
        // Verify constants are set correctly (macOS only)
        assert_eq!(KEYCHAIN_SERVICE, "patina");
        assert_eq!(KEYCHAIN_ACCOUNT, "Patina Secrets");
    }

    #[test]
    #[cfg(not(target_os = "macos"))]
    fn test_non_macos_stubs_return_errors() {
        // On non-macOS, keychain operations should fail gracefully
        assert!(store_identity("test").is_err());
        assert!(get_identity().is_err());
        assert!(delete_identity().is_err());
        assert!(!has_identity());
    }

    // Note: macOS keychain tests interact with actual Keychain
    // They should be run manually, not in CI
    //
    // #[test]
    // #[cfg(target_os = "macos")]
    // fn test_keychain_roundtrip() {
    //     let test_identity = "AGE-SECRET-KEY-1TEST...";
    //     store_identity(test_identity).unwrap();
    //     let retrieved = get_identity().unwrap();
    //     assert_eq!(retrieved, test_identity);
    //     delete_identity().unwrap();
    // }
}
