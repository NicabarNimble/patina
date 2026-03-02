//! Encrypted file storage for age identity.
//!
//! Platform-agnostic storage using ChaCha20-Poly1305 + machine-bound key derivation.
//! Works everywhere (macOS SSH, Linux), fallback when Keychain unavailable.
//!
//! Security properties:
//! - LLM-safe: cat ~/.patina/identity.enc shows encrypted gibberish
//! - Machine-bound: Encrypted with hardware-specific machine ID
//! - Authenticated: ChaCha20-Poly1305 prevents tampering
//! - Not in env: Never stored in environment variables
//!
//! File format (v1):
//! [magic: 6 bytes = b"PATINA"][version: 1 byte = 0x01][salt: 32 bytes][nonce: 12 bytes][ciphertext: variable][auth tag: 16 bytes]

use anyhow::{bail, Context, Result};
use chacha20poly1305::{
    aead::{Aead, KeyInit},
    ChaCha20Poly1305, Nonce,
};
use hkdf::Hkdf;
use rand::RngCore;
use sha2::Sha256;
use std::path::PathBuf;

// File format constants
const MAGIC: &[u8] = b"PATINA";
const VERSION: u8 = 0x01;
const SALT_LEN: usize = 32;
const NONCE_LEN: usize = 12;
const TAG_LEN: usize = 16; // ChaCha20-Poly1305 auth tag
const HEADER_LEN: usize = 7; // MAGIC (6) + VERSION (1)
const MIN_FILE_LEN: usize = HEADER_LEN + SALT_LEN + NONCE_LEN + TAG_LEN; // 67 bytes

/// Debug logging (matches keychain.rs pattern)
fn debug_log(msg: &str) {
    if std::env::var("PATINA_LOG").is_ok() {
        eprintln!("[DEBUG secrets::encrypted_file] {}", msg);
    }
}

/// Get path to encrypted identity file
fn identity_enc_path() -> PathBuf {
    dirs::home_dir()
        .expect("Cannot determine home directory")
        .join(".patina")
        .join("identity.enc")
}

/// Store an age identity in encrypted file.
///
/// File format: [PATINA][0x01][salt][nonce][ciphertext+tag]
/// Encryption: ChaCha20-Poly1305 with key derived from machine ID
pub fn store_identity(identity: &str) -> Result<()> {
    debug_log(r#"event="secrets.store" dest="encrypted_file""#);

    // 1. Get machine ID (platform-specific)
    let machine_id = get_machine_id()?;

    // 2. Generate random salt and nonce
    let mut salt = [0u8; SALT_LEN];
    let mut nonce_bytes = [0u8; NONCE_LEN];
    let mut rng = rand::thread_rng();
    rng.fill_bytes(&mut salt);
    rng.fill_bytes(&mut nonce_bytes);

    // 3. Derive encryption key via HKDF-SHA256
    let key = derive_key(&machine_id, &salt)?;

    // 4. Encrypt with ChaCha20-Poly1305
    let cipher = ChaCha20Poly1305::new(&key.into());
    let nonce = Nonce::from_slice(&nonce_bytes);
    let ciphertext = cipher
        .encrypt(nonce, identity.as_bytes())
        .map_err(|e| anyhow::anyhow!("Encryption failed: {}", e))?;

    // 5. Build file data: [MAGIC][VERSION][salt][nonce][ciphertext+tag]
    let mut file_data = Vec::with_capacity(HEADER_LEN + SALT_LEN + NONCE_LEN + ciphertext.len());
    file_data.extend_from_slice(MAGIC);
    file_data.push(VERSION);
    file_data.extend_from_slice(&salt);
    file_data.extend_from_slice(&nonce_bytes);
    file_data.extend_from_slice(&ciphertext);

    // 6. Write with file safety (atomic write, restrictive permissions)
    write_atomic(&identity_enc_path(), &file_data)?;

    debug_log(r#"event="secrets.store" result="ok""#);
    Ok(())
}

/// Retrieve the age identity from encrypted file.
///
/// Decrypts using machine ID. Fails if machine ID changed (hardware swap, OS reinstall).
pub fn get_identity() -> Result<String> {
    debug_log(r#"event="secrets.get" source="encrypted_file""#);

    let path = identity_enc_path();

    // Check if file exists
    if !path.exists() {
        #[cfg(target_os = "macos")]
        {
            // macOS SSH: Show helpful migration/setup message
            if is_remote_session() {
                bail!(
                    "Encrypted identity file not found: {}\n\
                     \n\
                     This is normal if you just upgraded or are in an SSH session.\n\
                     \n\
                     Setup options:\n\
                     1. If you have an identity in Keychain (from console):\n\
                        - Run 'patina secrets setup-claude' from console to create encrypted file\n\
                        - Then SSH will work automatically\n\
                     2. If this is a new setup:\n\
                        - Run 'patina secrets --import-key' from console\n\
                     3. Temporary workaround:\n\
                        - Set PATINA_IDENTITY env var in your SSH session",
                    path.display()
                );
            }
        }

        bail!(
            "Encrypted identity file not found: {}\n\
             Run: patina secrets --import-key",
            path.display()
        );
    }

    // 1. Read file
    let data = std::fs::read(&path)
        .with_context(|| format!("Failed to read encrypted identity: {}", path.display()))?;

    // 2. Check permissions (warn if too permissive)
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let metadata = std::fs::metadata(&path)?;
        let perms = metadata.permissions();
        let mode = perms.mode() & 0o777;

        if mode != 0o600 {
            eprintln!(
                "⚠️  Warning: Identity file has permissive permissions ({:o})",
                mode
            );
            eprintln!("   Recommended: chmod 600 {}", path.display());
            eprintln!("   File is encrypted but permissions should be owner-only.");
        }
    }

    // 3. Sanity-check file length
    if data.len() < MIN_FILE_LEN {
        bail!(
            "Corrupted identity file (too short: {} bytes, expected ≥ {}).\n\
             May have been truncated. Recovery: re-run setup-claude",
            data.len(),
            MIN_FILE_LEN
        );
    }

    // 4. Verify magic header
    if &data[0..6] != MAGIC {
        bail!(
            "Invalid encrypted identity file (missing magic header).\n\
             Expected PATINA header. Recovery: re-run setup-claude"
        );
    }

    // 5. Check version
    let version = data[6];
    if version != VERSION {
        bail!(
            "Unsupported file version {}. Current version: {}.\n\
             Please upgrade Patina.",
            version,
            VERSION
        );
    }

    // 6. Parse payload (safe - we validated length)
    let payload = &data[7..];
    let salt = &payload[0..SALT_LEN];
    let nonce_bytes = &payload[SALT_LEN..SALT_LEN + NONCE_LEN];
    let ciphertext = &payload[SALT_LEN + NONCE_LEN..];

    // 7. Get current machine ID
    let machine_id = get_machine_id().context(
        "Failed to get machine ID. If you've changed hardware or reinstalled OS,\n\
         you may need to re-import your identity: patina secrets --import-key",
    )?;

    // 8. Derive key (same salt as encryption)
    let key = derive_key(&machine_id, salt)?;

    // 9. Decrypt with ChaCha20-Poly1305
    let cipher = ChaCha20Poly1305::new(&key.into());
    let nonce = Nonce::from_slice(nonce_bytes);
    let plaintext = cipher.decrypt(nonce, ciphertext).map_err(|_| {
        anyhow::anyhow!(
            "Failed to decrypt identity file.\n\
             \n\
             This usually happens after hardware changes or OS reinstall.\n\
             \n\
             Recovery options:\n\
             1. Re-import identity: patina secrets --import-key\n\
             2. Use PATINA_IDENTITY env var temporarily\n\
             \n\
             The encrypted file is at: {}",
            path.display()
        )
    })?;

    let identity = String::from_utf8(plaintext).context("Decrypted identity is not valid UTF-8")?;

    debug_log(&format!(
        r#"event="secrets.get" result="ok" identity_length={}"#,
        identity.len()
    ));
    Ok(identity)
}

/// Check if encrypted identity file exists
pub fn has_identity() -> bool {
    identity_enc_path().exists()
}

/// Derive encryption key from machine ID using HKDF-SHA256
fn derive_key(machine_id: &[u8], salt: &[u8]) -> Result<[u8; 32]> {
    let hkdf = Hkdf::<Sha256>::new(Some(salt), machine_id);
    let mut key = [0u8; 32];
    hkdf.expand(b"patina-identity-v1", &mut key)
        .map_err(|e| anyhow::anyhow!("HKDF key derivation failed: {}", e))?;
    Ok(key)
}

/// Get machine ID (platform-specific stable identifier)
fn get_machine_id() -> Result<Vec<u8>> {
    #[cfg(target_os = "macos")]
    {
        // macOS: Only IOPlatformUUID (no fallback)
        let uuid = get_ioplatform_uuid()?;
        validate_machine_id(&uuid)?;
        Ok(uuid.into_bytes())
    }

    #[cfg(target_os = "linux")]
    {
        // Linux: Try /etc/machine-id first, fall back to D-Bus
        if let Ok(id) = read_machine_id_file("/etc/machine-id") {
            validate_machine_id(&id)?;
            return Ok(id.into_bytes());
        }
        if let Ok(id) = read_machine_id_file("/var/lib/dbus/machine-id") {
            validate_machine_id(&id)?;
            return Ok(id.into_bytes());
        }

        bail!(
            "Cannot determine machine ID\n\
             \n\
             Required for encrypted secret storage.\n\
             \n\
             Linux: /etc/machine-id not found or empty\n\
             Fallback: /var/lib/dbus/machine-id not found or empty\n\
             \n\
             Fix: Install systemd or dbus to generate machine-id:\n\
             systemd-machine-id-setup    # systemd systems\n\
             dbus-uuidgen > /etc/machine-id  # non-systemd systems"
        );
    }

    #[cfg(not(any(target_os = "macos", target_os = "linux")))]
    {
        bail!("Encrypted file storage not supported on this platform")
    }
}

/// Get IOPlatformUUID on macOS
#[cfg(target_os = "macos")]
fn get_ioplatform_uuid() -> Result<String> {
    use std::process::Command;

    let output = Command::new("ioreg")
        .args(["-rd1", "-c", "IOPlatformExpertDevice"])
        .output()
        .context("Failed to run ioreg command")?;

    if !output.status.success() {
        bail!("ioreg command failed");
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    for line in stdout.lines() {
        if line.contains("IOPlatformUUID") {
            // Example: "IOPlatformUUID" = "CF0AF895-29FC-52C2-BE21-D0CB6886A9FF"
            // Find the = sign, then extract value between quotes after it
            if let Some(equals_pos) = line.find('=') {
                let after_equals = &line[equals_pos + 1..];
                // Find first and last quote in the value part
                if let Some(first_quote) = after_equals.find('"') {
                    let after_first_quote = &after_equals[first_quote + 1..];
                    if let Some(last_quote) = after_first_quote.find('"') {
                        let uuid = &after_first_quote[..last_quote];
                        return Ok(uuid.to_string());
                    }
                }
            }
        }
    }

    bail!(
        "Cannot determine machine ID (IOPlatformUUID)\n\
         \n\
         Required for encrypted secret storage.\n\
         \n\
         The ioreg command failed or returned invalid data.\n\
         \n\
         Troubleshooting:\n\
         1. Try running manually: ioreg -rd1 -c IOPlatformExpertDevice | grep IOPlatformUUID\n\
         2. Reboot and try again (IOKit issue)\n\
         3. If problem persists, please file an issue"
    )
}

/// Read machine ID from file (Linux)
#[cfg(target_os = "linux")]
fn read_machine_id_file(path: &str) -> Result<String> {
    let content =
        std::fs::read_to_string(path).with_context(|| format!("Failed to read {}", path))?;
    let trimmed = content.trim();
    if trimmed.is_empty() {
        bail!("{} is empty", path);
    }
    Ok(trimmed.to_string())
}

/// Validate machine ID format
fn validate_machine_id(id: &str) -> Result<()> {
    let trimmed = id.trim();

    // UUID format (IOPlatformUUID): 36 chars with 4 hyphens
    if trimmed.len() == 36 && trimmed.matches('-').count() == 4 {
        return Ok(());
    }

    // Hex format (machine-id): 32 hex chars
    if trimmed.len() == 32 && trimmed.chars().all(|c| c.is_ascii_hexdigit()) {
        return Ok(());
    }

    bail!(
        "Invalid machine ID format: '{}'\n\
         Expected: UUID (36 chars) or hex (32 chars)",
        trimmed
    )
}

/// Write file atomically with restrictive permissions
fn write_atomic(path: &PathBuf, data: &[u8]) -> Result<()> {
    // Ensure .patina directory exists with correct permissions
    let patina_dir = path.parent().unwrap();
    std::fs::create_dir_all(patina_dir)?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = std::fs::metadata(patina_dir)?.permissions();
        perms.set_mode(0o700); // drwx------ (owner-only)
        std::fs::set_permissions(patina_dir, perms)?;
    }

    // Write to temp file first
    let temp_path = path.with_extension("enc.tmp");

    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(&temp_path)?;

    // Set restrictive permissions before writing (owner read/write only)
    #[cfg(unix)]
    {
        use std::io::Write;
        use std::os::unix::fs::PermissionsExt;
        let mut perms = file.metadata()?.permissions();
        perms.set_mode(0o600); // -rw------- (owner-only)
        std::fs::set_permissions(&temp_path, perms)?;

        // Write data
        file.write_all(data)?;

        // Ensure data is on disk before rename (prevent corruption on crash)
        file.sync_all()?;
    }

    #[cfg(not(unix))]
    {
        use std::io::Write;
        file.write_all(data)?;
        file.sync_all()?;
    }

    drop(file); // Close before rename

    // Atomic rename (replaces old file, or creates new)
    std::fs::rename(&temp_path, path)?;

    Ok(())
}

/// Check if we're in a remote session (SSH, CI, Codespaces)
///
/// Phase 1: Conservative - default to true (remote/encrypted file)
/// Returns false (use Keychain) only when we have POSITIVE console signals
#[cfg(target_os = "macos")]
fn is_remote_session() -> bool {
    // Known remote indicators (if ANY present → definitely remote)
    if std::env::var("SSH_CONNECTION").is_ok()
        || std::env::var("SSH_TTY").is_ok()
        || std::env::var("SSH_CLIENT").is_ok()
        || std::env::var("CI").is_ok()
        || std::env::var("CODESPACES").is_ok()
    {
        return true;
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
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_constants() {
        assert_eq!(MAGIC, b"PATINA");
        assert_eq!(VERSION, 0x01);
        assert_eq!(HEADER_LEN, 7);
        assert_eq!(MIN_FILE_LEN, 67);
    }

    #[test]
    fn test_validate_machine_id() {
        // Valid UUID
        assert!(validate_machine_id("12345678-1234-1234-1234-123456789ABC").is_ok());

        // Valid hex (32 chars)
        assert!(validate_machine_id("0123456789abcdef0123456789abcdef").is_ok());

        // Invalid formats
        assert!(validate_machine_id("").is_err());
        assert!(validate_machine_id("short").is_err());
        assert!(validate_machine_id("not-a-uuid-or-hex").is_err());
    }

    #[test]
    fn test_derive_key() {
        let machine_id = b"test-machine-id";
        let salt = b"test-salt-32-bytes-long-xxxxxxx";

        let key1 = derive_key(machine_id, salt).unwrap();
        let key2 = derive_key(machine_id, salt).unwrap();

        // Same input → same key
        assert_eq!(key1, key2);
        assert_eq!(key1.len(), 32);

        // Different salt → different key
        let different_salt = b"different-salt-32-bytes-xxxxxxx";
        let key3 = derive_key(machine_id, different_salt).unwrap();
        assert_ne!(key1, key3);
    }
}
