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
#[cfg(target_os = "macos")]
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
    /// Uses `kSecAttrAccessibleAlwaysThisDeviceOnly` via raw `SecItemAdd` —
    /// avoids `SecAccessControlCreateWithFlags` which fails with -34018 on
    /// macOS 26 for ad-hoc signed binaries.
    ///
    /// Policy properties:
    /// - Always accessible: SSH sessions, daemons, post-reboot (no login required)
    /// - Hardware-encrypted by M-chip Secure Enclave (device-unique key)
    /// - Device-bound: cannot sync to iCloud or restore to another device
    /// - Deprecated on iOS (stolen phone threat model), correct for stationary Mac
    ///
    /// Deletes any existing item first to upgrade old access policies.
    pub fn store_identity(identity: &str) -> Result<()> {
        use core_foundation::base::TCFType;
        use core_foundation::data::CFData;
        use core_foundation::string::CFString;
        use core_foundation::string::CFStringRef;
        use security_framework::passwords::delete_generic_password;
        use security_framework_sys::access_control::kSecAttrAccessibleAlwaysThisDeviceOnly;
        use security_framework_sys::item::{
            kSecAttrAccount, kSecAttrService, kSecClass, kSecClassGenericPassword, kSecValueData,
        };
        use security_framework_sys::keychain_item::SecItemAdd;

        // kSecAttrAccessible is not exported by security-framework-sys.
        // Declare it from the Security framework directly.
        extern "C" {
            static kSecAttrAccessible: CFStringRef;
        }

        // Delete any existing item first — SecItemAdd fails on duplicates.
        // Silently ignore "not found" errors on fresh installs.
        let _ = delete_generic_password(KEYCHAIN_SERVICE, KEYCHAIN_ACCOUNT);
        log_debug("store_identity: cleared existing item");

        // Build raw SecItemAdd dictionary with kSecAttrAccessible directly.
        // This bypasses SecAccessControlCreateWithFlags which is broken on
        // macOS 26 for ad-hoc signed binaries (error -34018).
        let keys = unsafe {
            [
                CFString::wrap_under_get_rule(kSecClass),
                CFString::wrap_under_get_rule(kSecAttrService),
                CFString::wrap_under_get_rule(kSecAttrAccount),
                CFString::wrap_under_get_rule(kSecValueData),
                CFString::wrap_under_get_rule(kSecAttrAccessible),
            ]
        };
        let values: Vec<core_foundation::base::CFType> = unsafe {
            vec![
                CFString::wrap_under_get_rule(kSecClassGenericPassword).into_CFType(),
                CFString::from(KEYCHAIN_SERVICE).into_CFType(),
                CFString::from(KEYCHAIN_ACCOUNT).into_CFType(),
                CFData::from_buffer(identity.as_bytes()).into_CFType(),
                CFString::wrap_under_get_rule(kSecAttrAccessibleAlwaysThisDeviceOnly)
                    .into_CFType(),
            ]
        };

        let dict = core_foundation::dictionary::CFDictionary::from_CFType_pairs(
            &keys.iter().cloned().zip(values).collect::<Vec<_>>(),
        );

        log_debug("SecItemAdd: attempting (AlwaysThisDeviceOnly, raw)");
        let status = unsafe { SecItemAdd(dict.as_concrete_TypeRef(), std::ptr::null_mut()) };
        if status != 0 {
            anyhow::bail!(
                "Failed to store identity in Keychain (SecItemAdd status: {})",
                status
            );
        }

        log_debug("store_identity: success");
        Ok(())
    }

    /// Retrieve the age identity from the macOS Keychain.
    ///
    /// Uses raw `SecItemCopyMatching` with explicit authentication UI control —
    /// avoids high-level wrappers that may set query attributes incompatible
    /// with SSH/headless contexts.
    ///
    /// No Touch ID prompt — items stored with AlwaysThisDeviceOnly are
    /// accessible without user presence, including over SSH and after reboot.
    /// Re-run `patina secrets setup-claude` to upgrade legacy items.
    pub fn get_identity() -> Result<String> {
        use core_foundation::base::TCFType;
        use core_foundation::string::CFString;
        use core_foundation::string::CFStringRef;
        use security_framework_sys::item::{
            kSecAttrAccount, kSecAttrService, kSecClass, kSecClassGenericPassword,
            kSecReturnData, kSecMatchLimit,
        };
        use security_framework_sys::keychain_item::SecItemCopyMatching;

        // Constants not exported by security-framework-sys.
        // Declare them from the Security framework directly.
        extern "C" {
            static kSecMatchLimitOne: CFStringRef;
            static kSecUseAuthenticationUI: CFStringRef;
            static kSecUseAuthenticationUIFail: CFStringRef;
        }

        // Build raw SecItemCopyMatching query with explicit auth UI control.
        // kSecUseAuthenticationUIFail tells macOS: "don't prompt, just fail if auth required"
        // Combined with AlwaysThisDeviceOnly items, this works in SSH without interaction.
        let keys = unsafe {
            [
                CFString::wrap_under_get_rule(kSecClass),
                CFString::wrap_under_get_rule(kSecAttrService),
                CFString::wrap_under_get_rule(kSecAttrAccount),
                CFString::wrap_under_get_rule(kSecReturnData),
                CFString::wrap_under_get_rule(kSecMatchLimit),
                CFString::wrap_under_get_rule(kSecUseAuthenticationUI),
            ]
        };
        let values: Vec<core_foundation::base::CFType> = unsafe {
            vec![
                CFString::wrap_under_get_rule(kSecClassGenericPassword).into_CFType(),
                CFString::from(KEYCHAIN_SERVICE).into_CFType(),
                CFString::from(KEYCHAIN_ACCOUNT).into_CFType(),
                core_foundation::boolean::CFBoolean::true_value().into_CFType(),
                CFString::wrap_under_get_rule(kSecMatchLimitOne).into_CFType(),
                CFString::wrap_under_get_rule(kSecUseAuthenticationUIFail).into_CFType(),
            ]
        };

        let dict = core_foundation::dictionary::CFDictionary::from_CFType_pairs(
            &keys.iter().cloned().zip(values).collect::<Vec<_>>(),
        );

        log_debug("SecItemCopyMatching: attempting (AlwaysThisDeviceOnly, no auth UI)");
        let mut result: core_foundation::base::CFTypeRef = std::ptr::null();
        let status = unsafe {
            SecItemCopyMatching(
                dict.as_concrete_TypeRef(),
                &mut result as *mut core_foundation::base::CFTypeRef,
            )
        };

        if status != 0 {
            log_debug(&format!("SecItemCopyMatching: error status {}", status));
            anyhow::bail!(
                "Failed to retrieve identity from Keychain. Run: patina secrets --import-key"
            );
        }

        log_debug("SecItemCopyMatching: success");

        // Result is CFData, extract bytes
        let data: core_foundation::data::CFData = unsafe {
            core_foundation::data::CFData::wrap_under_create_rule(
                result as core_foundation::data::CFDataRef,
            )
        };

        let bytes = data.bytes();
        String::from_utf8(bytes.to_vec()).context("Keychain identity is not valid UTF-8")
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
    /// Uses raw `SecItemCopyMatching` to avoid authentication UI prompts.
    /// Does not trigger Touch ID - just checks existence.
    pub fn has_identity() -> bool {
        use core_foundation::base::TCFType;
        use core_foundation::string::CFString;
        use core_foundation::string::CFStringRef;
        use security_framework_sys::item::{
            kSecAttrAccount, kSecAttrService, kSecClass, kSecClassGenericPassword,
            kSecMatchLimit,
        };
        use security_framework_sys::keychain_item::SecItemCopyMatching;

        extern "C" {
            static kSecMatchLimitOne: CFStringRef;
            static kSecUseAuthenticationUI: CFStringRef;
            static kSecUseAuthenticationUIFail: CFStringRef;
        }

        // Query without returning data, just check existence
        let keys = unsafe {
            [
                CFString::wrap_under_get_rule(kSecClass),
                CFString::wrap_under_get_rule(kSecAttrService),
                CFString::wrap_under_get_rule(kSecAttrAccount),
                CFString::wrap_under_get_rule(kSecMatchLimit),
                CFString::wrap_under_get_rule(kSecUseAuthenticationUI),
            ]
        };
        let values: Vec<core_foundation::base::CFType> = unsafe {
            vec![
                CFString::wrap_under_get_rule(kSecClassGenericPassword).into_CFType(),
                CFString::from(KEYCHAIN_SERVICE).into_CFType(),
                CFString::from(KEYCHAIN_ACCOUNT).into_CFType(),
                CFString::wrap_under_get_rule(kSecMatchLimitOne).into_CFType(),
                CFString::wrap_under_get_rule(kSecUseAuthenticationUIFail).into_CFType(),
            ]
        };

        let dict = core_foundation::dictionary::CFDictionary::from_CFType_pairs(
            &keys.iter().cloned().zip(values).collect::<Vec<_>>(),
        );

        log_debug("has_identity: checking existence (no auth UI)");
        let mut result: core_foundation::base::CFTypeRef = std::ptr::null();
        let status = unsafe {
            SecItemCopyMatching(
                dict.as_concrete_TypeRef(),
                &mut result as *mut core_foundation::base::CFTypeRef,
            )
        };

        let exists = status == 0;
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
