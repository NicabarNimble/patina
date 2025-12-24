# Spec: Secrets v2 (Local Vault)

**Status:** Design Complete - Ready for Implementation

**Goal:** Local-first secrets management. No cloud accounts, no SaaS dependencies. Touch ID for UX, age encryption for security, macOS Keychain for key storage.

**Supersedes:** spec-secrets-boundary.md (1Password implementation, archived as `spec/secrets-1password`)

---

## The Problem

The v1 1Password implementation works but has coverage gaps:

| Scenario | 1Password v1 |
|----------|--------------|
| Local Mac (Touch ID) | Works |
| Multiple terminals | Touch ID per terminal |
| Docker containers | Broken (no GUI) |
| CI/CD | Broken (service accounts read-only) |
| Headless dev | Broken |
| Requires account | Yes (1Password.com) |

**Root cause:** 1Password is designed for humans with browsers, not machines with scripts.

---

## The Solution: Local age-Encrypted Vault

```
┌─────────────────────────────────────────────────────────────┐
│  macOS Keychain                                             │
│  └── "Patina Secrets" (age identity, Touch ID protected)   │
│      └── Syncs via iCloud Keychain between Macs            │
└─────────────────────────────────────────────────────────────┘
                              │
                              │ decrypt (Touch ID)
                              ▼
┌─────────────────────────────────────────────────────────────┐
│  ~/.patina/                                                 │
│  ├── secrets.toml    # Registry: names → env vars          │
│  ├── recipient.txt   # Public key (encrypt without Touch ID)│
│  └── vault.age       # Encrypted values                    │
└─────────────────────────────────────────────────────────────┘
                              │
                              │ inject
                              ▼
┌─────────────────────────────────────────────────────────────┐
│  patina secrets run -- cargo test                           │
│  patina secrets run --ssh server -- ./deploy.sh            │
└─────────────────────────────────────────────────────────────┘
```

**Key insight:** Mac is the trust boundary. Containers and servers receive secrets at runtime via injection - they never have the key.

---

## Coverage Matrix

| Scenario | v2 Local Vault |
|----------|----------------|
| Local Mac (Touch ID) | ✓ Same great UX |
| Multiple terminals | ✓ Session cache (no repeated prompts) |
| Docker containers | ✓ Mac injects, container receives |
| CI/CD | ✓ Export key to CI secrets store |
| Headless dev | ✓ Session cache or key in env |
| Requires account | ✗ No account needed |
| FOSS | ✓ Fully open source |

---

## Command Surface

**Minimal. 2 subcommands + flags.**

```
patina secrets                     # Status
patina secrets add NAME            # Add secret (creates vault on first use)
patina secrets run [--ssh H] -- C  # Inject + execute
```

### Flags on `patina secrets`

```
--remove NAME        Remove a secret
--export-key         Print identity (requires --confirm)
--import-key         Store identity in Keychain
--lock               Clear session cache
```

---

## Behavior Rules

| Rule | Behavior |
|------|----------|
| **Auto-init** | Only on `add`. Never on `run` (run must not mutate state). |
| **Env inference** | `github-token` → `GITHUB_TOKEN`. Show default, user can override. |
| **TTY detection** | TTY → prompt for value. No TTY → require `--value` or stdin, fail fast. |
| **Touch ID** | Only on decrypt (run). Encrypt uses stored recipient (no prompt). |
| **Session cache** | In-memory TTL (10-30 min configurable). `--lock` clears. No daemon. |
| **SSH injection** | Env only - nothing written remotely. |
| **Export safety** | `--export-key --confirm` required. |

---

## File Layout

```
~/.patina/
├── secrets.toml      # Registry (plaintext, git-safe)
├── recipient.txt     # Public key (plaintext, git-safe)
└── vault.age         # Encrypted values (git-safe)

macOS Keychain:
└── "Patina Secrets"  # age identity (Touch ID protected)
```

---

## Registry Format

```toml
# ~/.patina/secrets.toml
version = 1

[secrets]
github-token = { env = "GITHUB_TOKEN" }
openai-key = { env = "OPENAI_API_KEY" }
database-url = { env = "DATABASE_URL" }
```

---

## Vault Format (Decrypted)

```toml
# Inside vault.age when decrypted
[meta]
version = 1
created_at = "2024-12-24T12:00:00Z"
modified_at = "2024-12-24T14:30:00Z"

[values]
github-token = "ghp_xxxxxxxxxxxx"
openai-key = "sk-proj-xxxxxxxx"
database-url = "postgres://user:pass@host/db"
```

---

## UX Flows

### First Secret (Auto-Init)

```bash
$ patina secrets add github-token
Vault not found. Creating...
✓ Generated encryption key
✓ Stored in macOS Keychain (Touch ID protected)
✓ Saved public key to ~/.patina/recipient.txt

Env var [GITHUB_TOKEN]:
Value: ********
✓ Added github-token → GITHUB_TOKEN
```

### Run (Cached Session)

```bash
$ patina secrets run -- cargo test
🔐 Touch ID for "Patina Secrets"
✓ Injecting 3 secrets
Running: cargo test
...

$ patina secrets run -- cargo build   # Within TTL
✓ Injecting 3 secrets (cached)
Running: cargo build
```

### Run Before Init (Fail Fast)

```bash
$ patina secrets run -- cargo test
✗ Vault not initialized
  Run: patina secrets add <name> to create vault
```

### Non-Interactive Add (LLM/Script)

```bash
# Stdin
$ echo "ghp_xxx" | patina secrets add github-token --env GITHUB_TOKEN
✓ Added github-token → GITHUB_TOKEN

# Flag
$ patina secrets add github-token --env GITHUB_TOKEN --value "ghp_xxx"
✓ Added github-token → GITHUB_TOKEN
```

### SSH Injection

```bash
$ patina secrets run --ssh deploy@prod -- ./restart.sh
🔐 Touch ID for "Patina Secrets"
✓ Injecting 2 secrets via SSH (env only, nothing written remotely)
Running on deploy@prod: ./restart.sh
```

### Export with Safety

```bash
$ patina secrets --export-key
⚠️  This will print your private key.
  Add --confirm to proceed.

$ patina secrets --export-key --confirm
⚠️  PRIVATE KEY - DO NOT SHARE
AGE-SECRET-KEY-1QTGQKZ9K8H7A6J5R3M2N4P8Q7W6E5T4Y3U2I1O0
```

### Import on New Mac

```bash
$ patina secrets --import-key
Paste identity: AGE-SECRET-KEY-1...
✓ Stored in macOS Keychain (Touch ID protected)
```

---

## Technical Design

### age Encryption

[age](https://github.com/FiloSottile/age) is a simple, modern encryption tool:

- **Identity** (private key): `AGE-SECRET-KEY-1...` (~74 chars)
- **Recipient** (public key): `age1...` (~62 chars)

```bash
# Encrypt (uses recipient, no identity needed)
echo "secret" | age -r age1... > secret.age

# Decrypt (uses identity)
age -d -i identity.txt secret.age
```

Rust crate: `age = "0.10"` (the rage implementation, pure Rust).

### macOS Keychain Integration

Store identity as a generic password with Touch ID protection:

```rust
use security_framework::passwords::{set_generic_password, get_generic_password};
use security_framework::access_control::{AccessControl, AccessControlFlags};

// Store with Touch ID requirement
let access = AccessControl::create_with_flags(
    AccessControlFlags::USER_PRESENCE |
    AccessControlFlags::BIOMETRY_CURRENT_SET
)?;
set_generic_password("patina", "Patina Secrets", identity.as_bytes())?;

// Retrieve (triggers Touch ID)
let identity_bytes = get_generic_password("patina", "Patina Secrets")?;
```

### Session Caching

In-memory cache to avoid repeated Touch ID prompts:

```rust
struct SessionCache {
    decrypted_vault: Option<HashMap<String, String>>,
    expires_at: Option<Instant>,
    ttl: Duration,  // Default 10-30 min
}

impl SessionCache {
    fn get_or_decrypt(&mut self) -> Result<&HashMap<String, String>> {
        if self.is_valid() {
            return Ok(self.decrypted_vault.as_ref().unwrap());
        }

        // Triggers Touch ID
        let identity = get_identity_from_keychain()?;
        let vault = decrypt_vault(&identity)?;

        self.decrypted_vault = Some(vault);
        self.expires_at = Some(Instant::now() + self.ttl);

        Ok(self.decrypted_vault.as_ref().unwrap())
    }

    fn lock(&mut self) {
        self.decrypted_vault = None;
        self.expires_at = None;
    }
}
```

### Encrypt Without Touch ID

Store recipient (public key) in plaintext. Encrypt path doesn't need identity:

```rust
fn add_secret(name: &str, value: &str, env: Option<&str>) -> Result<()> {
    // Read recipient from file (no Touch ID)
    let recipient = read_recipient()?;

    // Decrypt current vault (Touch ID)
    let mut vault = decrypt_vault()?;

    // Add new secret
    vault.values.insert(name.to_string(), value.to_string());

    // Encrypt with recipient (no Touch ID)
    encrypt_vault(&vault, &recipient)?;

    // Update registry
    update_registry(name, env)?;

    Ok(())
}
```

Wait - this still needs Touch ID to decrypt the current vault before re-encrypting with the new value. That's unavoidable for updates. But for the very first secret (init), we can avoid it since there's nothing to decrypt.

---

## Module Structure

```
src/secrets/
├── mod.rs           # Public API (thin facade)
├── vault.rs         # age encrypt/decrypt
├── keychain.rs      # macOS Keychain access
├── session.rs       # In-memory TTL cache
└── registry.rs      # secrets.toml parsing

# Delete:
└── internal.rs      # 1Password logic (archived)
```

### Public API

```rust
// src/secrets/mod.rs

pub use vault::VaultStatus;
pub use registry::SecretsRegistry;

/// Check vault status
pub fn check_status() -> Result<VaultStatus>;

/// Add a secret (auto-inits vault on first call)
pub fn add_secret(name: &str, value: &str, env: Option<&str>) -> Result<()>;

/// Remove a secret
pub fn remove_secret(name: &str) -> Result<()>;

/// Run command with secrets injected (fails if uninitialized)
pub fn run_with_secrets(project_root: &Path, command: &[String]) -> Result<i32>;

/// Run command on remote host via SSH
pub fn run_with_secrets_ssh(project_root: &Path, host: &str, command: &[String]) -> Result<i32>;

/// Clear session cache
pub fn lock_session() -> Result<()>;

/// Export identity (for backup/recovery)
pub fn export_identity() -> Result<String>;

/// Import identity (for new machine setup)
pub fn import_identity(identity: &str) -> Result<()>;

/// Load registry
pub fn load_registry() -> Result<SecretsRegistry>;

/// Load project requirements
pub fn load_project_requirements(project_root: &Path) -> Result<Vec<String>>;
```

---

## Dependencies

```toml
[dependencies]
age = "0.10"                    # age encryption (pure Rust)
security-framework = "2.9"      # macOS Keychain access
toml = "0.8"                    # Config parsing
```

---

## Threat Model

### Protected Against

| Threat | Protection |
|--------|------------|
| Disk theft / lost laptop | Vault encrypted at rest |
| Repo leak / git push | vault.age is encrypted |
| Accidental file exposure | Only encrypted blob on disk |
| Cross-terminal prompts | Session cache |

### NOT Protected Against

| Threat | Why |
|--------|-----|
| Malware while session unlocked | Same as any password manager |
| Memory scraping after decrypt | Secrets exist in memory during use |
| Compromised Keychain access | If attacker has Touch ID, game over |

This is the same threat model as 1Password - we don't claim more.

---

## Multi-Mac Sync

**Primary:** iCloud Keychain syncs the identity automatically between Macs.

**Fallback:** Manual export/import if iCloud disabled:

```bash
# Mac A
patina secrets --export-key --confirm > /secure/location/patina.key

# Mac B
cat /secure/location/patina.key | patina secrets --import-key
```

**Vault sync:** Put `~/.patina/` in git dotfiles repo (vault.age is encrypted, safe to commit).

---

## What Changes from v1

| Aspect | v1 (1Password) | v2 (Local Vault) |
|--------|----------------|------------------|
| Backend | 1Password CLI (`op`) | age crate + Keychain |
| Account required | Yes | No |
| `init` command | Explicit subcommand | Auto on first `add` |
| `--item`, `--field` | Map to 1Password | Gone |
| `--env` | Required | Optional (infer from name) |
| Container support | Broken | Works (Mac injects) |
| FOSS | No | Yes |

---

## Acceptance Criteria

1. [ ] `patina secrets add NAME` creates vault on first use
2. [ ] `patina secrets add NAME` prompts for value (TTY) or reads stdin/`--value`
3. [ ] `patina secrets run -- CMD` fails fast if vault uninitialized
4. [ ] `patina secrets run -- CMD` triggers Touch ID (or uses cache)
5. [ ] Session cache prevents repeated Touch ID within TTL
6. [ ] `patina secrets --lock` clears session cache
7. [ ] `patina secrets --export-key --confirm` prints identity
8. [ ] `patina secrets --import-key` stores identity in Keychain
9. [ ] `patina secrets run --ssh HOST -- CMD` injects via SSH
10. [ ] MCP gate validates secrets before serving (unchanged)

---

## References

- [age encryption](https://github.com/FiloSottile/age) - Simple, modern encryption
- [rage (Rust age)](https://github.com/str4d/rage) - Rust implementation
- [security-framework crate](https://docs.rs/security-framework) - macOS Keychain access
- Archived: `git show spec/secrets-1password:layer/surface/build/spec-secrets-boundary.md`
