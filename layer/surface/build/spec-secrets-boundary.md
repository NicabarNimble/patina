# Spec: Secrets Boundary

**Status:** New Design

**Goal:** LLMs never see secret values. Git never contains secret values. Users can safely manage secrets without worry.

---

## The Problem

When using LLMs for development:
1. **Context leakage** - API keys in config files end up in LLM context
2. **Git leakage** - Secrets accidentally committed to repositories
3. **Learning leakage** - LLMs may learn/memorize secret patterns
4. **User anxiety** - Constant worry about secret exposure

Current state: No protection. Secrets in `.env` files are read by LLMs, committed to git, everywhere.

---

## The Solution: Secrets Boundary

```
┌─────────────────────────────────────────────────────────────────────────┐
│                         SECRETS VAULT                                    │
│                                                                          │
│   1Password / op CLI                                                     │
│   ├── vault: "Development"                                               │
│   │   ├── item: "github-token" → actual value: ghp_xxxxx                │
│   │   ├── item: "openai-key"   → actual value: sk-xxxxx                 │
│   │   └── item: "db-password"  → actual value: hunter2                  │
│   └── Reference format: op://Development/github-token/credential        │
│                                                                          │
└─────────────────────────────────────────────────────────────────────────┘
                              │
                              │ REFERENCE ONLY (op://)
                              ▼
┌─────────────────────────────────────────────────────────────────────────┐
│                         PATINA LAYER                                     │
│                                                                          │
│   project/.patina/config.toml:                                           │
│     [secrets]                                                            │
│     github_token = "op://Development/github-token/credential"           │
│     openai_key = "op://Development/openai-key/credential"               │
│                                                                          │
│   project/.env (gitignored, for local dev):                              │
│     GITHUB_TOKEN=op://Development/github-token/credential               │
│     OPENAI_KEY=op://Development/openai-key/credential                   │
│                                                                          │
│   Git sees: op:// references ONLY                                        │
│   Patina sees: op:// references ONLY                                     │
│                                                                          │
└─────────────────────────────────────────────────────────────────────────┘
                              │
                              │ RUNTIME RESOLUTION
                              ▼
┌─────────────────────────────────────────────────────────────────────────┐
│                      EXECUTION BOUNDARY                                  │
│                                                                          │
│   op run / op read (1Password CLI):                                      │
│     Resolves op:// → actual values at execution time                     │
│     Values live in process memory only                                   │
│     Never written to disk, never in LLM context                          │
│                                                                          │
│   Example:                                                               │
│     op run -- cargo run                                                  │
│     → GITHUB_TOKEN=ghp_xxxxx (in process env only)                       │
│                                                                          │
└─────────────────────────────────────────────────────────────────────────┘
                              │
            ═══════════════════════════════════════════ WALL
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────────────┐
│                      LLM FRONTEND                                        │
│                                                                          │
│   Claude Code / Gemini / etc:                                            │
│     - Sees: GITHUB_TOKEN=op://Development/github-token/credential        │
│     - Never sees: ghp_xxxxx                                              │
│     - Can't leak what it doesn't know                                    │
│                                                                          │
└─────────────────────────────────────────────────────────────────────────┘
```

---

## Design Principles

### 1. References, Not Values

Patina only ever stores and transmits `op://` references:
- Config files: `op://vault/item/field`
- Environment: `SECRET=op://vault/item/field`
- Git commits: Only references, never values

### 2. Resolution at Execution Boundary

Values are resolved by 1Password CLI at the moment of execution:
- `op run -- <command>` injects secrets into process environment
- `op read op://vault/item/field` returns value for scripts
- Resolution happens in terminal, not in LLM-visible context

### 3. Defense in Depth

Multiple layers of protection:

| Layer | Protection | Implementation |
|-------|------------|----------------|
| **Config** | Only op:// references stored | Schema validation |
| **Scrape** | Detect/warn on secret patterns | Regex patterns for common secrets |
| **Context** | Filter secrets before MCP response | Redact patterns in scry output |
| **Git** | Pre-commit hook for secret detection | `patina doctor --secrets` |
| **LLM** | Never sees resolved values | op:// stays as reference |

---

## Implementation

### Phase 1: Config Schema (Foundation)

Support `op://` references in Patina config.

**Tasks:**
- [ ] Define `SecretRef` type in config schema
- [ ] Validate op:// URI format
- [ ] Document secret reference pattern

**Config example:**
```toml
# .patina/config.toml
[secrets]
# These are references, not values
github_token = "op://Development/github-token/credential"
openai_key = "op://Development/openai-key/credential"

[secrets.custom]
# User-defined secrets
db_password = "op://Production/database/password"
```

### Phase 2: Detection & Warning (Safety Net)

Detect potential secrets in code/config and warn.

**Tasks:**
- [ ] Define secret patterns (API keys, tokens, passwords)
- [ ] Add detection to `patina scrape`
- [ ] Add `patina doctor --secrets` check
- [ ] Pre-commit hook template

**Secret patterns:**
```rust
const SECRET_PATTERNS: &[(&str, &str)] = &[
    (r"ghp_[a-zA-Z0-9]{36}", "GitHub Personal Access Token"),
    (r"sk-[a-zA-Z0-9]{48}", "OpenAI API Key"),
    (r"AKIA[A-Z0-9]{16}", "AWS Access Key"),
    (r"(?i)password\s*=\s*['\"][^'\"]+['\"]", "Hardcoded password"),
    (r"(?i)api[_-]?key\s*=\s*['\"][^'\"]+['\"]", "Hardcoded API key"),
];
```

**Output:**
```
$ patina doctor --secrets

⚠️  Potential secrets detected:

  src/config.rs:42
    Pattern: GitHub Personal Access Token
    Line: let token = "ghp_xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx";

  .env:3
    Pattern: OpenAI API Key
    Line: OPENAI_KEY=sk-xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx

Recommendation: Use op:// references instead
  OPENAI_KEY=op://Development/openai-key/credential

Run `patina secrets migrate` for guided migration.
```

### Phase 3: Context Filtering (LLM Protection)

Filter secrets from MCP/scry responses before LLM sees them.

**Tasks:**
- [ ] Add secret pattern filter to MCP server
- [ ] Redact matching patterns in scry results
- [ ] Option to show redacted indicator: `[REDACTED: GitHub token]`

**Flow:**
```
scry query → results include src/config.rs with ghp_xxxx
           ↓
secret filter → detects GitHub token pattern
           ↓
redacted output → src/config.rs with [REDACTED]
           ↓
LLM sees → safe content only
```

### Phase 4: Resolution Integration (Optional)

Helper for resolving secrets at execution time.

**Tasks:**
- [ ] `patina run -- <command>` wrapper for `op run`
- [ ] Check 1Password CLI availability
- [ ] Graceful degradation if op not installed

**Usage:**
```bash
# Instead of:
op run -- cargo run

# Optionally:
patina run -- cargo run
# (wraps op run with better error messages)
```

---

## 1Password CLI Integration

### Prerequisites

```bash
# Install 1Password CLI
brew install 1password-cli

# Sign in (once)
op signin

# Verify
op vault list
```

### Reference Format

```
op://vault/item/field

Examples:
op://Development/github-token/credential
op://Production/database/password
op://Personal/ssh-key/private_key
```

### Usage Patterns

```bash
# Run command with secrets injected
op run -- cargo run

# Read single secret
export TOKEN=$(op read op://Development/github-token/credential)

# In scripts
#!/bin/bash
op run --env-file=.env -- ./deploy.sh
```

---

## Git Safety

### Pre-commit Hook

```bash
#!/bin/bash
# .git/hooks/pre-commit (or via patina init)

# Run patina secret detection
patina doctor --secrets --staged-only

if [ $? -ne 0 ]; then
    echo "❌ Potential secrets detected in staged files"
    echo "   Use op:// references instead of raw values"
    exit 1
fi
```

### .gitignore Template

```gitignore
# Secrets (never commit)
.env
.env.local
*.pem
*.key
credentials.json

# Patina local data (rebuildable)
.patina/data/
.patina/cache/
```

---

## User Workflow

### Initial Setup

```bash
# 1. Install and sign in to 1Password CLI
brew install 1password-cli
op signin

# 2. Create secrets in 1Password
#    (via 1Password app or CLI)

# 3. Reference in config
echo 'GITHUB_TOKEN=op://Development/github-token/credential' >> .env

# 4. Run with secrets
op run -- cargo run
```

### Checking for Leaks

```bash
# Check current project
patina doctor --secrets

# Check before commit
patina doctor --secrets --staged-only
```

### Migrating Existing Secrets

```bash
# Guided migration (future)
patina secrets migrate

# Shows:
# Found: GITHUB_TOKEN=ghp_xxxxx in .env
#
# To migrate:
# 1. Add to 1Password: op item create --title "github-token" password=ghp_xxxxx
# 2. Update .env: GITHUB_TOKEN=op://Development/github-token/credential
# 3. Delete the raw value from .env
```

---

## Acceptance Criteria

1. [ ] Config schema supports `op://` references
2. [ ] `patina doctor --secrets` detects common secret patterns
3. [ ] MCP responses filter/redact detected secrets
4. [ ] Pre-commit hook template prevents secret commits
5. [ ] Documentation: complete user workflow for 1Password integration
6. [ ] Persona capture: "Use op:// for secrets" becomes searchable pattern

---

## Security Considerations

### What This Does NOT Protect Against

- **Compromised 1Password account** - vault access = all secrets
- **Local machine compromise** - attacker with shell access can run `op read`
- **Secrets in memory** - values exist in process memory during execution
- **Intentional sharing** - user can still copy/paste secrets

### What This DOES Protect Against

- **Accidental git commits** - only references committed
- **LLM context leakage** - LLM never sees resolved values
- **Log file leakage** - references don't expose values in logs
- **Code review exposure** - reviewers see references, not secrets

---

## Related Work

- 1Password CLI documentation: https://developer.1password.com/docs/cli
- `op://` URI specification: https://developer.1password.com/docs/cli/secret-references
- Git secret scanning: GitHub secret scanning, gitleaks, trufflehog
