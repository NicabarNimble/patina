# Spec: Secrets Boundary

**Status:** Ready for Implementation

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
# name = { item = "1password-item-name", field = "field-name" }
# field defaults to "credential" if omitted

[secrets]
# Common development secrets
github-token = { item = "github-token" }
openai-key = { item = "openai-key" }
anthropic-key = { item = "anthropic-key" }

# Infrastructure
tailscale-authkey = { item = "tailscale-authkey", field = "password" }
docker-hub = { item = "docker-hub", field = "password" }

# Project-specific (namespaced)
myapp-database = { item = "myapp-postgres", field = "connection_string" }
myapp-stripe = { item = "myapp-stripe", field = "secret_key" }
```

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

### Mothership Level

```bash
# Show all registered secrets + validation status
$ patina secrets

Patina Vault: ✓ connected (12 items)

Registered secrets (8):
  ✓ github-token      → op://Patina/github-token/credential
  ✓ openai-key        → op://Patina/openai-key/credential
  ✓ anthropic-key     → op://Patina/anthropic-key/credential
  ✓ tailscale-authkey → op://Patina/tailscale-authkey/password
  ✓ docker-hub        → op://Patina/docker-hub/password
  ✓ myapp-database    → op://Patina/myapp-postgres/connection_string
  ✓ myapp-stripe      → op://Patina/myapp-stripe/secret_key
  ✗ old-api-key       → op://Patina/old-api-key/credential (not found)
```

```bash
# Add secret to registry (interactive)
$ patina secrets add stripe-key

? Search 1Password: stripe

Found in Patina vault:
  1. stripe-test (Development)
  2. stripe-live (Production)
  3. [Create new item]

? Select: 3

Opening 1Password to create item...
# 1Password app opens, user creates item securely
# Claude never sees the value

? Item name in 1Password: stripe-api-key
? Field [credential]: secret_key

✓ Added to ~/.patina/secrets.toml:
  stripe-key = { item = "stripe-api-key", field = "secret_key" }
```

```bash
# Validate all secrets resolve
$ patina secrets check

✓ 1Password CLI installed
✓ Signed in
✓ Patina vault accessible
✓ 7/8 secrets resolve

✗ old-api-key: item not found

$ echo $?
1
```

### Project Level

```bash
# Add requirement to project
$ patina secrets require openai-key

✓ Added 'openai-key' to .patina/config.toml [secrets.requires]
```

```bash
# Check project's requirements against mothership
$ patina secrets status

Project: myapp
Requires: 4 secrets

  ✓ github-token      (in mothership, resolves)
  ✓ openai-key        (in mothership, resolves)
  ✓ myapp-database    (in mothership, resolves)
  ✗ myapp-stripe      (not in mothership)

Run 'patina secrets add myapp-stripe' to register.
```

```bash
# Run with secrets injected
$ patina secrets run -- cargo test

Resolving secrets for myapp...
  ✓ github-token
  ✓ openai-key
  ✓ myapp-database
  ✓ myapp-stripe

Injecting 4 secrets...
Running: cargo test

   Compiling myapp v0.1.0
   Running tests...

test result: ok. 12 passed; 0 failed
```

---

## Integration Points

### `patina init`

```bash
$ patina init .

✓ Created .patina/config.toml

Checking mothership...
  ✓ ~/.patina/secrets.toml exists
  ✓ Patina vault connected

? Does this project need secrets? [Y/n] y
? Which secrets? (space to select)
  [x] github-token
  [x] openai-key
  [ ] anthropic-key
  [ ] tailscale-authkey
  [x] Add new...

? New secret name: myapp-database
  → Opening 1Password to create...

✓ Added to [secrets.requires]: github-token, openai-key, myapp-database
```

### `patina serve --mcp`

MCP gate - validates project secrets before serving:

```bash
$ patina serve --mcp

Checking secrets...
  ✓ 1Password signed in
  ✓ Project requires 3 secrets
  ✓ All secrets resolve

Starting MCP server on stdio...
```

```bash
$ patina serve --mcp

Checking secrets...
  ✗ myapp-stripe not found in mothership

❌ Cannot start MCP server.
   Run 'patina secrets add myapp-stripe' first.
```

### Layer Pattern

Create `layer/core/secrets-boundary.md` so LLMs learn the workflow:

```markdown
# Secrets Boundary

This project uses 1Password for all secrets.

## For LLMs

- NEVER ask users to paste secrets in chat
- NEVER put secrets in code or config files
- Use `patina secrets add <name>` to register new secrets
- Use `patina secrets require <name>` to add to project
- Use `patina secrets run -- <command>` to execute with secrets

## Architecture

- Mothership: ~/.patina/secrets.toml (registry)
- 1Password: "Patina" vault (storage)
- Project: .patina/config.toml [secrets.requires] (dependencies)

## Commands

- `patina secrets` - list all registered secrets
- `patina secrets init` - create Patina vault
- `patina secrets add <name>` - add to mothership registry
- `patina secrets check` - validate all secrets resolve (CI)
- `patina secrets require <name>` - add to project requirements
- `patina secrets status` - check project's secrets
- `patina secrets migrate` - migrate .env to 1Password
- `patina secrets run -- <cmd>` - run with secrets injected
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

Claude: [calls patina context "secrets"]
        [learns about secrets-boundary pattern]

Claude: "I'll help you set this up:

        1. Add the key to 1Password (I won't see it):
           $ patina secrets add tailscale-authkey

        2. Add to this project's requirements:
           $ patina secrets require tailscale-authkey

        3. Deploy:
           $ patina secrets run -- ./deploy.sh

        I don't need to see the key."
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
# Add to mothership registry
patina secrets add stripe-key
# Interactive: creates in 1Password, adds to registry

# Add to project requirements
cd ~/Projects/myapp
patina secrets require stripe-key
# Adds to .patina/config.toml
```

### New Project

```bash
cd ~/Projects/newapp
patina init .
# Prompts for secrets, adds to [secrets.requires]

patina secrets run -- cargo run
# Injects all required secrets
```

### Existing Project (Migration)

```bash
cd ~/Projects/legacy-app
patina init .

# Has .env file?
patina secrets migrate
# Reads .env, creates items in 1Password, updates registry
# Updates project requirements
# Backs up .env → .env.backup
```

### CI/CD

```yaml
# GitHub Actions
- name: Check secrets
  run: patina secrets check
  env:
    OP_SERVICE_ACCOUNT_TOKEN: ${{ secrets.OP_SERVICE_ACCOUNT_TOKEN }}

- name: Run tests
  run: patina secrets run -- cargo test
```

### Key Rotation

```bash
# 1. Update key in 1Password app (user does this)
# 2. Verify
patina secrets check
# 3. Redeploy all projects using it
patina secrets run -- ./deploy.sh
```

No code changes. No config edits. Update 1Password, redeploy.

---

## What We Build

- [ ] `~/.patina/secrets.toml` registry format
- [ ] `patina secrets init` - create Patina vault
- [ ] `patina secrets` - list registered secrets
- [ ] `patina secrets add <name>` - interactive add to registry
- [ ] `patina secrets check` - validate all secrets
- [ ] `patina secrets require <name>` - add to project
- [ ] `patina secrets status` - project secrets status
- [ ] `patina secrets migrate` - migrate .env to 1Password
- [ ] `patina secrets run -- <cmd>` - execute with secrets
- [ ] MCP gate - require secrets compliance
- [ ] `patina init` integration
- [ ] `layer/core/secrets-boundary.md` - LLM teaching pattern

## What We Don't Build

- ~~Pattern detection~~ - not our job
- ~~Redaction~~ - gate prevents leak entirely
- ~~Pre-commit hooks~~ - use gitleaks if wanted
- ~~Multi-backend support~~ - 1Password only, opinionated
- ~~Project-level secrets.toml~~ - mothership only
- ~~Separate `patina run` command~~ - consolidated under `patina secrets run`

---

## Acceptance Criteria

1. [ ] `patina secrets init` creates Patina vault if needed
2. [ ] `patina secrets` lists all registered secrets with status
3. [ ] `patina secrets add` interactively registers without showing values
4. [ ] `patina secrets check` exits 1 on any invalid secret
5. [ ] `patina secrets require` adds to project config
6. [ ] `patina secrets status` shows project requirements vs registry
7. [ ] `patina secrets run -- <cmd>` injects required secrets
8. [ ] MCP refuses to start if project secrets don't resolve
9. [ ] `patina init` offers to configure secrets
10. [ ] LLMs learn pattern via `patina context`

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

## References

- 1Password CLI: https://developer.1password.com/docs/cli
- op:// URIs: https://developer.1password.com/docs/cli/secret-references
- Service Accounts (CI): https://developer.1password.com/docs/service-accounts
- Patina Models Pattern: `src/models/mod.rs`
