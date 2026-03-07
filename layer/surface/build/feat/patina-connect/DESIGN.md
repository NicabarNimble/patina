# Design: Connection Model — One Command to Link Data

## Why This Work Exists

Today, connecting Patina to GitHub is four manual steps:

1. Generate a PAT on GitHub (navigate settings, create token, copy)
2. `patina secrets add github-token ghp_xxx` (store in vault)
3. Edit `secret-grants.toml` to allow the forge plugin to use it
4. Configure the forge plugin's owner/repo params somewhere

This is an expert workflow. It requires understanding vault mechanics,
secret grants, and plugin configuration. The user has to know how
Patina's security model works before they can fetch their first issue.

[[mother-holds-connections-pipes-transform]] says Mother manages
connections. A connection is a named pair: credential + connector child
config. One command creates both:

```
patina connect github
  -> OAuth device flow (browser opens, user approves)
  -> Token stored in vault
  -> Connection config created
  -> Done. GitHub data flows on next scrape.
```

This is the UX bridge between "install Patina" and "get value from
external data."

**Origin:** [[session-20260306-123021]] (connection model: one command
creates credential + connector config), [[session-20260306-174214]]
(audit: credential delivery via pipe/initialize, zeroize boundary is
Mother's code).

## Design Decisions

### 1. OAuth Device Flow (RFC 8628), Not Web Redirect

The device flow is designed for CLI tools — no redirect URI, no
embedded web server, no localhost port conflicts. The user sees a
code, opens a browser, approves. The CLI polls for the token.

This matters because Patina is a terminal tool. A redirect-based
OAuth flow would require opening a port, handling the callback, and
dealing with browser trust issues. The device flow is simpler, more
reliable, and works over SSH.

**GitHub-specific requirements:**
- Register a GitHub OAuth App with "Device Flow" enabled
- Public client (no client secret needed)
- Scopes: `repo` (read issues/PRs from private repos) + `read:org`
  (org membership for org-level queries)

### 2. Reuse Existing Vault, Don't Reinvent

The `src/secrets/` infrastructure already handles:
- Age-encrypted storage with macOS Keychain + Touch ID
- Session caching (decrypt once, use many times)
- Global vs project-scoped secrets

Connections use this directly. The OAuth token is stored as a vault
secret with name `github:default`. The connection config references
it by name. No new crypto, no new storage format.

### 3. Connection Config at ~/.patina/connections/

Per-provider TOML file linking auth to a connector child:

```toml
# ~/.patina/connections/github.toml

[connection]
name = "github"
provider = "github"
credential = "github:default"      # vault secret name
child = "github-connector"         # which child binary to use
created = "2026-03-06T20:49:43Z"
method = "oauth"                   # oauth | manual

[oauth]
client_id = "Iv1.xxxxxxxx"
scopes = ["repo", "read:org"]
```

This is referenced by name in `sources.toml`:
```toml
[sources.github]
connection = "github"              # -> ~/.patina/connections/github.toml
params = { owner = "NicabarNimble", repo = "patina" }
```

### 4. Credential Delivery Boundary

The security boundary for credentials is Mother's code, not the
types crate or the child. The delivery path:

```
Vault (age-encrypted, Touch ID)
  |
  | Mother decrypts -> Zeroizing<String>
  |
  | Serialize to child's stdin as InitializeParams.auth.token
  |
  | Drop Zeroizing<String> (memory zeroed)
  |
Child process (holds token for its lifetime)
  |
  | OS sandbox prevents exfiltration
  |
Process exits (memory freed)
```

Mother wraps the token in `Zeroizing<String>` (from the `zeroize`
crate, already in tree via `age`) for the brief window between vault
decrypt and pipe write. The child receives a plain `String` because
it must read the token to use it.

### 5. Manual Fallback for PAT Users

Not everyone wants OAuth. CI systems, headless servers, and users
with existing PATs need a manual path:

```bash
patina secrets add github-token ghp_xxx

# Then create connection config manually or via:
patina connect github --manual
# (prompts for vault secret name, creates config without OAuth)
```

The basic workflow (secrets add + hand-edit TOML) already works
without any new code. The `--manual` flag is a convenience, not a
requirement. Don't over-build.

## Module Structure

```
src/
  connect/
    mod.rs              # public API: connect, status, refresh, remove
    oauth.rs            # OAuth device flow (RFC 8628)
    config.rs           # connection config read/write/list
  commands/
    connect.rs          # CLI subcommands
```

This is a module in the main binary, not a new crate. It orchestrates
existing vault infrastructure + new connection config. The module
boundary follows [[dependable-rust]]: `mod.rs` exposes four public
functions, internals are hidden.

## Public API

```rust
// src/connect/mod.rs

/// Create a new connection via OAuth device flow.
/// Flow: OAuth -> token -> vault store -> connection config
pub fn connect_github() -> Result<()>;

/// Show status of all connections (name, auth status, method).
pub fn connect_status() -> Result<()>;

/// Refresh a connection (re-run OAuth flow, update vault).
pub fn connect_refresh(name: &str) -> Result<()>;

/// Remove a connection (delete config + credential from vault).
pub fn connect_remove(name: &str) -> Result<()>;
```

## CLI Commands

```
patina connect github          # OAuth device flow
patina connect status          # show all connections
patina connect refresh github  # re-authorize
patina connect remove github   # delete connection + credential
```

Integrates into the existing clap command hierarchy as a top-level
subcommand.

## What's NOT In Scope

- **Token refresh automation** — GitHub PATs don't expire. OAuth
  tokens from the device flow are long-lived. Automatic refresh
  is future work if/when expiring tokens become common.
- **Multi-account UX** — the design uses `github:default` as the
  vault secret name. Multi-account (`github:personal`,
  `github:work`) is structurally supported but the UX for choosing
  between accounts is deferred.
- **Non-GitHub providers** — the OAuth flow is GitHub-specific (client
  ID, scopes, endpoints). Slack, Jira, etc. would need their own
  flow implementations. The connection config format is
  provider-agnostic; only `oauth.rs` is GitHub-specific.
- **Credential rotation alerts** — "your token expires in 7 days"
  notifications. Future work.

## Belief Anchors

- [[mother-holds-connections-pipes-transform]] — Mother manages
  connections. This module is where Mother learns about external
  auth sources.
- [[host-proxied-io-is-the-security-model]] — credentials never in
  environment variables or files for children. Vault -> stdin pipe.
- [[safety-boundaries]] — user consent before OAuth flow. Explicit
  scope declaration. Vault encryption with biometric auth.

## Open Questions

1. **GitHub OAuth App registration.** External dependency that blocks
   testing. Code works with a placeholder client ID. Registration is
   quick (< 5 minutes) but requires a GitHub account decision
   (personal vs org). Must be done before OAuth can be tested.

2. **Multi-account support.** Current design is single-account per
   provider. The vault secret naming (`github:default`) and config
   file naming (`github.toml`) support exactly one. For multi-account,
   names would be `github:personal`, `github:work` with configs
   `github-personal.toml`, `github-work.toml`. Doesn't need solving
   now but the naming convention should anticipate it.

## Commits

1. `connect: add OAuth device flow for GitHub` — src/connect/oauth.rs
   with RFC 8628 implementation. Placeholder client ID.

2. `connect: add connection config format` — src/connect/config.rs
   with ConnectionConfig read/write/list.

3. `connect: add public API (connect, status, refresh, remove)` —
   src/connect/mod.rs orchestrating OAuth + vault + config.

4. `connect: add CLI commands` — src/commands/connect.rs with clap
   subcommands. Wire into main CLI.

5. `connect: document credential delivery path` — Update mother-broker
   design to show how InitializeParams gets built from connection
   config.

## Key Files

- `src/connect/mod.rs` — public API
- `src/connect/oauth.rs` — OAuth device flow
- `src/connect/config.rs` — connection config format
- `src/commands/connect.rs` — CLI subcommands
- `src/secrets/mod.rs` — vault (reused for token storage)
- `src/secrets/vault.rs` — age encryption (reused)
