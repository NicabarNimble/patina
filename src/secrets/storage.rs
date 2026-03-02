//! Storage orchestration for age identity.
//!
//! Dual-storage strategy:
//! - macOS: Keychain (Secure Enclave) + encrypted file (SSH fallback)
//! - Linux: Encrypted file only
//!
//! Resolution priority:
//! 1. Encrypted file (universal, works everywhere)
//! 2. Keychain (legacy, auto-migrates to encrypted file)
//!
//! Phase 1: Conservative - always use encrypted file (skip Keychain optimization)
//! Phase 2: Smart detection - use Keychain on confirmed local console

use crate::secrets::encrypted_file;
#[cfg(target_os = "macos")]
use crate::secrets::keychain;
use anyhow::{bail, Result};

/// Debug logging for storage module
fn debug_log(msg: &str) {
    if std::env::var("PATINA_LOG").is_ok() {
        eprintln!("[DEBUG secrets::storage] {}", msg);
    }
}

/// Store an identity using dual-storage strategy.
///
/// - macOS: Writes to BOTH Keychain and encrypted file
/// - Linux: Writes to encrypted file only
///
/// This ensures SSH works immediately after setup (no migration needed).
pub fn store_identity(identity: &str) -> Result<()> {
    #[cfg(target_os = "macos")]
    {
        debug_log("store: dual-write (Keychain + encrypted file)");

        // macOS: ALWAYS write both Keychain and encrypted file
        // This ensures automatic migration and SSH fallback
        let keychain_result = keychain::store_identity(identity);
        let file_result = encrypted_file::store_identity(identity);

        // Both succeed: best case
        // One succeeds: acceptable (other is fallback)
        // Both fail: error
        match (keychain_result, file_result) {
            (Ok(()), Ok(())) => {
                debug_log("store: both succeeded");
                Ok(())
            }
            (Ok(()), Err(e)) => {
                debug_log(&format!("store: Keychain OK, file failed: {}", e));
                Ok(()) // Keychain works, acceptable
            }
            (Err(_), Ok(())) => {
                debug_log("store: file OK, Keychain failed (acceptable)");
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
        debug_log("store: encrypted file only (non-macOS)");
        // Linux: Only encrypted file available
        encrypted_file::store_identity(identity)
    }
}

/// Retrieve the age identity from storage.
///
/// Resolution order (Phase 1):
/// 1. Encrypted file (universal, works everywhere)
/// 2. Keychain (legacy, auto-migrates to encrypted file)
///
/// Phase 1: Always use encrypted file (conservative, prevents -25308)
/// Phase 2: Smart console detection → use Keychain when safe
pub fn get_identity() -> Result<String> {
    // 1. Try encrypted file first (universal path)
    if encrypted_file::has_identity() {
        debug_log("get: source=encrypted_file");
        return encrypted_file::get_identity();
    }

    // 2. Fall back to Keychain (legacy, pre-dual-storage users)
    #[cfg(target_os = "macos")]
    {
        // Phase 1: is_remote_session() returns true everywhere (conservative)
        // So this branch COULD use Keychain if we wanted, but we skip it for safety
        // Phase 2: Will add positive console detection here

        if keychain::has_identity() {
            debug_log("get: source=keychain (legacy, auto-migrating)");
            let identity = keychain::get_identity()?;

            // Auto-migrate: Create encrypted file for future use
            // This is "Safety Net 2" from spec - eager on-demand migration
            debug_log(
                r#"event="secrets.migrate" source="keychain" dest="encrypted_file" reason="auto_migration""#,
            );

            if let Err(e) = encrypted_file::store_identity(&identity) {
                // Log failure but continue (Keychain still works)
                debug_log(&format!(
                    r#"event="secrets.migrate" result="failed" error="{}""#,
                    e
                ));
            } else {
                debug_log(r#"event="secrets.migrate" result="ok""#);
            }

            return Ok(identity);
        }
    }

    // No identity found anywhere
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

/// Check if an identity exists in storage.
///
/// Checks encrypted file, then Keychain (legacy).
pub fn has_identity() -> bool {
    // Check encrypted file first
    if encrypted_file::has_identity() {
        return true;
    }

    // Check Keychain (legacy, pre-dual-storage)
    #[cfg(target_os = "macos")]
    {
        keychain::has_identity()
    }

    #[cfg(not(target_os = "macos"))]
    {
        false
    }
}

/// Check if we're in a remote session (SSH, CI, Codespaces).
///
/// Phase 1: Conservative - default to true (remote/encrypted file).
/// Returns false (use Keychain) only when we have POSITIVE console signals.
///
/// Rationale: Wrong guess → slower (file instead of Keychain), not broken (-25308).
/// Better to skip Keychain optimization than hit -25308 errors.
#[allow(dead_code)] // Used in Phase 2
fn is_remote_session() -> bool {
    // Known remote indicators (if ANY present → definitely remote):
    // - SSH_CONNECTION (standard SSH)
    // - SSH_TTY (SSH with PTY)
    // - SSH_CLIENT (alternate SSH var)
    // - CI=true (GitHub Actions, GitLab CI, etc.)
    // - CODESPACES=true (GitHub Codespaces)
    if std::env::var("SSH_CONNECTION").is_ok()
        || std::env::var("SSH_TTY").is_ok()
        || std::env::var("SSH_CLIENT").is_ok()
        || std::env::var("CI").is_ok()
        || std::env::var("CODESPACES").is_ok()
    {
        return true; // Definitely remote
    }

    // Phase 1 conservative default: Treat unknown as remote
    // This skips Keychain optimization for mosh, VS Code Remote, etc.
    // but prevents -25308 errors
    true

    // Phase 2 (future): Add POSITIVE console detection
    // Only return false (use Keychain) if we detect:
    // - macOS: Controlling TTY owned by loginwindow process
    // - macOS: launchd session type is "Aqua" (not "Background" or "LoginWindow")
    // - macOS: TERM_PROGRAM is "Apple_Terminal" or "iTerm.app"
    //
    // Until then: encrypted file everywhere (safe, slightly slower)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_remote_session_conservative() {
        // Phase 1: Always returns true (conservative)
        // Even without SSH env vars, defaults to remote
        std::env::remove_var("SSH_CONNECTION");
        std::env::remove_var("SSH_TTY");
        std::env::remove_var("SSH_CLIENT");
        std::env::remove_var("CI");
        std::env::remove_var("CODESPACES");

        assert!(is_remote_session(), "Phase 1 should default to remote");
    }

    #[test]
    fn test_is_remote_session_detects_ssh() {
        std::env::set_var("SSH_CONNECTION", "test");
        assert!(is_remote_session());
        std::env::remove_var("SSH_CONNECTION");
    }
}
