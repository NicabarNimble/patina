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

## The Solution: Layered age-Encrypted Vaults

```
┌─────────────────────────────────────────────────────────────┐
│  Identity (your private key)                                │
│  ├── PATINA_IDENTITY env var (CI/headless)                 │
│  └── macOS Keychain "Patina Secrets" (Touch ID protected)  │
└─────────────────────────────────────────────────────────────┘
                              │
                              │ decrypt
                              ▼
┌─────────────────────────────────────────────────────────────┐
│  Global Vault (personal)        Project Vault (team)        │
│  ~/.patina/                     .patina/                    │
│  ├── secrets.toml               ├── secrets.toml            │
│  ├── recipient.txt  (you)       ├── recipients.txt (team)   │
│  └── vault.age                  └── vault.age               │
└─────────────────────────────────────────────────────────────┘
                              │
                              │ merge (project overrides global)
                              ▼
┌─────────────────────────────────────────────────────────────┐
│  patina secrets run -- cargo test                           │
│  patina secrets run --ssh server -- ./deploy.sh            │
└─────────────────────────────────────────────────────────────┘
```

**Key insights:**
- **Mac is the trust boundary.** Containers and servers receive secrets via injection - they never have the key.
- **Two vaults, merged at runtime.** Global for personal secrets, project for team secrets.
- **Project vault in git.** Encrypted, safe to commit. Travels with repo. CI just works.
- **Multi-recipient.** Team members and CI each have their own identity.

---

## Coverage Matrix

| Scenario | v2 Local Vault |
|----------|----------------|
| Local Mac (Touch ID) | ✓ Same great UX |
| Multiple terminals | ✓ Session cache (no repeated prompts) |
| Docker containers | ✓ Mac injects, container receives |
| CI/CD | ✓ Vault in repo + identity in CI secrets |
| Headless dev | ✓ PATINA_IDENTITY env var |
| Team sharing | ✓ Multi-recipient encryption |
| Requires account | ✗ No account needed |
| FOSS | ✓ Fully open source |

---

## Command Surface

**Core commands + recipient management.**

```
patina secrets                     # Status (both vaults)
patina secrets add NAME [--global] # Add secret (project vault by default)
patina secrets run [--ssh H] -- C  # Merge vaults, inject, execute
```

### Flags on `patina secrets`

```
--remove NAME        Remove a secret (from project or global)
--export-key         Print your identity (requires --confirm)
--import-key         Store identity in Keychain
--lock               Clear session cache
```

### Recipient Management (Project Vault)

```
patina secrets add-recipient KEY      # Add team member or CI
patina secrets remove-recipient KEY   # Remove recipient, re-encrypt
patina secrets list-recipients        # Show who can decrypt
```

---

## Behavior Rules

| Rule | Behavior |
|------|----------|
| **Vault selection** | `add` goes to project vault if `.patina/` exists, else global. Use `--global` to force. |
| **Secret merge** | `run` merges global + project vaults. Project overrides global on conflict. |
| **Identity resolution** | `PATINA_IDENTITY` env first (CI/headless), then macOS Keychain (Touch ID). |
| **Auto-init** | Only on `add`. Never on `run` (run must not mutate state). |
| **Env inference** | `github-token` → `GITHUB_TOKEN`. Show default, user can override. |
| **TTY detection** | TTY → prompt for value. No TTY → require `--value` or stdin, fail fast. |
| **Touch ID** | On `run` and `add` (both require decrypt). Only first `add` (init) skips Touch ID. |
| **Session cache** | Via `patina serve` daemon (TTL 10-30 min). Falls back to direct access if serve not running. `--lock` clears. |
| **SSH injection** | Env only - nothing written remotely. |
| **Export safety** | `--export-key --confirm` required. |

---

## File Layout

```
# Global vault (personal secrets, cross-project)
~/.patina/
├── secrets.toml      # Registry: names → env vars
├── recipient.txt     # Your public key (just you)
└── vault.age         # Your encrypted secrets

# Project vault (team secrets, per-project)
project/.patina/
├── secrets.toml      # Registry: names → env vars
├── recipients.txt    # Team public keys (plural!)
└── vault.age         # Team encrypted secrets (commit to git)

# Identity storage
macOS Keychain:
└── "Patina Secrets"  # Your age identity (Touch ID protected)

# Or for CI/headless:
env:
└── PATINA_IDENTITY   # Your age identity (no Touch ID)
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

## Recipients Format (Project Vault)

```
# .patina/recipients.txt
# One age public key per line. Comments allowed.

# Team members
age1alice0qwerty...   # Alice
age1bob00asdfgh...    # Bob

# CI/automation
age1ci000zxcvbn...    # GitHub Actions
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

### Docker/Container Injection

**Mechanism:** Environment variable inheritance. `patina secrets run` sets env vars, spawns the child process, Docker inherits them. No special Docker integration needed.

```bash
# New container - secrets passed via inherited env
$ patina secrets run -- docker run -e GITHUB_TOKEN -e OPENAI_API_KEY myimage ./script.sh
🔐 Touch ID for "Patina Secrets"
✓ Injecting 2 secrets
Running: docker run -e GITHUB_TOKEN -e OPENAI_API_KEY myimage ./script.sh

# Running container - same pattern
$ patina secrets run -- docker exec -e GITHUB_TOKEN mycontainer ./script.sh
✓ Injecting 1 secret (cached)
Running: docker exec -e GITHUB_TOKEN mycontainer ./script.sh

# Docker Compose - inherit from host env
$ patina secrets run -- docker compose run app ./test.sh
✓ Injecting 3 secrets (cached)
Running: docker compose run app ./test.sh
```

**Key insight:** The child command is opaque to `patina secrets run`. It doesn't matter if it's `cargo`, `docker`, `ssh`, or anything else - they all receive secrets the same way: environment variables set before spawn.

**For `docker compose up` (long-running):** Set env vars in compose file to reference host env:

```yaml
# docker-compose.yml
services:
  app:
    environment:
      - GITHUB_TOKEN  # Inherits from host when patina secrets run -- docker compose up
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

### CI Setup (One-Time)

```bash
# 1. Generate CI identity
$ age-keygen
Public key: age1ci000zxcvbn...
# (Copy the AGE-SECRET-KEY-1... line)

# 2. Add to GitHub Secrets as PATINA_IDENTITY
# (Paste the private key)

# 3. Add CI as recipient in project
$ patina secrets add-recipient age1ci000zxcvbn...
✓ Re-encrypted vault for 2 recipients

# 4. Commit
$ git add .patina/recipients.txt .patina/vault.age
$ git commit -m "Add CI to secrets recipients"
```

Now CI just works:
```yaml
# .github/workflows/test.yml
env:
  PATINA_IDENTITY: ${{ secrets.PATINA_IDENTITY }}
steps:
  - uses: actions/checkout@v4
  - run: patina secrets run -- cargo test
```

### Team Onboarding

```bash
# New team member generates identity
$ alice: age-keygen
Public key: age1alice...
# Alice shares public key (safe to share)

# Existing member adds Alice
$ patina secrets add-recipient age1alice...
✓ Re-encrypted vault for 3 recipients

$ git add .patina/recipients.txt .patina/vault.age
$ git commit -m "Add Alice to secrets"
$ git push

# Alice pulls and can now decrypt
$ alice: git pull
$ alice: patina secrets run -- cargo test
🔐 Touch ID for "Patina Secrets"
✓ Injecting 3 secrets
```

### Team Offboarding

```bash
$ patina secrets remove-recipient age1alice...
✓ Re-encrypted vault for 2 recipients

$ git commit -am "Remove Alice from secrets"
$ git push

# Note: Alice can still see old secrets from git history
# Rotate secrets if needed for security
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

### Identity Resolution

```rust
fn get_identity() -> Result<String> {
    // 1. Check env first (CI/headless path)
    if let Ok(identity) = std::env::var("PATINA_IDENTITY") {
        return Ok(identity);
    }

    // 2. Fall back to Keychain (Mac with Touch ID)
    get_identity_from_keychain()
}
```

This enables:
- **CI/headless:** Set `PATINA_IDENTITY` env var, no Keychain needed
- **Mac:** Touch ID via Keychain, great UX
- **Portable:** Same code works everywhere

### Vault Resolution

```rust
fn get_vault_paths() -> (Option<PathBuf>, Option<PathBuf>) {
    let global = home_dir().map(|h| h.join(".patina/vault.age"));
    let project = find_project_root().map(|p| p.join(".patina/vault.age"));
    (global, project)
}

fn load_secrets() -> Result<HashMap<String, String>> {
    let mut secrets = HashMap::new();

    // Load global first
    if let Some(global) = get_global_vault()? {
        secrets.extend(global);
    }

    // Project overrides global
    if let Some(project) = get_project_vault()? {
        secrets.extend(project);
    }

    Ok(secrets)
}
```

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

**Problem:** `patina secrets run` is a short-lived process. Each invocation is new - can't share in-memory state.

**Solution:** Leverage existing `patina serve` daemon as a passive cache. `secrets run` always handles decryption (Touch ID in foreground). `serve` never triggers Touch ID - it only stores what clients give it.

```
┌─────────────────────────────────────────────────────────────┐
│  patina secrets run                                          │
│  1. Check serve for cached values                            │
│  2. Cache hit → use cached values (no Touch ID)             │
│  3. Cache miss → decrypt locally (Touch ID), send to serve  │
│  4. Inject env vars, run command                            │
└─────────────────────────────────────────────────────────────┘
                              │
                              │ localhost:50051
                              ▼
┌─────────────────────────────────────────────────────────────┐
│  patina serve (passive cache only)                           │
│  └── GET /secrets/cache → return cached values or 404       │
│  └── POST /secrets/cache → store values with TTL            │
│  └── POST /secrets/lock → clear cache                       │
└─────────────────────────────────────────────────────────────┘
```

**Behavior:**
1. `secrets run` checks serve for cached values
2. Cache hit: use cached values, no Touch ID
3. Cache miss: decrypt locally (Touch ID), cache in serve for next time
4. serve not running: decrypt locally (Touch ID), no caching
5. `patina secrets --lock`: clears cache in serve (if running)

**Why this design:**
- Touch ID always in foreground process (reliable, no GUI issues for background daemon)
- serve stays simple (no Keychain access, no identity handling)
- Graceful degradation: works without serve, just more Touch ID prompts
- unix-philosophy: reuse existing infrastructure, single responsibility

### Encrypt Without Touch ID

Store recipients (public keys) in plaintext. Encrypt path doesn't need identity:

```rust
fn add_secret(name: &str, value: &str, env: Option<&str>, global: bool) -> Result<()> {
    let vault_path = if global { global_vault_path() } else { project_vault_path()? };

    // Read recipients from file (no Touch ID)
    // Global: recipient.txt (singular, just you)
    // Project: recipients.txt (plural, team)
    let recipients = read_recipients(&vault_path)?;

    // Decrypt current vault (Touch ID or PATINA_IDENTITY)
    let mut vault = decrypt_vault(&vault_path)?;

    // Add new secret
    vault.values.insert(name.to_string(), value.to_string());

    // Encrypt for all recipients (no Touch ID)
    encrypt_vault(&vault, &recipients)?;

    // Update registry
    update_registry(&vault_path, name, env)?;

    Ok(())
}
```

**Note: Add requires Touch ID (by design)**

Adding a secret requires decrypt→modify→re-encrypt. This means Touch ID is triggered except for the very first secret (vault creation). This is intentional:

- **Add is a privileged operation** - Touch ID confirms you're the one modifying the vault
- **Add is infrequent** - You add secrets occasionally, you *run* secrets frequently
- **Optimize for run, not add** - Session caching benefits the frequent operation
- **Matches 1Password behavior** - Every vault modification triggered biometric auth

The only Touch ID-free add is the first secret (init), since there's no existing vault to decrypt.

---

## Module Structure

```
src/secrets/
├── mod.rs           # Public API (thin facade)
├── identity.rs      # Identity resolution (env → Keychain)
├── vault.rs         # age encrypt/decrypt, multi-recipient
├── keychain.rs      # macOS Keychain access
├── session.rs       # Session cache (via serve daemon)
├── registry.rs      # secrets.toml parsing
└── recipients.rs    # recipients.txt parsing

# Delete:
└── internal.rs      # 1Password logic (archived)
```

### Public API

```rust
// src/secrets/mod.rs

pub use vault::VaultStatus;
pub use registry::SecretsRegistry;

/// Check vault status (both global and project)
pub fn check_status(project_root: Option<&Path>) -> Result<VaultStatus>;

/// Add a secret (project vault by default, --global for global)
pub fn add_secret(name: &str, value: &str, env: Option<&str>, global: bool) -> Result<()>;

/// Remove a secret
pub fn remove_secret(name: &str, global: bool) -> Result<()>;

/// Run command with secrets injected (merges global + project)
pub fn run_with_secrets(project_root: Option<&Path>, command: &[String]) -> Result<i32>;

/// Run command on remote host via SSH
pub fn run_with_secrets_ssh(project_root: Option<&Path>, host: &str, command: &[String]) -> Result<i32>;

/// Clear session cache
pub fn lock_session() -> Result<()>;

/// Export identity (for backup/recovery)
pub fn export_identity() -> Result<String>;

/// Import identity (for new machine setup)
pub fn import_identity(identity: &str) -> Result<()>;

// Recipient management (project vault only)

/// Add recipient to project vault
pub fn add_recipient(project_root: &Path, recipient: &str) -> Result<()>;

/// Remove recipient from project vault
pub fn remove_recipient(project_root: &Path, recipient: &str) -> Result<()>;

/// List recipients for project vault
pub fn list_recipients(project_root: &Path) -> Result<Vec<String>>;
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
| Unauthorized team access | Must be added as recipient |
| CI credential theft | Separate CI identity, can be rotated |

### NOT Protected Against

| Threat | Why |
|--------|-----|
| Malware while session unlocked | Same as any password manager |
| Memory scraping after decrypt | Secrets exist in memory during use |
| Compromised Keychain access | If attacker has Touch ID, game over |
| Removed team member + git history | Old secrets visible in git history - rotate if needed |

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
| CI/CD support | Broken | Vault in repo + identity in CI secrets |
| Team support | Via 1Password sharing | Multi-recipient encryption |
| Vault scope | N/A | Global (personal) + Project (team) |
| FOSS | No | Yes |

---

## Acceptance Criteria

### Core Functionality
1. [ ] `patina secrets add NAME` creates project vault on first use (if in project)
2. [ ] `patina secrets add NAME --global` creates/uses global vault
3. [ ] `patina secrets add NAME` prompts for value (TTY) or reads stdin/`--value`
4. [ ] `patina secrets run -- CMD` merges global + project vaults
5. [ ] `patina secrets run -- CMD` fails fast if no vaults exist
6. [ ] Project secrets override global secrets on name conflict

### Identity & Caching
7. [ ] `PATINA_IDENTITY` env var bypasses Keychain (for CI/headless)
8. [ ] Touch ID triggers on first decrypt (or uses session cache)
9. [ ] Session cache prevents repeated Touch ID within TTL
10. [ ] `patina secrets --lock` clears session cache

### Key Management
11. [ ] `patina secrets --export-key --confirm` prints identity
12. [ ] `patina secrets --import-key` stores identity in Keychain

### Team & Recipients
13. [ ] `patina secrets add-recipient KEY` adds to project recipients.txt
14. [ ] `patina secrets remove-recipient KEY` re-encrypts for remaining recipients
15. [ ] `patina secrets list-recipients` shows project recipients

### Integration
16. [ ] `patina secrets run --ssh HOST -- CMD` injects via SSH
17. [ ] MCP gate validates secrets before serving (unchanged)
18. [ ] CI works with vault in repo + `PATINA_IDENTITY` in secrets

---

## References

- [age encryption](https://github.com/FiloSottile/age) - Simple, modern encryption
- [rage (Rust age)](https://github.com/str4d/rage) - Rust implementation
- [security-framework crate](https://docs.rs/security-framework) - macOS Keychain access
- Archived: `git show spec/secrets-1password:layer/surface/build/spec-secrets-boundary.md`
