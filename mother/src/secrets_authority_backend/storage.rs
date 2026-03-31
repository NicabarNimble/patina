use super::{encrypted_file, keychain};
use anyhow::{bail, Result};

fn debug_log(msg: &str) {
    if std::env::var("PATINA_LOG").is_ok() {
        tracing::debug!(message = msg, "secrets::storage");
    }
}

pub fn store_identity(identity: &str) -> Result<()> {
    debug_log("store: dual-write (Keychain + encrypted file)");

    let keychain_result = keychain::store_identity(identity);
    let file_result = encrypted_file::store_identity(identity);

    match (keychain_result, file_result) {
        (Ok(()), Ok(())) => {
            debug_log("store: both succeeded");
            Ok(())
        }
        (Ok(()), Err(e)) => {
            debug_log(&format!("store: Keychain OK, file failed: {}", e));
            Ok(())
        }
        (Err(_), Ok(())) => {
            debug_log("store: file OK, Keychain failed (acceptable)");
            Ok(())
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

pub fn get_identity() -> Result<String> {
    if encrypted_file::has_identity() {
        debug_log("get: source=encrypted_file");
        return encrypted_file::get_identity();
    }

    if keychain::has_identity() {
        debug_log("get: source=keychain (legacy, auto-migrating)");
        let identity = keychain::get_identity()?;
        debug_log(
            r#"event="secrets.migrate" source="keychain" dest="encrypted_file" reason="auto_migration""#,
        );

        if let Err(e) = encrypted_file::store_identity(&identity) {
            debug_log(&format!(
                r#"event="secrets.migrate" result="failed" error="{}""#,
                e
            ));
        } else {
            debug_log(r#"event="secrets.migrate" result="ok""#);
        }

        return Ok(identity);
    }

    bail!(
        "No identity found in storage.\n\
         \n\
         This is called after env var check, so PATINA_IDENTITY is not set.\n\
         \n\
         Setup:\n\
         1. Import existing identity: patina secrets --import-key\n\
         2. Or generate new: patina secrets setup-claude"
    )
}

pub fn has_identity() -> bool {
    if encrypted_file::has_identity() {
        return true;
    }
    keychain::has_identity()
}
