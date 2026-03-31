use anyhow::Result;

#[cfg(not(target_os = "macos"))]
use anyhow::bail;

#[cfg(target_os = "macos")]
fn log_debug(msg: &str) {
    if std::env::var("PATINA_LOG").is_ok() {
        tracing::debug!(message = msg, "secrets::keychain");
    }
}

#[cfg(target_os = "macos")]
const KEYCHAIN_SERVICE: &str = "patina";
#[cfg(target_os = "macos")]
const KEYCHAIN_ACCOUNT: &str = "Patina Secrets";

#[cfg(target_os = "macos")]
mod platform {
    use super::*;
    use anyhow::Context;

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

        extern "C" {
            static kSecAttrAccessible: CFStringRef;
        }

        let _ = delete_generic_password(KEYCHAIN_SERVICE, KEYCHAIN_ACCOUNT);
        log_debug("store_identity: cleared existing item");

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
                CFString::wrap_under_get_rule(kSecAttrAccessibleAlwaysThisDeviceOnly).into_CFType(),
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

    pub fn has_identity() -> bool {
        use security_framework::passwords::get_generic_password;

        log_debug("has_identity: checking existence (no Touch ID)");
        let exists = get_generic_password(KEYCHAIN_SERVICE, KEYCHAIN_ACCOUNT).is_ok();
        log_debug(&format!("has_identity: {}", exists));
        exists
    }
}

#[cfg(not(target_os = "macos"))]
mod platform {
    use super::*;

    pub fn store_identity(_identity: &str) -> Result<()> {
        bail!(
            "Keychain storage is only available on macOS.\n\
             On Linux/Windows, set the PATINA_IDENTITY environment variable:\n\
             \n\
             export PATINA_IDENTITY='AGE-SECRET-KEY-1...'"
        )
    }

    pub fn get_identity() -> Result<String> {
        bail!(
            "Keychain is only available on macOS.\n\
             Set the PATINA_IDENTITY environment variable:\n\
             \n\
             export PATINA_IDENTITY='AGE-SECRET-KEY-1...'"
        )
    }

    pub fn delete_identity() -> Result<()> {
        bail!("Keychain is only available on macOS")
    }

    pub fn has_identity() -> bool {
        false
    }
}

pub fn store_identity(identity: &str) -> Result<()> {
    platform::store_identity(identity)
}

pub fn get_identity() -> Result<String> {
    platform::get_identity()
}

pub fn delete_identity() -> Result<()> {
    platform::delete_identity()
}

pub fn has_identity() -> bool {
    platform::has_identity()
}
