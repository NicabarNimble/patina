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
[salt: 32 bytes][nonce: 12 bytes][ciphertext: variable][auth tag: 16 bytes]
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

macOS:
```bash
# IOPlatformUUID from IOKit (survives reinstalls, FileVault recovery)
ioreg -rd1 -c IOPlatformExpertDevice | grep IOPlatformUUID
# Fallback: /var/lib/dbus/machine-id
```

Linux:
```bash
# /etc/machine-id (systemd standard, stable across reboots)
cat /etc/machine-id
# Fallback: /var/lib/dbus/machine-id
```

**Security Properties:**

1. **LLM-safe**: `cat ~/.patina/identity.enc` → encrypted gibberish
2. **Machine-bound**: Encrypted with hardware-specific key
3. **Authenticated**: ChaCha20-Poly1305 prevents tampering
4. **Not in env**: Never stored in environment variables
5. **Subprocess-only**: Secrets injected via `patina secrets run` wrapper

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
    std::env::var("SSH_CONNECTION").is_ok()
}
```

### New Files

**`src/secrets/encrypted_file.rs`** (~200 lines):
```rust
use chacha20poly1305::{aead::Aead, ChaCha20Poly1305, KeyInit, Nonce};
use hkdf::Hkdf;
use sha2::Sha256;

const SALT_LEN: usize = 32;
const NONCE_LEN: usize = 12;

pub fn store_identity(identity: &str) -> Result<()> {
    // 1. Generate random salt
    // 2. Get machine ID
    // 3. Derive key via HKDF-SHA256
    // 4. Encrypt with ChaCha20-Poly1305
    // 5. Write [salt][nonce][ciphertext+tag] to ~/.patina/identity.enc
}

pub fn get_identity() -> Result<String> {
    // 1. Read ~/.patina/identity.enc
    // 2. Parse [salt][nonce][ciphertext]
    // 3. Get current machine ID
    // 4. Derive key via HKDF-SHA256
    // 5. Decrypt with ChaCha20-Poly1305
    // 6. Return plaintext (or recovery error if machine ID changed)
}

fn get_machine_id() -> Result<Vec<u8>> {
    // Platform-specific stable identifier
}

fn derive_key(machine_id: &[u8], salt: &[u8]) -> [u8; 32] {
    // HKDF-SHA256 key derivation
}
```

### Modified Files

**`src/secrets/keychain.rs`**:
- Keep existing implementation (no changes)
- Used only on macOS console via storage.rs orchestrator

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

**Automatic for macOS users:**
1. Next time they run `patina secrets setup-claude` or `--import-key`
2. Store writes to BOTH Keychain and encrypted file
3. Get tries Keychain first (console), falls back to file (SSH)
4. No manual steps required

**User-visible changes:**
```diff
# Before (macOS)
✓ Stored in macOS Keychain (Touch ID protected)

# After (macOS)
✓ Stored in Keychain (Secure Enclave)
✓ Stored encrypted file (SSH/fallback)

# Linux (new capability)
✓ Stored encrypted file
```

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
PATINA_LOG=1 patina secrets run -- echo "console" 2>&1 | grep "Keychain"
# Output: Retrieved from Keychain (Secure Enclave)
```

**2. macOS SSH works (new capability)**
```bash
# Should use encrypted file
ssh localhost 'export PATINA_LOG=1; patina secrets run -- echo "ssh"' 2>&1 | grep "encrypted"
# Output: Retrieved from encrypted file
# Exit: 0 (success, not -25308 error)
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
security find-generic-password -s "patina"  # Exists

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
