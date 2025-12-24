# Spec: Secrets Boundary

**Status:** Core v1 Complete (8/9 acceptance criteria)

**Goal:** 1Password as first-class citizen in Patina. Secrets never in chat, never in git, never seen by LLMs.

**Pattern:** Follows the model pattern - mothership holds registry, project declares requirements.

---

## The Problem

```
User: "Deploy with the new API key"
Claude: "Paste it here and I'll run the update command"
```

Keys in chat. Keys in .env. Keys in git history. Keys everywhere.

---

## The Solution: Mothership + 1Password

```
┌─────────────────────────────────────────────────────────────────┐
│                     1PASSWORD "Patina" VAULT                     │
│                                                                  │
│  github-token        → ghp_xxxxx                                │
│  openai-key          → sk-proj-xxxxx                            │
│  tailscale-authkey   → tskey-xxxxx                              │
│  postgres-prod       → postgres://user:pass@...                 │
│                                                                  │
│  (All secrets live here - external to Patina)                   │
│                                                                  │
└─────────────────────────────────────────────────────────────────┘
                              │
                              │ op:// references
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│                     ~/.patina/secrets.toml                       │
│                     (MOTHERSHIP - central registry)              │
│                                                                  │
│  vault = "Patina"                                                │
│                                                                  │
│  [secrets]                                                       │
│  github-token = { item = "github-token" }                       │
│  openai-key = { item = "openai-key" }                           │
│  database-url = { item = "postgres-prod", field = "url" }       │
│                                                                  │
│  (Shared across all projects)                                   │
│                                                                  │
└─────────────────────────────────────────────────────────────────┘
                              │
                              │ project declares requirements
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│                     project/.patina/config.toml                  │
│                     (PROJECT - declares needs)                   │
│                                                                  │
│  [secrets]                                                       │
│  requires = ["github-token", "openai-key", "database-url"]      │
│                                                                  │
│  (Committed to git - just names, no values)                     │
│                                                                  │
└─────────────────────────────────────────────────────────────────┘
                              │
                              │ patina secrets run
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│                     RUNTIME                                      │
│                                                                  │
│  $ patina secrets run -- cargo test                                      │
│                                                                  │
│  1. Load project requirements                                    │
│  2. Resolve against mothership registry                          │
│  3. Validate against 1Password                                   │
│  4. Inject via op run                                            │
│                                                                  │
│  Environment:                                                    │
│    GITHUB_TOKEN=ghp_xxxxx                                        │
│    OPENAI_KEY=sk-proj-xxxxx                                      │
│    DATABASE_URL=postgres://...                                   │
│                                                                  │
│  Values in memory only. Never on disk.                          │
│                                                                  │
└─────────────────────────────────────────────────────────────────┘
```

---

## Parallel to Models Pattern

| Aspect | Models | Secrets |
|--------|--------|---------|
| Registry | `registry.toml` (in binary) | `~/.patina/secrets.toml` |
| Storage | `~/.patina/cache/models/` | 1Password "Patina" vault |
| Lock/Provenance | `~/.patina/models.lock` | 1Password (external) |
| Project config | `[embeddings] model = "e5-base-v2"` | `[secrets] requires = [...]` |
| Resolution | `models::resolve_model_path()` | `secrets::resolve_for_project()` |

---

## Files

### Mothership: `~/.patina/secrets.toml`

Central registry of all secrets across all projects:

```toml
# 1Password vault (always "Patina")
vault = "Patina"

# Secret definitions
# name = { item = "1password-item-name", field = "field-name", env = "ENV_VAR" }
#
# - item: required - 1Password item name in Patina vault
# - field: optional - defaults to "credential"
# - env: required - environment variable name (always explicit)
#
# The `env` field is always written to config, even when using default.
# This makes the config self-documenting for LLM tools.

[secrets]
# Common development secrets
github-token = { item = "github-token", env = "GITHUB_TOKEN" }
openai-key = { item = "openai-key", env = "OPENAI_API_KEY" }
anthropic-key = { item = "anthropic-key", env = "ANTHROPIC_API_KEY" }

# Infrastructure
tailscale-authkey = { item = "tailscale-authkey", field = "password", env = "TAILSCALE_AUTHKEY" }
docker-hub = { item = "docker-hub", field = "password", env = "DOCKER_HUB_TOKEN" }

# Project-specific (namespaced)
myapp-database = { item = "myapp-postgres", field = "connection_string", env = "DATABASE_URL" }
myapp-stripe = { item = "myapp-stripe", field = "secret_key", env = "STRIPE_SECRET_KEY" }
```

### Naming Rules

**Secret names** (the key in `[secrets]`):
- Lowercase letters, digits, hyphens only
- Pattern: `^[a-z][a-z0-9]*(-[a-z0-9]+)*$`
- Examples: `github-token`, `openai-key`, `myapp-db`

**Environment variable names** (`env` field):
- Uppercase letters, digits, underscores only
- Pattern: `^[A-Z][A-Z0-9_]*$`
- Examples: `GITHUB_TOKEN`, `DATABASE_URL`, `STRIPE_SECRET_KEY`

### Project: `.patina/config.toml`

Project declares what secrets it needs:

```toml
[project]
name = "myapp"

[embeddings]
model = "e5-base-v2"

[secrets]
requires = [
    "github-token",
    "openai-key",
    "myapp-database",
    "myapp-stripe"
]
```

Committed to git. Just names, no values, no 1Password details.

---

## The "Patina" Vault

Patina uses a dedicated 1Password vault:

```bash
$ patina secrets init

1Password:
  ✓ CLI installed (2.30.0)
  ✓ Signed in as nick@example.com

? Create 'Patina' vault? [Y/n] y

Creating vault...
✓ Vault 'Patina' created

✓ Created ~/.patina/secrets.toml
```

If vault exists:
```bash
$ patina secrets init

1Password:
  ✓ CLI installed
  ✓ Signed in
  ✓ Patina vault exists (12 items)

✓ ~/.patina/secrets.toml ready
```

---

## Commands

**Design:** 4 commands, LLM-first, non-interactive by default.

| Command | Purpose |
|---------|---------|
| `patina secrets` | Smart status (mothership + project context) |
| `patina secrets add NAME` | Register secret to mothership |
| `patina secrets run -- CMD` | Execute with secrets (local or remote) |
| `patina secrets init` | One-time vault setup |

---

### `patina secrets` - Status

Shows mothership registry + project requirements in one view. LLM reads this to understand state.

```bash
$ patina secrets

Patina Vault: ✓ connected
Project: myapp

Registered (mothership):
  ✓ github-token      → GITHUB_TOKEN
  ✓ openai-key        → OPENAI_API_KEY
  ✓ database          → DATABASE_URL
  ✗ old-api-key       → OLD_API_KEY         NOT FOUND

Required (project):
  ✓ github-token      (registered, resolves)
  ✓ database          (registered, resolves)
  ✗ stripe-key        (not registered)

Action needed:
  patina secrets add stripe-key --env STRIPE_SECRET_KEY
```

If not in a project directory:
```bash
$ patina secrets

Patina Vault: ✓ connected

Registered (mothership):
  ✓ github-token      → GITHUB_TOKEN
  ✓ openai-key        → OPENAI_API_KEY
  ✓ database          → DATABASE_URL
```

---

### `patina secrets add` - Register Secret

**Non-interactive (LLM use):** Assumes 1Password item already exists.

```bash
# Item exists in 1Password with same name
$ patina secrets add github-token --env GITHUB_TOKEN

✓ Found item 'github-token' in Patina vault
✓ Added to ~/.patina/secrets.toml:
  github-token = { item = "github-token", env = "GITHUB_TOKEN" }
```

```bash
# Item exists with different name
$ patina secrets add stripe --item stripe-live --field secret_key --env STRIPE_SECRET_KEY

✓ Found item 'stripe-live' in Patina vault
✓ Added to ~/.patina/secrets.toml:
  stripe = { item = "stripe-live", field = "secret_key", env = "STRIPE_SECRET_KEY" }
```

```bash
# Item doesn't exist - clear error, user creates in 1Password
$ patina secrets add newkey --env NEW_KEY

✗ Item 'newkey' not found in Patina vault.
  Create it in 1Password first, then retry.

$ echo $?
1
```

**Interactive (human use):** Add `-i` flag for guided flow.

```bash
$ patina secrets add stripe -i

? Search 1Password: stripe
Found:
  1. stripe-test (Development)
  2. stripe-live (Production)

? Select: 2
? Field [credential]: secret_key
? Environment variable [STRIPE]: STRIPE_SECRET_KEY

✓ Added to ~/.patina/secrets.toml:
  stripe = { item = "stripe-live", field = "secret_key", env = "STRIPE_SECRET_KEY" }
```

---

### `patina secrets run` - Execute with Secrets

Resolves secrets, injects via `op run`, executes command.

**Local execution:**
```bash
$ patina secrets run -- cargo test

Resolving secrets for myapp...
  ✓ github-token      → GITHUB_TOKEN
  ✓ database          → DATABASE_URL

Running: cargo test
   Compiling myapp v0.1.0
   Running tests...
test result: ok. 12 passed
```

**Remote execution (--ssh):**
```bash
$ patina secrets run --ssh root@server -- 'cd /app && docker-compose restart'

Resolving secrets for myapp...
  ✓ tailscale-authkey → TAILSCALE_AUTHKEY

Injecting via SSH...
Running on root@server: cd /app && docker-compose restart
Recreating tsdproxy ... done
```

How `--ssh` works:
1. Resolves secrets locally via `op read`
2. Constructs SSH command with env prefix
3. Secrets travel encrypted (SSH), never on disk, never in chat

**Failure cases:**
```bash
# Missing secret
$ patina secrets run -- cargo test

✗ Secret 'stripe-key' required but not registered.
  Run: patina secrets add stripe-key --env STRIPE_SECRET_KEY

$ echo $?
1
```

```bash
# Secret registered but 1Password item missing
$ patina secrets run -- cargo test

✗ Secret 'old-api-key' registered but item not found in 1Password.
  Update ~/.patina/secrets.toml or create item in 1Password.

$ echo $?
1
```

---

### `patina secrets init` - One-Time Setup

Creates Patina vault if needed. Auto-triggered on first `add` if vault missing.

```bash
$ patina secrets init

1Password:
  ✓ CLI installed (2.30.0)
  ✓ Signed in as nick@example.com

Creating vault...
  ✓ Vault 'Patina' created

✓ Created ~/.patina/secrets.toml
```

If already set up:
```bash
$ patina secrets init

1Password:
  ✓ CLI installed
  ✓ Signed in
  ✓ Patina vault exists (12 items)

✓ ~/.patina/secrets.toml ready
```

---

## Integration Points

### `patina init`

For v1, `patina init` does not prompt for secrets. Users configure secrets manually:

```bash
$ patina init .

✓ Created .patina/config.toml

# User manually edits .patina/config.toml:
# [secrets]
# requires = ["github-token", "database"]
```

### `patina serve --mcp`

MCP gate - validates project secrets before serving:

```bash
$ patina serve --mcp

Checking secrets...
  ✓ 1Password signed in
  ✓ Project requires 2 secrets
  ✓ All secrets resolve

Starting MCP server on stdio...
```

```bash
$ patina serve --mcp

Checking secrets...
  ✗ stripe-key not found in mothership

❌ Cannot start MCP server.
   Run: patina secrets add stripe-key --env STRIPE_SECRET_KEY
```

### Layer Pattern

Create `layer/core/secrets-boundary.md` so LLMs learn the workflow:

```markdown
# Secrets Boundary

This project uses 1Password for all secrets.

## For LLMs

- NEVER ask users to paste secrets in chat
- NEVER put secrets in code or config files
- Use `patina secrets` to check what's registered/missing
- Use `patina secrets add <name> --env VAR` to register new secrets
- Use `patina secrets run -- <command>` to execute with secrets
- Use `patina secrets run --ssh HOST -- <command>` for remote execution

## Architecture

- Mothership: ~/.patina/secrets.toml (registry)
- 1Password: "Patina" vault (storage)
- Project: .patina/config.toml [secrets.requires] (dependencies)

## Commands

- `patina secrets` - show status (mothership + project)
- `patina secrets add <name> --env VAR` - register secret
- `patina secrets run -- <cmd>` - execute with secrets (local)
- `patina secrets run --ssh HOST -- <cmd>` - execute with secrets (remote)
- `patina secrets init` - one-time vault setup
```

---

## LLM Workflow

**Before (no secrets boundary):**
```
User: "Deploy with the new Tailscale key"
Claude: "Paste the key here and I'll update .env"
```

**After (with secrets boundary):**
```
User: "Deploy with the new Tailscale key"

Claude: [calls patina secrets]
        [sees tailscale-authkey not registered]

Claude: "I see you need to register the Tailscale key:

        1. Create item 'tailscale-authkey' in 1Password (I won't see it)

        2. Register it:
           $ patina secrets add tailscale-authkey --env TAILSCALE_AUTHKEY

        3. Deploy:
           $ patina secrets run --ssh root@server -- './deploy.sh'

        I never see the key value."
```

---

## Module Design

```
src/
├── secrets/               # Core secrets logic
│   ├── mod.rs             # Public interface
│   └── internal.rs        # 1Password integration, config parsing
└── commands/
    └── secrets/           # CLI command (patina secrets)
        ├── mod.rs         # Subcommand routing
        └── internal.rs    # Command implementations
```

### External Interface

```rust
/// Check 1Password CLI and vault status
pub fn check_op_status() -> Result<OpStatus>;

/// Initialize Patina vault if needed
pub fn init_vault() -> Result<()>;

/// Load mothership secrets registry
pub fn load_registry() -> Result<SecretsRegistry>;

/// Save mothership secrets registry
pub fn save_registry(registry: &SecretsRegistry) -> Result<()>;

/// Load project secrets requirements
pub fn load_project_requirements(project_root: &Path) -> Result<Vec<String>>;

/// Validate secrets against 1Password
pub fn validate_secrets(names: &[String], registry: &SecretsRegistry) -> Result<ValidationReport>;

/// Generate op:// references for given secret names
pub fn generate_op_refs(names: &[String], registry: &SecretsRegistry) -> Result<Vec<OpRef>>;

/// Interactive: add a new secret to registry
pub fn add_secret_interactive(name: &str) -> Result<()>;

/// Add requirement to project config
pub fn require_secret(project_root: &Path, name: &str) -> Result<()>;

/// Execute command with secrets injected (patina secrets run)
pub fn run_with_secrets(project_root: &Path, command: &[String]) -> Result<i32>;
```

### Types

```rust
pub struct OpStatus {
    pub installed: bool,
    pub version: Option<String>,
    pub signed_in: bool,
    pub account: Option<String>,
    pub vault_exists: bool,
    pub vault_item_count: Option<u32>,
}

pub struct SecretsRegistry {
    pub vault: String,  // Always "Patina"
    pub secrets: HashMap<String, SecretDef>,
}

pub struct SecretDef {
    pub item: String,
    pub field: Option<String>,  // Defaults to "credential"
    pub env: String,            // Always explicit, e.g., "GITHUB_TOKEN"
}

pub struct OpRef {
    pub name: String,
    pub env_var: String,        // GITHUB_TOKEN
    pub op_reference: String,   // op://Patina/github-token/credential
}

pub struct ValidationReport {
    pub valid: Vec<String>,
    pub invalid: Vec<(String, String)>,  // (name, error)
}
```

---

## 1Password Integration

### Vault Operations

```bash
# Create vault
op vault create Patina

# Check vault exists
op vault get Patina --format=json

# List items in vault
op item list --vault=Patina --format=json
```

### Item Operations

```bash
# Search items
op item list --vault=Patina --format=json | jq '.[] | select(.title | contains("stripe"))'

# Validate reference resolves (without outputting value)
op read "op://Patina/github-token/credential" > /dev/null 2>&1
echo $?  # 0 = exists, 1 = not found

# Open 1Password app to item
open "onepassword://open/Patina/github-token"
```

### Running with Secrets

```bash
# Generate env file from op:// references
cat > /tmp/secrets.env << EOF
GITHUB_TOKEN=op://Patina/github-token/credential
OPENAI_KEY=op://Patina/openai-key/credential
EOF

# Run with injection
op run --env-file=/tmp/secrets.env -- cargo test

# Cleanup
rm /tmp/secrets.env
```

---

## User Workflows

### First Time Setup

```bash
# Install 1Password CLI
brew install 1password-cli
op signin

# Initialize Patina vault
patina secrets init
# Creates vault + ~/.patina/secrets.toml
```

### Adding a Secret

```bash
# 1. Create item in 1Password app (user does this manually)
#    Item name: "stripe-key"

# 2. Register with Patina (LLM runs this)
patina secrets add stripe-key --env STRIPE_SECRET_KEY

# 3. Add to project requirements (edit config manually)
# .patina/config.toml:
# [secrets]
# requires = ["stripe-key"]
```

### New Project

```bash
cd ~/Projects/newapp
patina init .

# Edit .patina/config.toml to add required secrets:
# [secrets]
# requires = ["github-token", "database"]

# Run with secrets injected
patina secrets run -- cargo run
```

### Remote Deployment

```bash
# Deploy to server with secrets (never paste in chat)
patina secrets run --ssh root@server -- 'cd /app && docker-compose restart'

# Secrets resolved locally, injected over SSH, never on disk
```

### CI/CD

```yaml
# GitHub Actions
- name: Run tests with secrets
  run: patina secrets run -- cargo test
  env:
    OP_SERVICE_ACCOUNT_TOKEN: ${{ secrets.OP_SERVICE_ACCOUNT_TOKEN }}
```

### Key Rotation

```bash
# 1. Update key in 1Password app (user does this)
# 2. Redeploy - no code changes needed
patina secrets run -- ./deploy.sh
# Or remote:
patina secrets run --ssh root@server -- './deploy.sh'
```

No code changes. No config edits. Update 1Password, redeploy.

---

## What We Build

**Core (v1):**
- [x] `~/.patina/secrets.toml` registry format
- [x] `patina secrets` - smart status (mothership + project)
- [x] `patina secrets add <name>` - register secret (non-interactive default)
- [x] `patina secrets run -- <cmd>` - execute with secrets (local)
- [x] `patina secrets run --ssh HOST -- <cmd>` - execute with secrets (remote)
- [x] `patina secrets init` - create Patina vault
- [x] MCP gate - require secrets compliance before serving
- [ ] `layer/core/secrets-boundary.md` - LLM teaching pattern (deferred)

**Deferred (v2):**
- [ ] `patina secrets add -i` - interactive mode for humans
- [ ] `patina secrets migrate` - migrate .env to 1Password
- [ ] `patina init` integration - prompt for secrets during init

## What We Don't Build

- ~~Pattern detection~~ - not our job
- ~~Redaction~~ - gate prevents leak entirely
- ~~Pre-commit hooks~~ - use gitleaks if wanted
- ~~Multi-backend support~~ - 1Password only, opinionated
- ~~Project-level secrets.toml~~ - mothership only
- ~~Separate `patina run` command~~ - consolidated under `patina secrets run`
- ~~`patina secrets check`~~ - implicit in `run` (fails if missing)
- ~~`patina secrets require`~~ - edit `.patina/config.toml` directly
- ~~`patina secrets status`~~ - merged into bare `patina secrets`

---

## Acceptance Criteria

**Core (v1):**
1. [x] `patina secrets init` creates Patina vault if needed
2. [x] `patina secrets` shows mothership + project status in one view
3. [x] `patina secrets add NAME --env VAR` registers without prompts (non-interactive)
4. [x] `patina secrets add` fails with clear error if 1Password item not found
5. [x] `patina secrets run -- <cmd>` injects required secrets locally
6. [x] `patina secrets run --ssh HOST -- <cmd>` injects secrets over SSH
7. [x] `patina secrets run` exits 1 with actionable error if secrets missing
8. [x] MCP refuses to start if project secrets don't resolve
9. [ ] LLMs learn pattern via `patina context` (deferred)

**Deferred (v2):**
10. [ ] `patina secrets add -i` interactive mode for humans
11. [ ] `patina secrets migrate` converts .env to 1Password
12. [ ] `patina init` offers to configure secrets

---

## Security Model

### Trust Model

```
1Password         ←── User trusts for secret storage
    │
    │ Patina vault
    ▼
~/.patina/        ←── Mothership registry (names only)
    │
    │ project declares requirements
    ▼
.patina/config    ←── Project config (names only, committed)
    │
    │ patina secrets run resolves
    ▼
LLM               ←── Sees names only, triggers workflows
```

### What This Protects Against

- **Secrets in chat** - LLM triggers commands, never sees values
- **Secrets in git** - Only names committed, no values
- **Secrets in .env** - Migrated to 1Password, .env deleted
- **LLM context leakage** - MCP gate prevents serving non-compliant projects
- **Cross-project leakage** - Each project declares its own requirements

### What This Does NOT Protect Against

- **Compromised 1Password** - Vault access = all secrets
- **Local machine compromise** - Attacker can run `op read`
- **Intentional sharing** - User can still copy/paste

---

## Future Exploration: Headless & Container Support

Current implementation requires GUI/biometric unlock (Touch ID on Mac). This is problematic for:

1. **Touch ID popup every time** - Each new terminal/process triggers biometric
2. **Running in containers locally** - No GUI available
3. **Fully headless local dev** - No 1Password app

### 1Password Options Explored

| Option | How It Works | Limitations |
|--------|--------------|-------------|
| Session caching | 10-min session, auto-refreshes | New terminal = new prompt |
| `OP_BIOMETRIC_UNLOCK_ENABLED=false` | CLI password auth | Still interactive |
| 1Password Connect Server | Self-hosted REST API | Requires running server |
| Service Accounts | Token-based, no GUI | **Read-only**, admin pre-setup |

Service accounts break the workflow because they can't create vaults or items - an admin must pre-configure everything.

### Bitwarden as Alternative

Bitwarden's architecture is more automation-friendly:

| Feature | Bitwarden | 1Password |
|---------|-----------|-----------|
| Self-hosted | ✓ Full support | ✗ No |
| Open source CLI | ✓ Yes | ✗ No |
| Session tokens | ✓ `BW_SESSION` env var, long-lived | 10-min, per-terminal |
| Secrets Manager | ✓ Dedicated product for machines | Service Accounts only |
| Container-friendly | ✓ Designed for it | Needs Connect server |

Bitwarden session model:
```bash
# Unlock once, get session token
export BW_SESSION=$(bw unlock --raw)

# All subsequent calls use token - no prompts
bw get password github-token
```

This session token can be long-lived and works in containers/headless environments.

### Potential v2 Directions

1. **Multi-backend support** - Abstract secrets provider, support both 1Password and Bitwarden
2. **1Password Connect** - Add support for Connect server for container/CI use
3. **Bitwarden-first** - Consider Bitwarden as primary backend for better automation story
4. **Hybrid approach** - 1Password for local dev (Touch ID UX), Bitwarden for headless

### Research Sources

- [1Password CLI Docs](https://developer.1password.com/docs/cli/)
- [1Password Biometric Security](https://developer.1password.com/docs/cli/app-integration-security/)
- [1Password vs Bitwarden Comparison](https://alexn.org/blog/2024/08/20/1password-vs-bitwarden/)
- [Bitwarden Secrets Manager](https://bitwarden.com/products/secrets-manager/)

---

## References

- 1Password CLI: https://developer.1password.com/docs/cli
- op:// URIs: https://developer.1password.com/docs/cli/secret-references
- Service Accounts (CI): https://developer.1password.com/docs/service-accounts
- Patina Models Pattern: `src/models/mod.rs`
