# Design: Connection Model — patina connect with OAuth Device Flow

## Approach

New `src/connect/` module in the main binary, plus `patina connect`
CLI commands. Reuses existing `src/secrets/` vault infrastructure
for credential storage. Adds OAuth device flow (RFC 8628) for GitHub
and a connection config format at `~/.patina/connections/`.

This is not a new crate — it's a module in the main binary that
orchestrates vault + connection config. The connection model is the
bridge between pipe protocol auth and the user.

## 1. Module Structure

```
src/
  connect/
    mod.rs              # public API: connect, status, refresh, remove
    oauth.rs            # OAuth device flow (RFC 8628)
    config.rs           # connection config read/write
  commands/
    connect.rs          # CLI subcommands (patina connect ...)
```

## 2. OAuth Device Flow (RFC 8628)

### 2.1 Protocol Steps

The device flow is designed for CLI tools — no redirect URI needed,
no embedded web server. The user approves in their browser.

```
Step 1: Device Authorization Request
  POST https://github.com/login/device/code
  Body: client_id=<patina_client_id>&scope=repo,read:org
  Response: { device_code, user_code, verification_uri, expires_in, interval }

Step 2: Display Code to User
  "Enter code ABCD-1234 at https://github.com/login/device"
  Open browser automatically if possible

Step 3: Poll for Token (every `interval` seconds)
  POST https://github.com/login/oauth/access_token
  Body: client_id=<id>&device_code=<code>&grant_type=urn:ietf:params:oauth:grant-type:device_code
  Response: { access_token, token_type, scope } or { error: "authorization_pending" }

Step 4: Token Acquired
  Store access_token in vault as "github:<user>" or "github:default"
  Create connection config at ~/.patina/connections/github.toml
```

### 2.2 Implementation

```rust
// src/connect/oauth.rs

use crate::secrets;
use anyhow::{bail, Result};

/// GitHub OAuth App client ID.
/// Registered at https://github.com/settings/applications/new
/// with "Device Flow" enabled. This is a public client ID (no secret).
const GITHUB_CLIENT_ID: &str = "PLACEHOLDER_REGISTER_BEFORE_BUILD";

/// GitHub OAuth scopes needed for connector.
/// - repo: read issues, PRs, code (private repos)
/// - read:org: read org membership (for org-level queries)
const GITHUB_SCOPES: &str = "repo,read:org";

/// Device authorization response from GitHub.
#[derive(Debug, serde::Deserialize)]
struct DeviceAuthResponse {
    device_code: String,
    user_code: String,
    verification_uri: String,
    expires_in: u64,
    interval: u64,
}

/// Token response from GitHub.
#[derive(Debug, serde::Deserialize)]
struct TokenResponse {
    access_token: Option<String>,
    error: Option<String>,
    error_description: Option<String>,
}

/// Run the OAuth device flow for GitHub.
///
/// Returns the access token on success.
pub fn github_device_flow() -> Result<String> {
    let client = reqwest::blocking::Client::new();

    // Step 1: Request device code
    eprintln!("Requesting device authorization from GitHub...");
    let auth_resp: DeviceAuthResponse = client
        .post("https://github.com/login/device/code")
        .header("Accept", "application/json")
        .form(&[
            ("client_id", GITHUB_CLIENT_ID),
            ("scope", GITHUB_SCOPES),
        ])
        .send()?
        .json()?;

    // Step 2: Display code and open browser
    eprintln!();
    eprintln!("  Open: {}", auth_resp.verification_uri);
    eprintln!("  Enter code: {}", auth_resp.user_code);
    eprintln!();

    // Try to open browser automatically
    #[cfg(target_os = "macos")]
    {
        let _ = std::process::Command::new("open")
            .arg(&auth_resp.verification_uri)
            .spawn();
    }

    // Step 3: Poll for token
    eprintln!("Waiting for approval...");
    let poll_interval = std::time::Duration::from_secs(auth_resp.interval.max(5));
    let deadline = std::time::Instant::now()
        + std::time::Duration::from_secs(auth_resp.expires_in);

    loop {
        if std::time::Instant::now() > deadline {
            bail!("Authorization expired. Run `patina connect github` to try again.");
        }

        std::thread::sleep(poll_interval);

        let token_resp: TokenResponse = client
            .post("https://github.com/login/oauth/access_token")
            .header("Accept", "application/json")
            .form(&[
                ("client_id", GITHUB_CLIENT_ID),
                ("device_code", &auth_resp.device_code),
                ("grant_type", "urn:ietf:params:oauth:grant-type:device_code"),
            ])
            .send()?
            .json()?;

        if let Some(token) = token_resp.access_token {
            eprintln!("  Approved!");
            return Ok(token);
        }

        match token_resp.error.as_deref() {
            Some("authorization_pending") => continue,
            Some("slow_down") => {
                // Back off by adding 5 seconds
                std::thread::sleep(std::time::Duration::from_secs(5));
                continue;
            }
            Some("expired_token") => {
                bail!("Authorization expired. Run `patina connect github` to try again.");
            }
            Some("access_denied") => {
                bail!("Authorization denied by user.");
            }
            Some(other) => {
                let desc = token_resp.error_description.unwrap_or_default();
                bail!("OAuth error: {} — {}", other, desc);
            }
            None => continue,
        }
    }
}
```

## 3. Connection Config Format

```toml
# ~/.patina/connections/github.toml

[connection]
name = "github"
provider = "github"
credential = "github:default"          # vault secret name
child = "github-connector"             # child binary to use
created = "2026-03-06T20:49:43Z"
method = "oauth"                       # oauth | manual

[oauth]
client_id = "Iv1.xxxxxxxx"
scopes = ["repo", "read:org"]
```

### 3.1 config.rs — Connection Config Read/Write

```rust
// src/connect/config.rs

use anyhow::{bail, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// Where connections live.
fn connections_dir() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".patina/connections")
}

/// A connection config linking auth to a connector child.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionConfig {
    pub connection: ConnectionMeta,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub oauth: Option<OAuthMeta>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionMeta {
    pub name: String,
    pub provider: String,
    pub credential: String,         // vault secret name
    pub child: String,              // child binary name
    pub created: String,            // ISO 8601
    pub method: String,             // "oauth" or "manual"
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OAuthMeta {
    pub client_id: String,
    pub scopes: Vec<String>,
}

impl ConnectionConfig {
    /// Load a connection config by name.
    pub fn load(name: &str) -> Result<Self> {
        let path = connections_dir().join(format!("{}.toml", name));
        if !path.exists() {
            bail!("Connection '{}' not found", name);
        }
        let content = std::fs::read_to_string(&path)?;
        let config: Self = toml::from_str(&content)?;
        Ok(config)
    }

    /// Save a connection config.
    pub fn save(&self) -> Result<()> {
        let dir = connections_dir();
        std::fs::create_dir_all(&dir)?;
        let path = dir.join(format!("{}.toml", self.connection.name));
        let content = toml::to_string_pretty(self)?;
        std::fs::write(&path, content)?;
        Ok(())
    }

    /// Remove a connection config.
    pub fn remove(name: &str) -> Result<()> {
        let path = connections_dir().join(format!("{}.toml", name));
        if path.exists() {
            std::fs::remove_file(&path)?;
        }
        Ok(())
    }

    /// List all connection configs.
    pub fn list() -> Result<Vec<Self>> {
        let dir = connections_dir();
        if !dir.exists() {
            return Ok(vec![]);
        }
        let mut configs = Vec::new();
        for entry in std::fs::read_dir(&dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.extension().map(|e| e == "toml").unwrap_or(false) {
                if let Ok(config) = Self::load(
                    path.file_stem().unwrap().to_str().unwrap_or("")
                ) {
                    configs.push(config);
                }
            }
        }
        Ok(configs)
    }
}
```

## 4. Public API (connect/mod.rs)

```rust
// src/connect/mod.rs

mod config;
mod oauth;

pub use config::ConnectionConfig;

use anyhow::Result;

/// Create a new connection via OAuth device flow.
///
/// 1. Run OAuth device flow → get token
/// 2. Store token in vault (existing secrets infra)
/// 3. Create connection config
pub fn connect_github() -> Result<()> {
    // Run OAuth flow
    let token = oauth::github_device_flow()?;

    // Store in vault
    let secret_name = "github:default";
    crate::secrets::add_secret(
        secret_name,
        &token,
        None,       // no env var mapping needed
        true,       // global vault
        None,       // no project root
    )?;
    eprintln!("  Token stored in vault: {}", secret_name);

    // Create connection config
    let config = ConnectionConfig {
        connection: config::ConnectionMeta {
            name: "github".to_string(),
            provider: "github".to_string(),
            credential: secret_name.to_string(),
            child: "github-connector".to_string(),
            created: chrono::Utc::now().to_rfc3339(),
            method: "oauth".to_string(),
        },
        oauth: Some(config::OAuthMeta {
            client_id: oauth::GITHUB_CLIENT_ID.to_string(),
            scopes: vec!["repo".to_string(), "read:org".to_string()],
        }),
    };
    config.save()?;
    eprintln!("  Connection configured: ~/.patina/connections/github.toml");

    eprintln!();
    eprintln!("Done. GitHub data flows on next `patina mother run github`.");
    Ok(())
}

/// Show status of all connections.
pub fn connect_status() -> Result<()> {
    let configs = ConnectionConfig::list()?;

    if configs.is_empty() {
        eprintln!("No connections configured.");
        eprintln!("Run `patina connect github` to create one.");
        return Ok(());
    }

    for config in &configs {
        let credential_status = match crate::secrets::get_global_secret(
            &config.connection.credential
        ) {
            Ok(Some(_)) => "connected",
            Ok(None) => "missing credential",
            Err(_) => "vault error",
        };

        eprintln!("  {}: {} ({})",
            config.connection.name,
            credential_status,
            config.connection.method,
        );
    }

    Ok(())
}

/// Refresh a connection (re-run OAuth flow).
pub fn connect_refresh(name: &str) -> Result<()> {
    // Load existing config to verify it exists
    let _config = ConnectionConfig::load(name)?;

    match name {
        "github" => {
            let token = oauth::github_device_flow()?;
            let secret_name = "github:default";
            crate::secrets::add_secret(secret_name, &token, None, true, None)?;
            eprintln!("  Token refreshed for {}", name);
        }
        _ => {
            anyhow::bail!("Don't know how to refresh '{}'. Only github is supported.", name);
        }
    }

    Ok(())
}

/// Remove a connection (delete config + credential).
pub fn connect_remove(name: &str) -> Result<()> {
    let config = ConnectionConfig::load(name)?;

    // Remove credential from vault
    let _ = crate::secrets::remove_secret(&config.connection.credential, true, None);

    // Remove config file
    ConnectionConfig::remove(name)?;

    eprintln!("  Removed connection '{}'", name);
    Ok(())
}
```

## 5. CLI Commands

```rust
// src/commands/connect.rs

use clap::Subcommand;
use anyhow::Result;

#[derive(Subcommand)]
pub enum ConnectCommand {
    /// Connect to GitHub via OAuth device flow
    Github,
    /// Show connection status
    Status,
    /// Refresh a connection (re-authorize)
    Refresh {
        /// Connection name to refresh
        name: String,
    },
    /// Remove a connection
    Remove {
        /// Connection name to remove
        name: String,
    },
}

pub fn run(cmd: ConnectCommand) -> Result<()> {
    match cmd {
        ConnectCommand::Github => crate::connect::connect_github(),
        ConnectCommand::Status => crate::connect::connect_status(),
        ConnectCommand::Refresh { name } => crate::connect::connect_refresh(&name),
        ConnectCommand::Remove { name } => crate::connect::connect_remove(&name),
    }
}
```

Integration into main CLI (in `src/main.rs` or wherever clap
commands are declared):

```rust
#[derive(Subcommand)]
enum Commands {
    // ... existing commands ...
    /// Manage external connections (GitHub, Slack, etc.)
    Connect {
        #[command(subcommand)]
        command: ConnectCommand,
    },
}
```

## 6. Credential Delivery via pipe/initialize

When Mother spawns a child, it reads the connection config, decrypts
the credential from vault, and passes it via pipe/initialize:

```rust
// In Mother's spawn logic (src/broker/spawn.rs — mother-broker spec)

fn build_init_params(connection_name: &str) -> Result<InitializeParams> {
    let config = ConnectionConfig::load(connection_name)?;

    // Decrypt credential from vault — triggers Touch ID if needed
    let token = crate::secrets::get_global_secret(&config.connection.credential)?
        .ok_or_else(|| anyhow::anyhow!(
            "credential '{}' not found in vault. Run `patina connect {}`",
            config.connection.credential, connection_name
        ))?;

    Ok(InitializeParams {
        protocol_version: "1.0".to_string(),
        auth: Some(AuthConfig {
            token,  // plain String — zeroize happens after serialize
            provider: config.connection.provider,
        }),
    })
}
```

The token lives in Mother's memory only long enough to serialize
to the child's stdin. After `serde_json::to_string()` writes it
to the pipe, the `InitializeParams` is dropped. Mother can wrap
the token in `Zeroizing<String>` (from the zeroize crate already in
tree) for the brief window between vault decrypt and pipe write.

## 7. GitHub OAuth App Registration

External dependency — must be done before OAuth can be tested.

### 7.1 Registration Steps

1. Go to https://github.com/settings/applications/new
2. Application name: "Patina"
3. Homepage URL: https://github.com/NicabarNimble/patina
4. Authorization callback URL: (not used for device flow, any value)
5. Enable "Device Flow" checkbox
6. Note the Client ID (public, goes in source code)
7. No client secret needed (device flow is a public client)

### 7.2 Scope Justification

| Scope | Why |
|-------|-----|
| `repo` | Read issues, PRs, code from private repos |
| `read:org` | Read org membership for org-level queries |

These are the minimum scopes needed for the github-connector to
fetch issues and PRs from both public and private repos.

### 7.3 Token Storage

The OAuth token is stored as a vault secret with name format
`github:default` (or `github:<username>` for multi-account future).
Uses the existing age-encrypted vault with Keychain + Touch ID.

## 8. Manual Fallback (Non-OAuth)

For users who prefer PATs or can't use OAuth (CI, headless):

```bash
# Existing workflow still works
patina secrets add github-token ghp_xxx

# Create connection config manually
# ~/.patina/connections/github.toml
# [connection]
# name = "github"
# provider = "github"
# credential = "github-token"
# child = "github-connector"
# method = "manual"
```

`patina connect github --manual` could prompt for a token and create
the connection config without OAuth. But the basic manual workflow
(secrets add + hand-edit config) already works. Don't over-build.

## Commits

1. `connect: add OAuth device flow for GitHub`
   — src/connect/oauth.rs with RFC 8628 implementation. Placeholder
   client ID until GitHub app is registered.

2. `connect: add connection config format`
   — src/connect/config.rs with ConnectionConfig read/write/list.
   ~/.patina/connections/ directory.

3. `connect: add public API (connect, status, refresh, remove)`
   — src/connect/mod.rs orchestrating OAuth + vault + config.

4. `connect: add CLI commands`
   — src/commands/connect.rs with clap subcommands. Wire into
   main CLI. Verify: `patina connect --help` shows subcommands.

5. `connect: document credential delivery path`
   — Update mother-broker design to show how InitializeParams
   gets built from connection config.

## Key Files

- `src/connect/mod.rs` — public API (connect, status, refresh, remove)
- `src/connect/oauth.rs` — OAuth device flow (RFC 8628)
- `src/connect/config.rs` — connection config format
- `src/commands/connect.rs` — CLI subcommands
- `src/secrets/mod.rs` — vault (reused for token storage)
- `src/secrets/vault.rs` — age encryption (reused)

## Open Questions

1. **GitHub OAuth App registration.** This is an external dependency
   that blocks testing. The code can be written with a placeholder
   client ID and tested by swapping in the real ID later. Registration
   itself is quick (< 5 minutes) but requires a GitHub account
   decision (personal vs org).

2. **Multi-account support.** The design uses `github:default` as the
   vault secret name. For future multi-account (personal + work), the
   name would be `github:<label>` and connection configs would be
   `github-personal.toml`, `github-work.toml`. The current design
   doesn't preclude this — it just doesn't implement the UX for
   choosing between accounts.
