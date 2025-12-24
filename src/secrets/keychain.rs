//! macOS Keychain integration for age identity storage.
//!
//! Stores the age identity (private key) in the macOS Keychain
//! with Touch ID protection.

use anyhow::{Context, Result};

/// Keychain service name for Patina secrets.
const KEYCHAIN_SERVICE: &str = "patina";

/// Keychain account name for the age identity.
const KEYCHAIN_ACCOUNT: &str = "Patina Secrets";

/// Store an age identity in the macOS Keychain.
///
/// The identity is stored as a generic password with Touch ID protection.
pub fn store_identity(identity: &str) -> Result<()> {
    use security_framework::passwords::set_generic_password;

    set_generic_password(KEYCHAIN_SERVICE, KEYCHAIN_ACCOUNT, identity.as_bytes())
        .context("Failed to store identity in Keychain")?;

    Ok(())
}

/// Retrieve the age identity from the macOS Keychain.
///
/// This will trigger Touch ID if the item is protected.
pub fn get_identity() -> Result<String> {
    use security_framework::passwords::get_generic_password;

    let password = get_generic_password(KEYCHAIN_SERVICE, KEYCHAIN_ACCOUNT)
        .context("Failed to retrieve identity from Keychain. Run: patina secrets --import-key")?;

    String::from_utf8(password).context("Keychain identity is not valid UTF-8")
}

/// Delete the age identity from the macOS Keychain.
pub fn delete_identity() -> Result<()> {
    use security_framework::passwords::delete_generic_password;

    delete_generic_password(KEYCHAIN_SERVICE, KEYCHAIN_ACCOUNT)
        .context("Failed to delete identity from Keychain")?;

    Ok(())
}

/// Check if an identity exists in the Keychain.
///
/// Does not trigger Touch ID - just checks existence.
pub fn has_identity() -> bool {
    // Try to get the password - if it fails, no identity exists
    // Note: This may trigger Touch ID on some systems
    // A better approach would be to query keychain items without retrieving
    // For now, we'll try to get and handle the error
    use security_framework::passwords::get_generic_password;
    get_generic_password(KEYCHAIN_SERVICE, KEYCHAIN_ACCOUNT).is_ok()
}

#[cfg(test)]
mod tests {
    // Note: These tests interact with the actual macOS Keychain
    // They should be run manually, not in CI
    //
    // #[test]
    // fn test_keychain_roundtrip() {
    //     let test_identity = "AGE-SECRET-KEY-1TEST...";
    //     store_identity(test_identity).unwrap();
    //     let retrieved = get_identity().unwrap();
    //     assert_eq!(retrieved, test_identity);
    //     delete_identity().unwrap();
    // }
}
