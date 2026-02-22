---
type: feat
id: spec-secrets-dual-storage
status: draft
created: 2026-02-22
replaces:
- layer/surface/build/fix/spec-keychain-macos26-regression/SPEC.md
- layer/surface/build/fix/spec-secrets-keychain-ssh/SPEC.md
related:
- layer/surface/build/fix/spec-launcher-auth/SPEC.md
beliefs: []
sessions:
- 20260222-054702
---

# feat: Dual Storage Strategy for LLM-Safe Secrets

> Enable secure secret storage that works in all contexts (macOS console/SSH, Linux)
> while preventing LLM leakage. Use Keychain where available (macOS console),
> encrypted file elsewhere (macOS SSH, Linux).

## Problem

### The Discovery (Session 20260222-054702)

After extensive testing, we discovered that **macOS Keychain never worked over SSH**:

**Test Results:**
```
✅ macOS console + current code    → Keychain works
✅ macOS console + PATINA_IDENTITY → Works
❌ macOS SSH + current code        → OSStatus -25308
❌ macOS SSH + raw SecItemCopyMatching (commit 14064aa1) → OSStatus -25308
❌ macOS SSH + fresh AlwaysThisDeviceOnly item → OSStatus -25308
```

**Root Cause:** macOS Security framework blocks keychain access from SSH sessions
with `errSecInteractionNotAllowed` (-25308) as a **security policy**, regardless of:
- API approach (high-level `get_generic_password` vs raw `SecItemCopyMatching`)
- Accessibility attribute (`kSecAttrAccessibleAlwaysThisDeviceOnly` confirmed)
- Item freshness (recreating with `--import-key` doesn't help)
- Code signing (both signed and unsigned binaries fail)

This is not a bug we can fix - it's a macOS security boundary.

### The Real Threat Model

The original goal was **preventing secrets from leaking to LLMs**, not traditional infosec:

**What we need:**
- ✅ LLM can trigger actions: `patina secrets run -- patina launch`
- ✅ Secrets injected into subprocess environment only (not parent shell)
- ✅ Works from console (Claude Code desktop)
- ✅ Works from SSH (Claude Code over Tailscale)
- ✅ Works on Linux (expanding beyond macOS)
- ❌ LLM should NEVER see secret values

**Attack vectors to prevent:**
- ❌ LLM runs `env | grep PATINA_IDENTITY` and sees secret
- ❌ LLM runs `cat ~/.patina/identity` and reads plaintext
- ❌ Secret appears in LLM context (tool results, error messages)
- ❌ Secret gets committed to git

### Current State (Broken)

**macOS console**: ✅ Keychain works perfectly (LLM-safe, hardware-backed)

**macOS SSH**: ❌ Keychain fails → forces `PATINA_IDENTITY` env var workaround
- Problem: `printenv PATINA_IDENTITY` exposes secret to LLM
- LLM can trivially leak it via shell commands

**Linux**: ❌ No keychain → must use `PATINA_IDENTITY` env var
- Same LLM leakage problem as macOS SSH
- No hardware-backed storage available

### Why Previous Attempts Failed

**spec-secrets-keychain-ssh (complete)**:
- Goal: Make keychain work over SSH with `AlwaysThisDeviceOnly`
- Result: Never actually worked - likely tested with `PATINA_IDENTITY` set

**spec-keychain-macos26-regression (active)**:
- Goal: Fix SSH access via raw `SecItemCopyMatching`
- Result: Fails with same -25308 error (tested empirically in session 20260222-054702)

**The "working build" was using `PATINA_IDENTITY` env var, not Keychain.**

## Solution

### Dual Storage Strategy

Use platform-specific best security when available, with universal fallback:

| Platform | Context | Storage | Security | LLM-Safe? |
|----------|---------|---------|----------|-----------|
| **macOS** | Console | Keychain | Hardware-backed (Secure Enclave) | ✅ Yes |
| **macOS** | SSH | Encrypted file | Machine-bound (software) | ✅ Yes |
| **Linux** | Any | Encrypted file | Machine-bound (software) | ✅ Yes |

### Encrypted File Design

**Storage Location:**
```
~/.patina/identity.enc
```

**File Format:**
```
[magic: 6 bytes = b"PATINA"][version: 1 byte = 0x01][salt: 32 bytes][nonce: 12 bytes][ciphertext: variable][auth tag: 16 bytes]
```

**Version 1 Specification:**
- **Magic bytes:** `b"PATINA"` (6 bytes) - identifies Patina encrypted file
- **Version byte:** `0x01` (1 byte) - format version for future migration
- **Total header:** 7 bytes before encrypted payload

**Versioning Strategy:**
- Version 1 (0x01): HKDF-SHA256 + ChaCha20-Poly1305 (this spec)
- Future versions: Can use different KDF, AEAD, or add metadata
- **Reject unknown versions:** If version > 0x01, error with upgrade prompt
- **Migration path:** Future builds can detect v1 and auto-upgrade to v2

**Reading logic:**
```rust
pub fn get_identity() -> Result<String> {
    let data = std::fs::read(identity_enc_path())?;

    // Check magic bytes
    if data.len() < 7 || &data[0..6] != b"PATINA" {
        bail!("Invalid encrypted identity file (missing magic header)");
    }

    // Check version
    let version = data[6];
    match version {
        0x01 => decrypt_v1(&data[7..]),  // Current version
        _ => bail!("Unsupported file version {}. Please upgrade Patina.", version),
    }
}
```

**Encryption:**
- Algorithm: ChaCha20-Poly1305 (authenticated encryption)
- Library: RustCrypto `chacha20poly1305` crate (well-audited, cross-platform)
- No custom crypto - use proven AEAD primitives

**Key Derivation:**
```rust
// HKDF-SHA256(machine_id, salt, info)
let key = HKDF-SHA256(
    ikm: machine_id,      // Input key material (platform-specific)
    salt: random_32_bytes, // Stored in file header
    info: b"patina-identity-v1"
);
```

**Machine ID Sources (Stable Identifiers):**

**macOS Fallback Order:**
1. **IOPlatformUUID** (only source): Hardware UUID from IOKit
   ```bash
   ioreg -rd1 -c IOPlatformExpertDevice | grep IOPlatformUUID
   ```
   - Survives OS reinstalls, FileVault recovery
   - Format: UUID (36 chars, e.g., `12345678-1234-1234-1234-123456789ABC`)
   - Implementation: Parse `ioreg` output, extract UUID value
   - **This is the canonical machine ID on macOS**

2. **Error** if IOPlatformUUID fails (no secondary fallback):
   ```
   Error: Cannot determine machine ID (IOPlatformUUID)

   Required for encrypted secret storage.

   The ioreg command failed or returned invalid data.

   Troubleshooting:
   1. Try running manually: ioreg -rd1 -c IOPlatformExpertDevice | grep IOPlatformUUID
   2. Reboot and try again (IOKit issue)
   3. If problem persists, please file an issue

   Note: macOS has no secondary machine ID source. Unlike Linux,
   /var/lib/dbus/machine-id does not exist on macOS.
   ```

**Rationale (macOS no fallback):**
- macOS has **one reliable machine-level ID**: IOPlatformUUID
- No equivalent to Linux's `/etc/machine-id` (that's systemd-specific)
- `/var/lib/dbus/machine-id` does **not exist** on macOS (no `/var/lib` directory)
- Preferences UUIDs are user-specific (not machine-level, survive user migration)
- Better to fail fast with clear guidance than pretend we have a fallback

**Linux Fallback Order:**
1. **/etc/machine-id** (preferred): systemd standard
   ```bash
   cat /etc/machine-id
   ```
   - Format: 32 hex chars
   - Stable across reboots
   - May be symlink to /var/lib/dbus/machine-id (follow it)

2. **/var/lib/dbus/machine-id** (fallback)
   - Same format as /etc/machine-id
   - Older systems without systemd

3. **Error** if both fail:
   ```
   Error: Cannot determine machine ID

   Required for encrypted secret storage.

   Linux: /etc/machine-id not found or empty
   Fallback: /var/lib/dbus/machine-id not found or empty

   Fix: Install systemd or dbus to generate machine-id:
     systemd-machine-id-setup    # systemd systems
     dbus-uuidgen > /etc/machine-id  # non-systemd systems
   ```

**Validation Rules:**
- **Non-empty**: Reject empty or whitespace-only values
- **Format check**:
  - UUID: 36 chars matching `XXXXXXXX-XXXX-XXXX-XXXX-XXXXXXXXXXXX`
  - machine-id: 32 hex chars matching `[0-9a-f]{32}`
- **Never generate**: If all sources fail, error instead of generating
  - Generating defeats "machine-bound" security goal
  - User must fix system configuration

**Implementation Note:**
```rust
fn get_machine_id() -> Result<Vec<u8>> {
    #[cfg(target_os = "macos")]
    {
        // macOS: Only IOPlatformUUID (no fallback)
        // Note: /var/lib/dbus/machine-id does NOT exist on macOS
        if let Ok(uuid) = get_ioplatform_uuid() {
            return Ok(uuid.into_bytes());
        }
        bail!("Cannot determine machine ID (IOPlatformUUID failed - see error above)");
    }

    #[cfg(target_os = "linux")]
    {
        // Linux: Try /etc/machine-id first, fall back to D-Bus
        if let Ok(id) = read_machine_id("/etc/machine-id") {
            return Ok(id);
        }
        if let Ok(id) = read_machine_id("/var/lib/dbus/machine-id") {
            return Ok(id);
        }
        bail!("Cannot determine machine ID (see error message above)");
    }
}

fn validate_machine_id(id: &str) -> bool {
    let trimmed = id.trim();
    // UUID format (IOPlatformUUID)
    if trimmed.len() == 36 && trimmed.matches('-').count() == 4 {
        return true;
    }
    // Hex format (machine-id)
    if trimmed.len() == 32 && trimmed.chars().all(|c| c.is_ascii_hexdigit()) {
        return true;
    }
    false
}
```

**Security Properties:**

1. **LLM-safe**: `cat ~/.patina/identity.enc` → encrypted gibberish
2. **Machine-bound**: Encrypted with hardware-specific key
3. **Authenticated**: ChaCha20-Poly1305 prevents tampering
4. **Not in env**: Never stored in environment variables
5. **Subprocess-only**: Secrets injected via `patina secrets run` wrapper

**Security Limitations (LLM Threat Model):**

Given the **LLM threat model** (prevent `cat` and `env` leakage, not disk theft):
- ✅ **Reusing machine-id as HKDF IKM is acceptable**
  - Traditional crypto would use separate values for salt and IKM
  - In our threat model: attacker with machine access can already read memory/keychain
  - Protection goal: Prevent LLM from reading file, not cryptanalyst with disk image

- ⚠️ **Not tamper-proof against machine access**
  - Anyone with machine access + machine-id could brute-force if identity.enc leaks
  - ChaCha20-Poly1305 provides authenticated encryption (detects tampering)
  - But attacker with machine-id can decrypt file contents

- ✅ **Good enough for our threat model**
  - LLM cannot access machine-id (requires parsing `ioreg` or reading `/etc/machine-id`)
  - LLM cannot decrypt file without machine-id
  - Users should not treat encrypted file as protection against physical disk theft
  - For that: use macOS FileVault (full disk encryption) or Linux LUKS

**Comparison to Alternatives:**
- **Better than**: `PATINA_IDENTITY` env var (LLM can read via `printenv`)
- **Better than**: Plaintext file (LLM can read via `cat`)
- **Worse than**: macOS Keychain on console (Secure Enclave hardware protection)
- **Same as**: Most password managers on Linux (software encryption, machine-bound)

**Recovery (Machine ID Changes):**

If hardware changes or OS reinstall changes machine-id:
```
Error: Failed to decrypt identity file.

This usually happens after hardware changes or OS reinstall.

Recovery options:
1. Re-import identity: patina secrets --import-key
2. Use PATINA_IDENTITY env var temporarily

The encrypted file is at: ~/.patina/identity.enc
If you have a backup of the identity, re-import it.
```

## Implementation

### Architecture

```rust
// src/secrets/storage.rs (new orchestrator)

pub fn store_identity(identity: &str) -> Result<()> {
    #[cfg(target_os = "macos")]
    {
        // macOS: ALWAYS write both Keychain and encrypted file
        // This ensures automatic migration and SSH fallback
        let keychain_result = keychain::store_identity(identity);
        let file_result = encrypted_file::store_identity(identity);

        // Both succeed: best case
        // One succeeds: acceptable (other is fallback)
        // Both fail: error
    }

    #[cfg(target_os = "linux")]
    {
        // Linux: Only encrypted file available
        encrypted_file::store_identity(identity)
    }
}

pub fn get_identity() -> Result<String> {
    #[cfg(target_os = "macos")]
    {
        // macOS console: Try Keychain first (best security)
        if !is_ssh_session() {
            if let Ok(identity) = keychain::get_identity() {
                return Ok(identity); // Secure Enclave path
            }
        }
        // Fall through to encrypted file
    }

    // Universal fallback: encrypted file
    encrypted_file::get_identity()
}

fn is_ssh_session() -> bool {
    // Conservative detection: default to encrypted file (safe fallback)
    // Only return false (use Keychain) when we're CONFIDENT it's a true console session
    //
    // Rationale: Wrong guess → slower (file instead of Keychain), not broken (-25308)
    // This prevents -25308 errors even if we mis-detect remote contexts
    //
    // Positive signals for TRUE CONSOLE (all must be absent for Keychain):
    // - SSH_CONNECTION (standard SSH)
    // - SSH_TTY (SSH with TTY)
    // - SSH_CLIENT (alternate SSH var)
    // - Future: Check controlling TTY owned by loginwindow (macOS-specific)
    // - Future: Check launchd session type (macOS-specific)
    //
    // Edge cases that should use encrypted file:
    // - mosh sessions (no SSH_* vars, but still remote)
    // - VS Code Remote (may not set SSH_*)
    // - GitHub Actions (CI context)
    // - tmux/screen spawned from SSH (SSH_* may persist)
    //
    // Phase 1: Simple check (good enough for 90% of cases)
    std::env::var("SSH_CONNECTION").is_ok()
        || std::env::var("SSH_TTY").is_ok()
        || std::env::var("SSH_CLIENT").is_ok()

    // Phase 2 (future): Add TTY ownership check for macOS
    // Phase 3 (future): Add launchd session type check for macOS
}
```

### New Files

**Logging Security Rules:**

To prevent secret leakage via logs (including with `PATINA_LOG=1`):

**CRITICAL: Never log secret bytes, only metadata**

```rust
// ✅ CORRECT: Log metadata only
log::info!(
    event = "secrets.get",
    source = "keychain",
    result = "ok",
    identity_length = identity.len()  // OK: length, not content
);

log::info!(
    event = "secrets.store",
    dest = "encrypted_file",
    path = %path.display(),  // OK: file path, not file contents
    result = "ok"
);

// ❌ WRONG: Never log secret values
log::debug!("Retrieved identity: {}", identity);  // LEAKS SECRET!
log::debug!("Encrypted data: {:?}", encrypted);    // Leaks ciphertext (may reveal length patterns)
```

**Structured Log Fields (for testing):**

Instead of grepping human text, emit structured fields:

```rust
// Field naming convention: event.operation
log::info!(
    event = "secrets.get",      // Event type
    source = "keychain",        // Source: "keychain" | "encrypted_file" | "env_var"
    result = "ok",              // Result: "ok" | "failed"
    error = %e                  // Error details if failed (no secrets in error messages)
);

log::info!(
    event = "secrets.store",
    dest = "encrypted_file",
    path = %path.display(),
    result = "ok"
);

log::info!(
    event = "secrets.migrate",
    source = "keychain",
    dest = "encrypted_file",
    reason = "auto_migration",
    result = "ok"
);
```

**Testing with Structured Logs (Staged Approach):**

**Phase 1: Key-Value Pairs (stable field matching)**
```bash
# Match ONLY field names and values, not prose
# ✅ CORRECT: Match structured fields (stable API)
PATINA_LOG=1 patina secrets run -- echo "test" 2>&1 | grep 'event="secrets.get"' | grep 'source="keychain"'

# ❌ WRONG: Grep freeform prose (brittle, changes with text)
PATINA_LOG=1 patina secrets run -- echo "test" 2>&1 | grep "Retrieved from Keychain"

# The log output should look like:
# event="secrets.get" source="keychain" result="ok" identity_length=74
```

**Phase 2: JSON Logs (future, when implemented)**
```bash
# Parse with jq (proper structured parsing)
PATINA_LOG=json patina secrets run -- echo "test" | jq 'select(.event == "secrets.get") | .source'
# Output: "keychain"

# Filter by multiple fields
PATINA_LOG=json patina secrets run -- echo "test" | jq 'select(.event == "secrets.get" and .result == "ok")'
```

**Testing Philosophy:**
- **Phase 1 goal:** Emit key-value pairs, match field names/values only
- **No human prose in tests:** "Retrieved from X" can change, `source="X"` is stable
- **Stable API contract:** Field names (`event`, `source`, `result`) are the API
- **Phase 2 goal:** Full JSON with proper structured parsing (jq, not grep)

**`src/secrets/encrypted_file.rs`** (~200 lines):
```rust
use chacha20poly1305::{aead::Aead, ChaCha20Poly1305, KeyInit, Nonce};
use hkdf::Hkdf;
use sha2::Sha256;

const SALT_LEN: usize = 32;
const NONCE_LEN: usize = 12;

pub fn store_identity(identity: &str) -> Result<()> {
    // 1. Generate random salt (32 bytes)
    // 2. Generate random nonce (12 bytes)
    // 3. Get machine ID (platform-specific)
    // 4. Derive key via HKDF-SHA256(machine_id, salt, "patina-identity-v1")
    // 5. Encrypt with ChaCha20-Poly1305
    // 6. Write [b"PATINA"][0x01][salt][nonce][ciphertext+tag] to ~/.patina/identity.enc
    //    ^^^^^^^^^^^^^^^^^^ CRITICAL: Must include header (7 bytes)
}

pub fn get_identity() -> Result<String> {
    // 1. Read ~/.patina/identity.enc
    // 2. Verify magic header: data[0..6] == b"PATINA"
    // 3. Check version: data[6] == 0x01 (reject if > 0x01)
    // 4. Parse payload: [salt][nonce][ciphertext] from data[7..]
    // 5. Get current machine ID
    // 6. Derive key via HKDF-SHA256(machine_id, salt, "patina-identity-v1")
    // 7. Decrypt with ChaCha20-Poly1305
    // 8. Return plaintext (or recovery error if machine ID changed)
}

fn get_machine_id() -> Result<Vec<u8>> {
    // Platform-specific stable identifier
}

fn derive_key(machine_id: &[u8], salt: &[u8]) -> [u8; 32] {
    // HKDF-SHA256 key derivation
}
```

**File Safety (Critical Implementation Details):**

To prevent partial writes, permission leaks, and concurrent corruption:

1. **Directory Creation:**
   ```rust
   let patina_dir = dirs::home_dir()
       .ok_or_else(|| anyhow!("Cannot determine home directory"))?
       .join(".patina");

   // Create with restrictive permissions (owner-only)
   std::fs::create_dir_all(&patina_dir)?;
   #[cfg(unix)]
   {
       use std::os::unix::fs::PermissionsExt;
       let mut perms = std::fs::metadata(&patina_dir)?.permissions();
       perms.set_mode(0o700); // drwx------ (owner-only)
       std::fs::set_permissions(&patina_dir, perms)?;
   }
   ```

2. **Atomic Write (Prevent Corruption):**
   ```rust
   pub fn store_identity(identity: &str) -> Result<()> {
       let final_path = identity_enc_path(); // ~/.patina/identity.enc
       let temp_path = final_path.with_extension("enc.tmp"); // ~/.patina/identity.enc.tmp

       // Write to temp file first
       let mut file = std::fs::OpenOptions::new()
           .write(true)
           .create(true)
           .truncate(true)
           .open(&temp_path)?;

       // Set restrictive permissions before writing (owner read/write only)
       #[cfg(unix)]
       {
           use std::os::unix::fs::PermissionsExt;
           let mut perms = file.metadata()?.permissions();
           perms.set_mode(0o600); // -rw------- (owner-only)
           file.set_permissions(perms)?;
       }

       // Write encrypted data (includes PATINA header + version + payload)
       // Format: [b"PATINA"][0x01][salt][nonce][ciphertext+tag]
       file.write_all(&encrypted_data)?;

       // Ensure data is on disk before rename (prevent corruption on crash)
       file.sync_all()?;
       drop(file); // Close before rename

       // Atomic rename (replaces old file, or creates new)
       std::fs::rename(&temp_path, &final_path)?;

       Ok(())
   }
   ```

3. **Concurrent Access Protection:**
   ```rust
   use std::fs::OpenOptions;

   pub fn store_identity(identity: &str) -> Result<()> {
       // Option A: Advisory lock (allows concurrent readers)
       #[cfg(unix)]
       {
           use std::os::unix::fs::OpenOptionsExt;
           use nix::fcntl::{flock, FlockArg};

           let lock_path = identity_enc_path().with_extension("lock");
           let lock_file = OpenOptions::new()
               .write(true)
               .create(true)
               .open(&lock_path)?;

           // Exclusive lock (blocks other writers)
           flock(lock_file.as_raw_fd(), FlockArg::LockExclusive)?;

           // ... perform write ...

           // Lock released when lock_file drops
       }

       // Option B: Fail-fast on concurrent setup (simpler)
       // Let atomic rename handle races (last writer wins)
       // This is acceptable since setup-claude is rare
   }
   ```

   **Decision:** Use Option B (fail-fast) for Phase 1 simplicity.
   - `setup-claude` is rare (once per machine)
   - Atomic rename ensures consistency (no partial writes)
   - If two setups race, last one wins (both write same data anyway)
   - Future: Add locking if concurrent corruption is observed

4. **Permission Validation on Read:**
   ```rust
   pub fn get_identity() -> Result<String> {
       let path = identity_enc_path();

       // Verify file permissions (warn if world-readable)
       #[cfg(unix)]
       {
           use std::os::unix::fs::PermissionsExt;
           let perms = std::fs::metadata(&path)?.permissions();
           let mode = perms.mode() & 0o777;
           if mode & 0o077 != 0 {
               log::warn!(
                   event = "secrets.permissions",
                   path = %path.display(),
                   mode = format!("{:o}", mode),
                   expected = "0600",
                   warning = "File permissions too permissive (should be -rw-------)"
               );
           }
       }

       // ... proceed with read ...
   }
   ```

**File Safety Checklist:**
- [x] Directory created with 0700 (owner-only)
- [x] File written with 0600 (owner read/write only)
- [x] Write to temp file first (`.enc.tmp`)
- [x] `fsync()` before rename (prevent corruption on crash)
- [x] Atomic rename (temp → final)
- [x] Permission validation on read (warn if too permissive)
- [ ] Concurrent write locking (deferred to Phase 2 if needed)

### Modified Files

**`src/secrets/keychain.rs`**:
- Keep existing implementation (no changes)
- Used only on macOS console via storage.rs orchestrator
- **Important constants** (referenced in tests/examples):
  - `KEYCHAIN_SERVICE = "patina"` (line 25)
  - `KEYCHAIN_ACCOUNT = "Patina Secrets"` (line 27)
  - If these change, update test examples that hardcode service/account names

**`src/secrets/identity.rs`**:
- Update to use `storage::get_identity()` instead of `keychain::get_identity()`
- Keep PATINA_IDENTITY env var as escape hatch

**`src/commands/secrets/setup_claude.rs`**:
- Update UX to show dual-write on macOS
- Show file-only on Linux

### Dependencies

```toml
# Cargo.toml
[dependencies]
chacha20poly1305 = "0.10"  # RustCrypto, well-audited
hkdf = "0.12"               # Key derivation
sha2 = "0.10"               # HKDF hash function
rand = "0.8"                # Secure random for salt/nonce

# Existing (no changes)
security-framework = "2.9"  # macOS Keychain
age = "0.10"                # Identity format
```

### Migration Strategy

**Automatic Migration Strategy (Two Safety Nets):**

Existing macOS users have Keychain-only entries. To make SSH work immediately without manual re-setup:

**Safety Net 1: Proactive Migration (setup commands)**
- **When:** User runs `patina secrets setup-claude` or `--import-key` after upgrade
- **Action:** Always write BOTH Keychain and encrypted file
- **Benefit:** Ensures dual-storage for any explicit setup action

**Safety Net 2: Eager On-Demand Migration (first console use)**
- **When:** First time `get_identity()` is called from console after upgrade
- **Trigger:** Keychain succeeds + `~/.patina/identity.enc` doesn't exist
- **Action:** Immediately persist to encrypted file

```rust
pub fn get_identity() -> Result<String> {
    #[cfg(target_os = "macos")]
    {
        if !is_ssh_session() {
            if let Ok(identity) = keychain::get_identity() {
                // On-demand migration: if encrypted file missing, create it now
                let enc_path = identity_enc_path();
                if !enc_path.exists() {
                    // Log metadata only (never log secret bytes)
                    log::info!(
                        event = "secrets.migrate",
                        source = "keychain",
                        dest = "encrypted_file",
                        reason = "auto_migration"
                    );

                    // Write encrypted file (side effect in getter)
                    if let Err(e) = encrypted_file::store_identity(&identity) {
                        // Log failure but continue (Keychain still works)
                        log::warn!(
                            event = "secrets.migrate",
                            result = "failed",
                            error = %e
                        );
                    } else {
                        log::info!(
                            event = "secrets.migrate",
                            result = "ok",
                            path = %enc_path.display()
                        );
                    }
                }
                return Ok(identity); // Secure Enclave path
            }
        }
    }

    // Universal fallback: encrypted file
    encrypted_file::get_identity()
}
```

**Safety Net 3: SSH-First Detection (clear guidance)**
- **When:** SSH session calls `get_identity()` and encrypted file doesn't exist
- **Trigger:** SSH context detected (via `is_ssh_session()`) + file missing
- **Action:** Show clear guidance (covers both migration and initial setup)

**Note:** Cannot use `security find-generic-password` to detect Keychain existence
from SSH, because that command also fails with -25308 (same security policy).
Instead, show helpful guidance unconditionally when file missing in SSH context.

```rust
// In encrypted_file::get_identity() when file missing
pub fn get_identity() -> Result<String> {
    let path = identity_enc_path();

    if !path.exists() {
        #[cfg(target_os = "macos")]
        {
            // If in SSH context, show migration/setup guidance
            // Note: Can't detect if Keychain exists from SSH (security blocks it)
            if is_ssh_session() {
                bail!(
                    r#"Encrypted identity file not found: ~/.patina/identity.enc

You're connecting via SSH, but the encrypted identity file doesn't exist yet.

SETUP REQUIRED (one-time):
  Run any patina secrets command from a local console (not SSH):
    patina secrets setup-claude
  OR:
    patina secrets run -- echo "setup complete"

  This will create the encrypted file for SSH access.

Why: macOS Keychain blocks SSH access (security policy).
     Dual-storage gives you Keychain on console, encrypted file over SSH.

If you've already setup secrets:
  - This is likely your first SSH use after upgrading to dual-storage
  - Running from console once will migrate your existing Keychain entry

See: layer/surface/build/feat/spec-secrets-dual-storage/SPEC.md
"#
                );
            }
        }

        bail!("Encrypted identity file not found: {}", path.display());
    }

    // ... continue with decryption ...
}
```

**Rationale:**
- **Safety Net 1 (proactive)**: Covers users who run setup commands after upgrade
- **Safety Net 2 (eager)**: Covers users who run console commands first (auto-migrates)
- **Safety Net 3 (guidance)**: Covers SSH-first users with clear one-time instructions
- **Side effect in getter**: Acceptable for UX benefit (transparent migration)
- **Safe failure**: If migration fails, Keychain still works on console
- **Logged**: Structured logs show what happened (not secret bytes)

**Future `setup-claude` and `--import-key`:**
After migration, these commands write to BOTH Keychain and encrypted file:

**User-visible changes:**
```diff
# Before (macOS)
✓ Stored in macOS Keychain (Touch ID protected)

# After (macOS) - setup-claude
✓ Stored in Keychain (Secure Enclave)
✓ Stored encrypted file (SSH/fallback)

# After (macOS) - auto-migration
# (No visible output, happens during first get_identity() call)

# Linux (new capability)
✓ Stored encrypted file
```

### Upgrade Notes (for existing users)

**For teams upgrading to dual-storage:**

After upgrading Patina to dual-storage secrets, run any `patina secrets` command **once from a local console** (not SSH) to migrate:

```bash
# Any of these will trigger automatic migration:
patina secrets run -- echo "migration complete"
patina secrets setup-claude  # Re-setup (safest)
patina secrets --import-key   # Re-import from backup
```

**What happens:**
- Your existing Keychain identity is copied to encrypted file (`~/.patina/identity.enc`)
- Console workflows continue using Keychain (Secure Enclave)
- SSH workflows now use encrypted file (no more -25308 errors)
- No data loss, no re-authentication needed

**Proactive recommendation:**
Include this in deployment/upgrade documentation so teams migrate before first SSH use.

### Setup UX

**macOS:**
```bash
$ patina secrets setup-claude
Storage strategy:
  • Keychain (Secure Enclave) - console access
  • Encrypted file - SSH/fallback access

Token: <paste>
✓ Stored in Keychain (Secure Enclave)
✓ Stored encrypted file (SSH/fallback)

Setup complete. Ready for console and SSH workflows.
```

**Linux:**
```bash
$ patina secrets setup-claude
Storage: Encrypted file (~/.patina/identity.enc)
Machine-bound: Encrypted with /etc/machine-id

Token: <paste>
✓ Stored encrypted file

Setup complete.
```

## Testing

### Exit Criteria

**1. macOS console still works (no regression)**
```bash
# Should use Keychain (Secure Enclave)
# Match structured fields only (not prose)
PATINA_LOG=1 patina secrets run -- echo "console" 2>&1 \
  | grep 'event="secrets.get"' \
  | grep 'source="keychain"' \
  | grep 'result="ok"'

# Expected log line (key-value pairs):
# event="secrets.get" source="keychain" result="ok" identity_length=74
```

**2. macOS SSH works (new capability)**
```bash
# Should use encrypted file (not Keychain, which fails over SSH)
# Match structured fields only
ssh localhost 'export PATINA_LOG=1; patina secrets run -- echo "ssh"' 2>&1 \
  | grep 'event="secrets.get"' \
  | grep 'source="encrypted_file"' \
  | grep 'result="ok"'

# Expected log line:
# event="secrets.get" source="encrypted_file" result="ok" identity_length=74

# Exit code: 0 (success, not -25308 error like before)
```

**3. Linux works (new platform)**
```bash
# On Linux VM
patina secrets setup-claude  # Setup succeeds
patina secrets run -- printenv PATINA_IDENTITY  # Should be empty (not in env)
patina secrets run -- echo "linux"  # Should inject secrets
```

**4. LLM-safe (threat model validation)**
```bash
# File is encrypted
cat ~/.patina/identity.enc  # Gibberish, not plaintext

# Not in environment
env | grep PATINA_IDENTITY  # Empty (unless explicitly set)

# Only in subprocess
patina secrets run -- env | grep -c "vault-injected"  # Count > 0
```

**5. Migration automatic (existing macOS users)**
```bash
# User with existing Keychain identity
patina secrets --export-key --stdout | patina secrets --import-key

# Should create both:
ls ~/.patina/identity.enc  # Exists
security find-generic-password -s "patina" -a "Patina Secrets"  # Exists
# Note: Service/account from KEYCHAIN_SERVICE and KEYCHAIN_ACCOUNT constants
# (src/secrets/keychain.rs lines 25, 27)

# Both work:
patina secrets run -- echo "console"  # Keychain
ssh localhost 'patina secrets run -- echo "ssh"'  # File
```

**6. Recovery documented (machine-id change)**
```bash
# Simulate machine-id change
echo "fake-machine-id" > /tmp/fake-machine-id
# Modify code to use /tmp/fake-machine-id

patina secrets run -- echo "test"
# Output: Error: Failed to decrypt identity file.
#         Recovery options:
#         1. Re-import identity: patina secrets --import-key
```

**7. Cross-platform consistency**
```bash
# Same encrypted file works on both platforms
# (if machine-id is the same - useful for testing)

# macOS: Create encrypted file
patina secrets --export-key --stdout > /tmp/identity.txt
cat /tmp/identity.txt | patina secrets --import-key

# Copy ~/.patina/identity.enc to Linux VM
# Linux: Should decrypt (if we mock same machine-id)
```

### Manual Testing Checklist

- [ ] macOS console: Keychain retrieval works
- [ ] macOS console: Encrypted file retrieval works (if Keychain disabled)
- [ ] macOS SSH: Encrypted file retrieval works
- [ ] macOS: Both files written during setup
- [ ] Linux: Encrypted file written during setup
- [ ] Linux: Encrypted file retrieval works
- [ ] LLM safety: `cat` shows encrypted data
- [ ] LLM safety: `env` doesn't show secret
- [ ] Recovery: Clear error on decryption failure
- [ ] Migration: Existing Keychain users get file automatically

## Security Review

**Threat Model Coverage:**

| Threat | Mitigation | Status |
|--------|------------|--------|
| LLM runs `printenv` | Not in environment | ✅ Mitigated |
| LLM runs `cat ~/.patina/identity.enc` | Encrypted with machine key | ✅ Mitigated |
| LLM includes secret in prompt | Only in subprocess, not parent | ✅ Mitigated |
| Secret in git | Never written to project dir | ✅ Mitigated |
| Stolen laptop (macOS) | Secure Enclave encryption | ✅ Mitigated |
| Stolen disk (Linux) | Machine-bound encryption | ⚠️ Partial |

**Known Limitations:**

1. **Linux disk encryption weaker than macOS Secure Enclave**
   - Mitigation: Still better than plaintext PATINA_IDENTITY
   - Future: Could use TPM 2.0 if available

2. **Machine-id changes break decryption**
   - Mitigation: Clear error message with recovery steps
   - User must re-import from backup

3. **Not protecting against root access**
   - Threat model: LLM leakage, not system compromise
   - Root can read memory, keychain, encrypted file anyway

**Crypto Dependencies:**
- `chacha20poly1305`: RustCrypto, used in many production systems
- `hkdf`: Standard key derivation (RFC 5869)
- `sha2`: SHA-256, NIST standard

## Documentation Updates

**CLAUDE.md:**
```markdown
## Secret Storage Strategy

**macOS Console**: Keychain (Secure Enclave hardware encryption)
**macOS SSH**: Encrypted file (`~/.patina/identity.enc`)
**Linux**: Encrypted file (`~/.patina/identity.enc`)

### How It Works
- LLM never sees secrets (encrypted at rest, injected only into subprocess)
- File encrypted with machine-specific key (bound to hardware)
- Automatic dual-write on macOS (best + fallback)

### Setup
```bash
# macOS or Linux
patina secrets setup-claude  # Prompts for token, stores securely
```

### Recovery
If encrypted file fails after hardware change:
```bash
patina secrets --import-key  # Re-import from backup
```

### Manual Override (Not Recommended)
```bash
export PATINA_IDENTITY="AGE-SECRET-KEY-1..."  # Visible to LLM!
```
```

## Success Metrics

1. **SSH workflows unblocked**: spec-launcher-auth can proceed
2. **Linux support**: Patina works on Linux for first time
3. **Zero LLM leakage**: No reports of secrets in LLM context
4. **Migration seamless**: Existing users don't notice change
5. **Clear security story**: Can explain to users why it's safe

## Implementation Details Summary

This section captures decisions made during spec review (sessions 20260222-132656):

### Round 1: Initial Implementation Details

Decisions from first agent review:

### 1. SSH Detection Strategy (Conservative)
- **Default to encrypted file** (safe fallback)
- Only use Keychain when confident it's true console (no SSH_* vars)
- Prevents -25308 even if we mis-detect remote contexts
- Phase 1: Check `SSH_CONNECTION`, `SSH_TTY`, `SSH_CLIENT`
- Future: Add TTY ownership and launchd session checks (macOS)

### 2. Machine ID Robustness
- **macOS:** IOPlatformUUID → error (no fallback - /var/lib/dbus doesn't exist)
- **Linux:** /etc/machine-id → /var/lib/dbus/machine-id → error
- **Validation:** Non-empty + format check (UUID or 32 hex chars)
- **Never generate:** Error if all sources fail (defeats machine-binding)
- **Security:** Acceptable to reuse machine-id as HKDF IKM for LLM threat model

### 3. Migration Strategy (On-Demand)
- **Eager migration:** When Keychain succeeds + encrypted file missing → write it
- **Side effect in getter:** Acceptable for UX (unblocks SSH immediately)
- **Log metadata only:** Never log secret bytes during migration
- **Safe failure:** If migration fails, Keychain still works on console

### 4. File Safety
- **Directory:** `mkdir -p ~/.patina` with 0700 permissions
- **Temp write:** Write to `.identity.enc.tmp` with 0600 permissions
- **Atomic:** `fsync()` + `rename()` to final path
- **Locking:** Deferred to Phase 2 (atomic rename sufficient for rare setup)
- **Validation:** Warn on read if permissions too permissive

### 5. Logging Security
- **CRITICAL:** Never log secret bytes, only metadata
- **Structured fields:** `event="secrets.get" source="keychain" result="ok"`
- **Testing:** Match fields instead of grepping human text
- **Allowed:** source, operation, status, error messages (no secrets)
- **Forbidden:** identity value, decrypted data, encryption keys

### Round 2: Format Versioning & Edge Cases

Decisions from second agent review (same session):

### 6. File Format Versioning
- **Magic header:** `b"PATINA\x01"` (6 bytes magic + 1 byte version)
- **Version 1:** Current format (HKDF-SHA256 + ChaCha20-Poly1305)
- **Future-proof:** Can detect version, reject unknown, migrate old formats
- **Simple:** No length fields or complex headers (can add in v2 if needed)
- **Early rejection:** Check magic + version before attempting decryption

### 7. macOS Fallback Correction
- **No /var/lib/dbus fallback on macOS:** That path doesn't exist (Linux-only)
- **Single source:** IOPlatformUUID only (canonical machine ID)
- **Fail fast:** Error with clear guidance if ioreg fails
- **Honest design:** Don't pretend we have fallback that silently fails

### 8. Migration Safety Nets (Three Layers)
- **Net 1 (proactive):** Run migration in setup-claude and --import-key
- **Net 2 (eager):** Auto-migrate when console get_identity() succeeds + file missing
- **Net 3 (guidance):** Detect Keychain exists from SSH, show clear one-time migration prompt
- **Upgrade notes:** Document one-time "run locally once" requirement for teams

### 9. Structured Log Testing (Staged)
- **Phase 1:** Key-value pairs (`event="secrets.get" source="keychain"`)
- **Test by field matching:** Not freeform prose (stable API contract)
- **Phase 2:** JSON logs with jq parsing (future)
- **Philosophy:** Field names are the API, human text can change

## Related Work

**Replaces:**
- spec-keychain-macos26-regression (impossible to fix)
- spec-secrets-keychain-ssh (never actually worked)

**Unblocks:**
- spec-launcher-auth (needs working SSH secret access)

**Future Extensions:**
- TPM 2.0 support on Linux (hardware-backed like macOS)
- Encrypted file sync via Mother (cross-machine secrets)
- FIDO2/WebAuthn for additional auth layer
