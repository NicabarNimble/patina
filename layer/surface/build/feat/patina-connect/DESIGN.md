# Design: patina-connect — Connection Subsystem

## Why This Design

The previous design treated patina-connect as a CLI convenience —
"one command replaces four manual steps." That framing was too narrow.

The broker already consumes connections (read side exists in
`src/broker/connection.rs`). What's missing is the product-grade
connection model that separates storage, domain, and runtime concerns.

Today `broker/mod.rs:36-57` collapses three layers into one function:
loads TOML, decrypts vault, builds a credential tuple. The connection
"model" (`ConnectionConfig`) is a 4-field TOML reader with no lifecycle
state, no auth strategy metadata, and no mutation API.

This design builds patina-connect as a subsystem with three layers:

```
Acquisition (per-provider)
    │ acquire credential, probe account identity
    ▼
Persistence (connection domain model)
    │ ConnectionRecord: identity metadata + durable auth config
    │ vault: encrypted credential bytes
    ▼
Consumption (auth plan resolution)
    │ resolve_auth() → AuthPlan
    │ broker dispatches on auth strategy
    ▼
Runtime (broker/http.rs)
    │ inject credential per AuthPlan strategy
    ▼
Child (sandboxed, proxied HTTP)
```

**Origin:** [[session-20260309-131917]] (subsystem design session),
[[session-20260306-123021]] (connection model concept),
[[session-20260306-174214]] (credential delivery audit).

### Architectural Drift

The runtime path for external services has shifted to native child +
broker + connection config, but parts of the code still reflect the
older plugin-era model. Since this is pre-v1 with no compatibility
burden, we clean the seams now rather than preserving transitional
shapes.

**What is structurally right (keep):**
- Vault as credential substrate (`src/secrets/mod.rs`)
- Native child runtime + brokered HTTP proxy (`src/broker/spawn.rs`,
  `src/broker/http.rs`)
- Source-to-connection binding (`src/broker/sources.rs`)

**What lives in the wrong place (move):**
- Connection path ownership: hardcoded in `broker/connection.rs:42`
  → move to `paths::connections`
- Connection loading/parsing: `broker/connection.rs`
  → move to `src/connect/` domain module
- Auth resolution: half-embedded in `broker/mod.rs:36`
  → move to `connect::resolve_auth()` seam

**What is actually broken (fix):**
- `auth.required` declared in `child.toml:16` but ignored by
  `broker/mod.rs:44` — proceeds without auth on missing credential.
  This is fail-open on a fail-closed contract. `resolve_auth()`
  makes this an error, not a warning.
- Hardcoded Bearer injection at `broker/http.rs:79-86` — GitHub's
  model baked into the runtime. `AuthPlan` dispatch replaces this.
- Stale CLI text: `commands/mother/mod.rs:103` describes `Run` as
  writing to events.db only. Code already handles lake routing.

**What should not be copied forward (delete or sever):**
- Broker dependency on plugin auth types: `broker/http.rs:12-13`
  imports `CredentialMapping` and `InjectionLocation` from
  `src/plugin/internal/mod.rs`. These types serve the WASM plugin
  security boundary (explicit per-plugin per-secret grants). Native
  children use a different security model (sandboxed process, proxied
  HTTP, domain allowlist). The broker should import from `connect`,
  not `plugin`.
- Plugin secret-grant instructions: `host_support.rs:276,296,309`
  tells users to run `patina plugin grant`, a command that does not
  exist. This is stale error messaging from the plugin era.
- Connector-plugin vocabulary as auth model: `plugin/internal/mod.rs:92`
  treats connectors as plugins. The auth abstraction for external
  services is now "connection", not "connector plugin".

**Patterns to avoid in the new code:**
- Do not let broker parse connection TOML directly.
- Do not let broker decrypt secrets directly for connection use.
- Do not let runtime code branch on provider identity.
- Do not model auth strategy as "Bearer because GitHub."
- Do not inherit plugin secret-grant semantics.
- Do not treat the 4-field ConnectionConfig as a sufficient model.

## §1 — Connection Domain Model

The connection record carries two kinds of durable metadata.

### Connection Identity Metadata (human-facing)

Who is connected, to what, how, and when.

```rust
pub struct ConnectionRecord {
    // Identity
    pub name: String,              // "github" — unique key, filename stem
    pub provider: String,          // "github" — which provider definition
    pub account_id: Option<String>,// "octocat" — who authenticated
    pub auth_method: AuthMethod,   // OAuth | Manual
    pub scopes: Vec<String>,       // ["repo", "read:org"]
    pub is_default: bool,          // default connection for this provider

    // Lifecycle
    pub created_at: String,        // ISO 8601
    pub updated_at: String,        // ISO 8601
    pub last_validated: Option<String>, // last successful health check

    // Durable auth config (see §2)
    pub auth: AuthConfig,

    // Schema version for forward compat
    pub schema_version: u64,       // 0

    // Scope (v1: always Global)
    pub scope: ConnectionScope,
}

pub enum AuthMethod {
    OAuth,   // acquired via device flow or similar
    Manual,  // user pasted a token
}

pub enum ConnectionScope {
    Global,  // ~/.patina/connections/ (v1 default)
    // Project(PathBuf), // future: project-local connections
    // Persona(String),  // future: persona-scoped
}
```

### Durable Auth Configuration (machine-facing)

How the broker should use this connection's credential at runtime.
This is *configuration* — no decrypted secrets, no ephemeral state.

```rust
pub struct AuthConfig {
    pub injection: InjectionStrategy,
    pub secret_ref: String,           // vault secret name: "github:default"
    pub child: String,                // "github-connector"
    pub allowed_domains: Vec<String>, // ["api.github.com"]

    // Refresh/expiry (durable state, not live checks)
    pub refresh_capable: bool,
    pub expires_at: Option<String>,   // ISO 8601, if known at acquisition
    pub last_error: Option<String>,   // last auth failure message
}

pub enum InjectionStrategy {
    /// Authorization: Bearer {token}
    Bearer,
    /// Custom header: {name}: {value}
    Header { name: String },
    /// Raw token delivered via pipe/initialize to child
    InProcess,
}
```

**Why these fields:**

- `injection` — drives `broker/http.rs` dispatch. Today hardcoded
  to Bearer (`broker/http.rs:80-86`). This makes it data-driven.
- `secret_ref` — vault secret name. Today `conn_config.credential`
  (`broker/mod.rs:41`). Same concept, better name.
- `child` — which child binary to spawn. Today `conn_config.child`
  (`broker/mod.rs:60`). Unchanged.
- `allowed_domains` — today comes from `child.toml` manifest
  (`spawn.rs:159-163`). The connection record carries the expected
  domains so the domain layer can validate consistency with the
  child manifest at load time.
- `refresh_capable` — whether `patina connect refresh` can re-acquire.
  OAuth connections are refresh-capable. Manual tokens are not.
- `expires_at` — if the provider reports expiry at acquisition time.
  GitHub device flow tokens don't expire, but future providers may.
- `last_error` — persisted auth failure for status display. Updated
  when `resolve_auth()` or runtime auth fails.

### TOML Format

```toml
# ~/.patina/connections/github.toml
schema_version = 0

[identity]
name = "github"
provider = "github"
account_id = "octocat"
auth_method = "oauth"
scopes = ["repo", "read:org"]
is_default = true
created_at = "2026-03-09T17:00:00Z"
updated_at = "2026-03-09T17:00:00Z"
scope = "global"

[auth]
injection = "bearer"
secret_ref = "github:default"
child = "github-connector"
allowed_domains = ["api.github.com"]
refresh_capable = true
```

The `[identity]` and `[auth]` sections map to the two metadata
kinds. The file is the single durable record. The vault holds the
encrypted credential value separately.

## §2 — Auth Plan Resolution

`resolve_auth()` is the seam between durable metadata and runtime.

```rust
/// Execution-ready auth for the broker. No TOML, no vault access,
/// no provider knowledge — just what the broker needs to run.
pub struct AuthPlan {
    pub child: String,
    pub credential: Option<ResolvedCredential>,
    pub allowed_domains: Vec<String>,
}

pub struct ResolvedCredential {
    pub value: String,             // decrypted secret
    pub injection: InjectionStrategy,
}
```

The resolution function:

```rust
/// Resolve a ConnectionRecord into an execution-ready AuthPlan.
///
/// This is the ONLY place that decrypts vault material for
/// connection use. Returns typed errors for each failure mode.
///
/// FAIL CLOSED: if the credential cannot be resolved, this returns
/// an error. The broker must not proceed without auth. This replaces
/// the current behavior (broker/mod.rs:44-56) which logs a warning
/// and spawns unauthenticated — violating the child.toml contract.
pub fn resolve_auth(record: &ConnectionRecord) -> Result<AuthPlan> {
    // 1. Resolve child binary existence
    let child_path = crate::broker::spawn::resolve_child_binary(&record.auth.child);
    if child_path.is_err() {
        return Err(ConnectError::ChildNotFound {
            connection: record.name.clone(),
            child: record.auth.child.clone(),
        });
    }

    // 2. Decrypt credential from vault
    let credential = match secrets::get_global_secret(&record.auth.secret_ref) {
        Ok(Some(value)) => ResolvedCredential {
            value,
            injection: record.auth.injection.clone(),
        },
        Ok(None) => {
            // FAIL CLOSED — credential missing is an error, not a warning.
            // Today broker/mod.rs:44 says "proceeding without auth" here.
            // That violates the child.toml auth.required contract.
            return Err(ConnectError::CredentialMissing {
                connection: record.name.clone(),
                secret_ref: record.auth.secret_ref.clone(),
            });
        }
        Err(e) => {
            // Vault decryption failure — identity unavailable, corrupted vault
            return Err(ConnectError::VaultError {
                connection: record.name.clone(),
                detail: e.to_string(),
            });
        }
    };

    Ok(AuthPlan {
        child: record.auth.child.clone(),
        credential: Some(credential),
        allowed_domains: record.auth.allowed_domains.clone(),
    })
}
```

**Error types for resolution failures:**

```rust
pub enum ConnectError {
    /// Connection TOML not found at expected path
    NotFound { name: String },
    /// Connection TOML exists but fails to parse
    MalformedConfig { name: String, detail: String },
    /// Vault secret referenced by connection doesn't exist
    CredentialMissing { connection: String, secret_ref: String },
    /// Vault decryption failed (identity unavailable, corrupted vault)
    VaultError { connection: String, detail: String },
    /// Connection references unknown child binary
    ChildNotFound { connection: String, child: String },
}
```

Each variant produces an actionable error message. The broker
doesn't need to interpret these — it can display them directly.

**Why fail closed matters:** Today `broker/mod.rs:44-48` catches
`Ok(None)` (credential not found) and `Err(e)` (vault failure) and
proceeds with `None` as the credential. This means a child that
declares `auth.required = true` in its `child.toml` (`children/
github-connector/child.toml:16`) can be spawned unauthenticated.
The child doesn't know auth is missing because Mother mediates all
HTTP — it just gets empty responses or 401s from the provider API.
`resolve_auth()` makes this a hard error at the connection layer
before the child is ever spawned.

### Broker Integration

Today (`broker/mod.rs:36-57`):
```rust
let conn_config = load_connection(&source.connection)?;
let credential = get_global_secret(&conn_config.credential)?;
let (mut child, manifest) = spawn_native(
    &conn_config.child, credential, no_sandbox, &conn_config.provider, None,
)?;
```

After:
```rust
let record = connect::load(&source.connection)?;
let auth_plan = connect::resolve_auth(&record)?;
let (mut child, manifest) = spawn_native_with_plan(
    &auth_plan, no_sandbox, &record.provider, None,
)?;
```

`spawn_native_with_plan` replaces `spawn_native`. It takes an
`AuthPlan` instead of a raw `(String, String)` credential tuple.

`build_production_handler` (`broker/http.rs:24`) changes from:
```rust
pub fn build_production_handler(
    allowed_domains: &[String],
    credential: Option<(String, String)>,
    child_name: &str,
) -> Result<HttpHandler>
```

to:
```rust
pub fn build_production_handler(
    auth_plan: &AuthPlan,
    child_name: &str,
) -> Result<HttpHandler>
```

The handler dispatches on `auth_plan.credential.injection`:

```rust
match &cred.injection {
    InjectionStrategy::Bearer => {
        builder = builder.header("Authorization", format!("Bearer {}", cred.value));
    }
    InjectionStrategy::Header { name } => {
        builder = builder.header(name, &cred.value);
    }
    InjectionStrategy::InProcess => {
        // No HTTP injection — credential delivered via pipe/initialize
    }
}
```

This replaces the hardcoded Bearer injection at `broker/http.rs:80-86`.

### Severing Plugin Auth Imports

Today `broker/http.rs:12-13`:
```rust
use crate::plugin::internal::host_support;
use crate::plugin::{CredentialMapping, InjectionLocation};
```

After: these imports are deleted. The broker uses:
```rust
use crate::connect::{AuthPlan, InjectionStrategy, ResolvedCredential};
```

The plugin module's `CredentialMapping`, `InjectionLocation`, and
`host_support::inject_credential()` continue to exist for the WASM
plugin path, but the broker's native-child HTTP handler no longer
depends on them. The two auth paths are separate:

- **WASM plugins**: plugin manifest → secret grants check → host
  boundary injection. Types: `CredentialMapping`, `InjectionLocation`.
  Governed by `secret-grants.toml`. (`src/plugin/internal/`)
- **Native children**: connection record → `resolve_auth()` →
  `AuthPlan`. Types: `AuthPlan`, `InjectionStrategy`. Governed
  by connection model. (`src/connect/`)

The broker only uses the native-child path. The shared functions
that both paths need (HTTP client construction, URL validation,
leak detection) stay in `host_support` but the broker calls them
directly, not through the plugin credential types.

## §3 — Provider Interface

Acquisition is per-provider. The trait defines what a provider
must know how to do:

```rust
/// Provider-specific credential acquisition.
///
/// Each provider implements this trait. The connect module calls
/// it during `patina connect <provider>`. The trait is the boundary
/// between provider-specific code and the uniform connection model.
pub trait Provider {
    /// Provider identifier (e.g., "github").
    fn name(&self) -> &str;

    /// Acquire a credential interactively.
    /// Returns the raw credential string to store in vault.
    fn acquire(&self) -> Result<AcquisitionResult>;

    /// Acquire a credential from a manually-provided value.
    /// For --manual mode.
    fn acquire_manual(&self, secret_ref: &str) -> Result<AcquisitionResult>;

    /// Probe account identity (e.g., call /user endpoint).
    /// Returns a display name for the authenticated account.
    fn probe_account(&self, credential: &str) -> Result<Option<String>>;

    /// Default OAuth/API scopes for this provider.
    fn default_scopes(&self) -> Vec<String>;

    /// Default child binary name for this provider.
    fn default_child(&self) -> &str;

    /// Default injection strategy for this provider.
    fn default_injection(&self) -> InjectionStrategy;

    /// Allowed API domains for this provider.
    fn allowed_domains(&self) -> Vec<String>;
}

pub struct AcquisitionResult {
    pub credential: String,       // the secret to store in vault
    pub account_id: Option<String>, // who authenticated
    pub auth_method: AuthMethod,
    pub scopes: Vec<String>,
    pub expires_at: Option<String>,
}
```

### GitHub Provider

```rust
pub struct GitHubProvider;

impl Provider for GitHubProvider {
    fn name(&self) -> &str { "github" }

    fn acquire(&self) -> Result<AcquisitionResult> {
        // OAuth device flow (RFC 8628):
        // 1. POST https://github.com/login/device/code
        //    (client_id, scope)
        // 2. Display user_code, open verification_uri in browser
        // 3. Poll POST https://github.com/login/oauth/access_token
        //    (client_id, device_code, grant_type)
        // 4. Return access_token
        //
        // Then probe_account() to get login name
    }

    fn acquire_manual(&self, secret_ref: &str) -> Result<AcquisitionResult> {
        // User has a PAT already stored in vault.
        // Optionally probe account to populate account_id.
    }

    fn probe_account(&self, credential: &str) -> Result<Option<String>> {
        // GET https://api.github.com/user with Bearer auth
        // Return response.login
    }

    fn default_scopes(&self) -> Vec<String> {
        vec!["repo".into(), "read:org".into()]
    }

    fn default_child(&self) -> &str { "github-connector" }

    fn default_injection(&self) -> InjectionStrategy {
        InjectionStrategy::Bearer
    }

    fn allowed_domains(&self) -> Vec<String> {
        vec!["api.github.com".into()]
    }
}
```

**Adding a second provider** (e.g., Slack) requires:
1. Create `src/connect/providers/slack.rs`
2. Implement `Provider` trait
3. Register in provider lookup

No changes to connection model, auth resolution, broker, or CLI.

## §4 — Connection Store

CRUD operations on connection TOML files + vault entries.

```rust
// src/connect public API (subset)

/// Load a connection by name. Returns durable state only.
pub fn load(name: &str) -> Result<ConnectionRecord>;

/// List all connections with computed status.
pub fn list() -> Result<Vec<ConnectionSummary>>;

/// Create a new connection (writes TOML + vault entry).
pub fn create(record: &ConnectionRecord, credential: &str) -> Result<()>;

/// Remove a connection. Checks referential integrity first.
pub fn remove(name: &str) -> Result<()>;

/// Resolve durable connection into execution-ready auth plan.
/// This is the ONLY place that decrypts vault material for connections.
pub fn resolve_auth(record: &ConnectionRecord) -> Result<AuthPlan>;
```

### Referential Integrity

Connections are referenced by `sources.toml` entries across projects.
Before removing a connection, scan all registered projects:

```rust
fn check_references(name: &str) -> Result<Vec<ProjectReference>> {
    // broker::sources::scan_all_sources() already exists
    // Filter for sources where source.connection == name
    // Return list of (project_path, source_name) pairs
}
```

If references exist, `remove()` returns an error listing them:
```
Error: connection "github" is referenced by:
  /home/user/patina/.patina/sources.toml → source "github"
  /home/user/docs/.patina/sources.toml → source "github-docs"
Use --force to remove anyway.
```

### Status Computation

Connection health is computed from durable metadata, not live checks:

```rust
pub enum ConnectionStatus {
    Connected,     // credential exists in vault, no errors
    Missing,       // credential not found in vault
    Expired,       // expires_at is in the past
    Errored,       // last_error is set (last auth attempt failed)
    Unchecked,     // never validated (last_validated is None)
}

pub fn compute_status(record: &ConnectionRecord) -> ConnectionStatus {
    // Check vault existence (does secret_ref resolve?)
    // Check expiry (is expires_at in the past?)
    // Check last_error (was there a recent failure?)
    // Check last_validated (has it ever been checked?)
}
```

This is fast (no network, no vault decryption — just metadata
checks + vault existence test) and suitable for `connect list`
and `connect status` commands.

## §5 — Path API

`src/paths.rs` gets a `connections` module:

```rust
pub mod connections {
    use super::*;

    /// Connection configs directory: `~/.patina/connections/`
    pub fn connections_dir() -> PathBuf {
        patina_home().join("connections")
    }

    /// Individual connection config: `~/.patina/connections/{name}.toml`
    pub fn connection_path(name: &str) -> PathBuf {
        connections_dir().join(format!("{}.toml", name))
    }
}
```

This replaces the hardcoded path at `broker/connection.rs:42-48`.

## §6 — CLI Surface

```
patina connect                     # show help
patina connect github              # OAuth device flow for GitHub
patina connect github --manual     # manual token (vault secret ref)
patina connect list                # table of all connections + status
patina connect show <name>         # full detail for one connection
patina connect status              # health summary
patina connect refresh <name>      # re-acquire credential
patina connect remove <name>       # delete (checks references)
patina connect remove <name> --force  # delete (skip reference check)
```

Integrates into the existing clap hierarchy as a top-level
subcommand, same level as `secrets`, `mother`, `schema`.

## §7 — Module Structure

```
src/
  connect/
    mod.rs                    # Public API: load, list, create, remove,
                              #   resolve_auth. Small, stable.
    internal/
      mod.rs                  # Wire internals
      model.rs                # ConnectionRecord, AuthConfig, AuthPlan,
                              #   ConnectionStatus, ConnectError
      store.rs                # TOML read/write/list/delete, referential
                              #   integrity checks
      status.rs               # compute_status() from metadata
      provider.rs             # Provider trait definition
    providers/
      mod.rs                  # Provider registry/lookup
      github.rs               # GitHub OAuth device flow (RFC 8628)
  commands/
    connect.rs                # CLI subcommands (clap)
```

Follows [[dependable-rust]]: `mod.rs` exposes a small public
interface, `internal/` hides implementation. Provider implementations
are separate from the core module — they populate the model but
don't define it.

## §8 — Scope Decision

**v1: Global only.** Connections live at `~/.patina/connections/`.
Credentials live in the global vault (`~/.patina/vault.age`).
`resolve_auth()` calls `get_global_secret()`.

**Why global:** Connections are user identity — "I am octocat on
GitHub." This doesn't vary by project. Different projects use the
same connection via `sources.toml`, which already varies per project.

**Migration path for project-local:** The `ConnectionScope` enum
has a `Global` variant. Future variants (`Project`, `Persona`)
would extend the resolution path in `resolve_auth()` to check
project or persona vaults. The `AuthPlan` interface doesn't change —
the broker still gets the same resolved plan regardless of scope.

## §9 — What Changes in Existing Code

### Keep (structurally right)

| File | What | Why keep |
|---|---|---|
| `src/secrets/mod.rs` | Vault substrate | Provider-agnostic encrypted storage works |
| `src/broker/spawn.rs` | Child spawn + sandbox | Mother-side credential custody is correct boundary |
| `src/broker/sources.rs` | Source-to-connection binding | Sources already name connections correctly |
| `src/broker/routing.rs` | Fact validation | Unrelated to auth path |
| `src/broker/lifecycle.rs` | Child lifecycle | Unrelated to auth path |
| `src/broker/cursor.rs` | Cursor management | Unrelated to auth path |

### Move (right responsibility, wrong location)

| From | To | What moves |
|---|---|---|
| `broker/connection.rs:42-48` | `paths::connections` | Hardcoded connection path |
| `broker/connection.rs` | `connect/internal/store.rs` | Connection loading/parsing |
| `broker/mod.rs:36-57` | `connect::resolve_auth()` | Auth resolution logic |

### Fix (broken contracts)

| File | What's broken | Fix |
|---|---|---|
| `broker/mod.rs:44-48` | Proceeds without auth when credential missing — violates `child.toml auth.required` | `resolve_auth()` returns error, broker fails before spawn |
| `broker/http.rs:79-86` | Hardcoded Bearer injection — GitHub model baked in | `AuthPlan` strategy dispatch |
| `commands/mother/mod.rs:103` | CLI text says "write to events.db" — code already routes to lake | Update help text to reflect destination routing |

### Delete or sever

| File | What to remove | Why |
|---|---|---|
| `broker/http.rs:12-13` | `use crate::plugin::{CredentialMapping, InjectionLocation}` | Plugin auth types don't belong in broker native-child path |
| `broker/http.rs:12` | `use crate::plugin::internal::host_support` | Broker imports from shared `http_util` instead |
| `broker/connection.rs` | Entire file | Replaced by `connect` module — `ConnectionConfig` is not a product model |

### Extract (not copy)

| From | To | What | Rule |
|---|---|---|---|
| `plugin/internal/host_support.rs` | `src/http_util.rs` | `validate_http_url`, `build_http_client`, `leak_check` | No plugin concepts, no connection concepts. Pure HTTP plumbing. Both plugin and broker import from here. |

### Add

| File | What | Why |
|---|---|---|
| `src/http_util.rs` | Shared HTTP utilities | Both plugin and broker need URL validation, client construction, leak detection |
| `src/paths.rs` | `connections` module | First-class path ownership |
| `src/connect/mod.rs` | Public API | Connection subsystem interface |
| `src/connect/internal/` | Model, store, status, provider trait | Domain layer |
| `src/connect/providers/github.rs` | GitHub OAuth device flow | First provider |
| `src/commands/connect.rs` | CLI subcommands | User-facing surface |
| `src/commands/mod.rs` | `pub mod connect;` | Wire CLI |

## Commits

Build order is model-out, not feature-in. Cleanup is interleaved
with construction — not deferred to the end.

1. **`paths: add connections path API`**
   `src/paths.rs` gets `connections` module. Tests.

2. **`connect: add domain model`**
   `ConnectionRecord`, `AuthConfig`, `AuthPlan`, `InjectionStrategy`,
   `ConnectionStatus`, `ConnectError`. Types only — no I/O.
   Round-trip TOML tests. These types live in `src/connect/`,
   NOT in `src/plugin/` or `src/broker/`.

3. **`connect: add store (TOML persistence + referential integrity)`**
   `load()`, `list()`, `create()`, `remove()` with reference checks.
   Uses `paths::connections`. Tests with temp directories.

4. **`connect: add auth plan resolution — fail closed`**
   `resolve_auth()` — the seam between durable metadata and runtime.
   Tests for each failure mode (missing vault, missing child, etc.).
   Missing credential is an error, not a warning.

5. **`connect: add provider trait + GitHub implementation`**
   `Provider` trait, GitHub OAuth device flow (RFC 8628),
   `probe_account()`, provider registry. Tests with mock provider.

6. **`broker: consume AuthPlan — sever plugin auth, extract HTTP utils`**
   Refactor `run_source`, `spawn_native`, `build_production_handler`
   to consume `AuthPlan`. Changes in this commit:
   - Extract `validate_http_url`, `build_http_client`, `leak_check`
     from `plugin/internal/host_support.rs` to shared `src/http_util.rs`.
     No plugin concepts, no connection concepts — pure HTTP plumbing.
     Both `plugin/internal/host_support.rs` and `broker/http.rs`
     import from this shared module.
   - Delete `broker/connection.rs` (replaced by `connect` module).
   - Delete `use crate::plugin::{CredentialMapping, InjectionLocation}`
     from `broker/http.rs`.
   - Delete `use crate::plugin::internal::host_support` from
     `broker/http.rs`. Broker imports from `http_util` + `connect`.
   - `broker/mod.rs` calls `connect::load()` + `connect::resolve_auth()`,
     never calls `get_global_secret` or `load_connection` directly.
   - `build_production_handler` takes `AuthPlan`, dispatches on
     `InjectionStrategy` (Bearer, Header, InProcess).
   - "Proceeding without auth" path removed — fail closed.
   - Verify: `grep -r "crate::plugin" src/broker/` returns zero
     matches. No duplicated utility implementations.

7. **`connect: add CLI commands`**
   `src/commands/connect.rs` with clap subcommands. Wire into
   `commands/mod.rs` and main CLI. All subcommands: github, list,
   show, status, refresh, remove.

8. **`broker: fix stale CLI text + help strings`**
   `commands/mother/mod.rs:103` — update `Run` help text to reflect
   destination routing (project events.db or lake), not just
   "write to events.db". Minor but prevents drift from compounding.

9. **`connect: end-to-end verification`**
   Full flow: `patina connect github` → `patina mother run <source>`.
   Exit criteria verification. Release.

## Open Questions

1. **GitHub OAuth App registration.** External dependency that blocks
   end-to-end testing (commit 8). Code builds and unit-tests without
   it (commits 1-7). Registration is quick (< 5 minutes) but
   requires a GitHub account decision (personal vs org).

2. **Vault existence check without decryption.** `compute_status()`
   needs to know if a secret exists without triggering Touch ID.
   Today `get_global_secret()` decrypts the entire vault. Options:
   (a) check registry (`secrets.toml`) instead of vault — fast but
   registry could be stale; (b) add `vault_has_secret(name)` that
   decrypts once and caches; (c) use session cache if `patina serve`
   is running. Option (a) is probably right for status display.

3. **InProcess injection and AuthPlan.** When `injection = InProcess`,
   the credential goes via `pipe/initialize` instead of HTTP proxy.
   The `spawn_native_with_plan` function needs to handle this case:
   include credential in `InitializeParams.auth` instead of in the
   HTTP handler. This is how `requires_in_process_token` works today
   (`spawn.rs:77-98`) — just driven by AuthPlan instead of manifest.

4. **Domain allowlist: connection vs manifest.** Today allowed domains
   come from `child.toml` manifest (`spawn.rs:159`). The connection
   record also carries `allowed_domains`. Which wins? Proposal:
   intersection — both must agree. The manifest declares what the
   child needs, the connection declares what the user authorized.

5. **Shared HTTP utilities — extract, don't copy.** When
   `broker/http.rs` stops importing from `plugin/internal/host_support`,
   it still needs `validate_http_url()`, `build_http_client()`, and
   `leak_check()`. **Decision: extract to a shared module.**

   Create `src/http_util.rs` (or `src/http/mod.rs` if it grows) with
   only the generic pieces:
   - URL/domain validation (`validate_http_url`)
   - HTTP client construction (`build_http_client`)
   - Leak detection (`leak_check`)
   - Generic request auth application (apply header/bearer to builder)

   This module has NO plugin concepts (no grant checking, no manifest
   parsing, no `CredentialMapping`). It has NO connection concepts
   (no `AuthPlan`, no `InjectionStrategy`). It is pure HTTP plumbing.

   Both `plugin/internal/host_support.rs` and `broker/http.rs` import
   from this shared module. Plugin adds grant gating on top. Broker
   adds AuthPlan dispatch on top.

   **Do not copy the utility functions into broker.** That trades one
   kind of drift for another — two copies of leak detection logic
   that diverge silently. The whole point of severing the import is
   to remove the *conceptual* dependency (plugin auth types), not
   the *utility* dependency (HTTP helpers).

   This extraction belongs in commit 6 (broker: consume AuthPlan).

## Belief Anchors

- [[mother-is-connection-and-continuity]] — Mother federates
  connections. This module is where connections are defined.
- [[wasm-host-boundary-hides-credentials]] — Conceptual precedent.
  The pipe/http proxy is the native-child equivalent.
- [[safety-boundaries]] — User consent before OAuth flow. Vault
  encryption with biometric auth. Project-scoped operations.
- [[defense-in-depth-over-perfect-isolation]] — Domain allowlist +
  injection strategy + leak detection. Multiple layers, not one
  perfect boundary.
