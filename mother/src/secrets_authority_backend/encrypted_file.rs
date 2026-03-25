use anyhow::{bail, Context, Result};
use chacha20poly1305::{
    aead::{Aead, KeyInit},
    ChaCha20Poly1305, Nonce,
};
use hkdf::Hkdf;
use rand::RngCore;
use sha2::Sha256;
use std::path::PathBuf;

use crate::secrets_paths as paths;

const MAGIC: &[u8] = b"PATINA";
const VERSION: u8 = 0x01;
const SALT_LEN: usize = 32;
const NONCE_LEN: usize = 12;
const TAG_LEN: usize = 16;
const HEADER_LEN: usize = 7;
const MIN_FILE_LEN: usize = HEADER_LEN + SALT_LEN + NONCE_LEN + TAG_LEN;

fn debug_log(msg: &str) {
    if std::env::var("PATINA_LOG").is_ok() {
        eprintln!("[DEBUG secrets::encrypted_file] {}", msg);
    }
}

fn identity_enc_path() -> PathBuf {
    paths::patina_home().join("identity.enc")
}

pub fn store_identity(identity: &str) -> Result<()> {
    debug_log(r#"event="secrets.store" dest="encrypted_file""#);

    let machine_id = get_machine_id()?;

    let mut salt = [0u8; SALT_LEN];
    let mut nonce_bytes = [0u8; NONCE_LEN];
    let mut rng = rand::thread_rng();
    rng.fill_bytes(&mut salt);
    rng.fill_bytes(&mut nonce_bytes);

    let key = derive_key(&machine_id, &salt)?;

    let cipher = ChaCha20Poly1305::new(&key.into());
    let nonce = Nonce::from_slice(&nonce_bytes);
    let ciphertext = cipher
        .encrypt(nonce, identity.as_bytes())
        .map_err(|e| anyhow::anyhow!("Encryption failed: {}", e))?;

    let mut file_data = Vec::with_capacity(HEADER_LEN + SALT_LEN + NONCE_LEN + ciphertext.len());
    file_data.extend_from_slice(MAGIC);
    file_data.push(VERSION);
    file_data.extend_from_slice(&salt);
    file_data.extend_from_slice(&nonce_bytes);
    file_data.extend_from_slice(&ciphertext);

    write_atomic(&identity_enc_path(), &file_data)?;

    debug_log(r#"event="secrets.store" result="ok""#);
    Ok(())
}

pub fn get_identity() -> Result<String> {
    debug_log(r#"event="secrets.get" source="encrypted_file""#);

    let path = identity_enc_path();

    if !path.exists() {
        #[cfg(target_os = "macos")]
        {
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

    let data = std::fs::read(&path)
        .with_context(|| format!("Failed to read encrypted identity: {}", path.display()))?;

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

    if data.len() < MIN_FILE_LEN {
        bail!(
            "Corrupted identity file (too short: {} bytes, expected ≥ {}).\n\
             May have been truncated. Recovery: re-run setup-claude",
            data.len(),
            MIN_FILE_LEN
        );
    }

    if &data[0..6] != MAGIC {
        bail!(
            "Invalid encrypted identity file (missing magic header).\n\
             Expected PATINA header. Recovery: re-run setup-claude"
        );
    }

    let version = data[6];
    if version != VERSION {
        bail!(
            "Unsupported file version {}. Current version: {}.\n\
             Please upgrade Patina.",
            version,
            VERSION
        );
    }

    let payload = &data[7..];
    let salt = &payload[0..SALT_LEN];
    let nonce_bytes = &payload[SALT_LEN..SALT_LEN + NONCE_LEN];
    let ciphertext = &payload[SALT_LEN + NONCE_LEN..];

    let machine_id = get_machine_id().context(
        "Failed to get machine ID. If you've changed hardware or reinstalled OS,\n\
         you may need to re-import your identity: patina secrets --import-key",
    )?;

    let key = derive_key(&machine_id, salt)?;

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

pub fn has_identity() -> bool {
    identity_enc_path().exists()
}

fn derive_key(machine_id: &[u8], salt: &[u8]) -> Result<[u8; 32]> {
    let hkdf = Hkdf::<Sha256>::new(Some(salt), machine_id);
    let mut key = [0u8; 32];
    hkdf.expand(b"patina-identity-v1", &mut key)
        .map_err(|e| anyhow::anyhow!("HKDF key derivation failed: {}", e))?;
    Ok(key)
}

fn get_machine_id() -> Result<Vec<u8>> {
    #[cfg(target_os = "macos")]
    {
        let uuid = get_ioplatform_uuid()?;
        validate_machine_id(&uuid)?;
        Ok(uuid.into_bytes())
    }

    #[cfg(target_os = "linux")]
    {
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
            if let Some(equals_pos) = line.find('=') {
                let after_equals = &line[equals_pos + 1..];
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

fn validate_machine_id(id: &str) -> Result<()> {
    let trimmed = id.trim();

    if trimmed.len() == 36 && trimmed.matches('-').count() == 4 {
        return Ok(());
    }

    if trimmed.len() == 32 && trimmed.chars().all(|c| c.is_ascii_hexdigit()) {
        return Ok(());
    }

    bail!(
        "Invalid machine ID format: '{}'\n\
         Expected: UUID (36 chars) or hex (32 chars)",
        trimmed
    )
}

fn write_atomic(path: &PathBuf, data: &[u8]) -> Result<()> {
    let patina_dir = path.parent().expect("identity file has parent");
    std::fs::create_dir_all(patina_dir)?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = std::fs::metadata(patina_dir)?.permissions();
        perms.set_mode(0o700);
        std::fs::set_permissions(patina_dir, perms)?;
    }

    let temp_path = path.with_extension("enc.tmp");

    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(&temp_path)?;

    #[cfg(unix)]
    {
        use std::io::Write;
        use std::os::unix::fs::PermissionsExt;
        let mut perms = file.metadata()?.permissions();
        perms.set_mode(0o600);
        std::fs::set_permissions(&temp_path, perms)?;
        file.write_all(data)?;
        file.sync_all()?;
    }

    #[cfg(not(unix))]
    {
        use std::io::Write;
        file.write_all(data)?;
        file.sync_all()?;
    }

    drop(file);
    std::fs::rename(&temp_path, path)?;
    Ok(())
}

#[cfg(target_os = "macos")]
fn is_remote_session() -> bool {
    if std::env::var("SSH_CONNECTION").is_ok()
        || std::env::var("SSH_TTY").is_ok()
        || std::env::var("SSH_CLIENT").is_ok()
        || std::env::var("CI").is_ok()
        || std::env::var("CODESPACES").is_ok()
    {
        return true;
    }

    true
}
