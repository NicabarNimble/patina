//! Connect command — manage external service connections.
//!
//! Subcommands: github (acquire), list, show, status, refresh, remove.
//! Integrates with the connect subsystem for lifecycle operations.

use anyhow::{bail, Result};
use std::io::{BufRead, Write};

/// Connect CLI subcommands
#[derive(Debug, Clone, clap::Subcommand)]
pub enum ConnectCommands {
    /// Connect to GitHub via OAuth device flow
    ///
    /// Starts the GitHub OAuth device flow (RFC 8628): displays a user code,
    /// opens the verification URL in your browser, and polls for authorization.
    /// The token is stored in the vault and a connection record is created.
    Github {
        /// Use manual token entry instead of OAuth flow
        #[arg(long)]
        manual: bool,

        /// Connection name (default: "github")
        #[arg(long)]
        name: Option<String>,
    },

    /// List all connections with status
    ///
    /// Shows name, provider, account, and computed health status for
    /// each connection in ~/.patina/connections/.
    List,

    /// Show detailed information for a connection
    Show {
        /// Connection name
        name: String,
    },

    /// Show health summary for all connections
    Status,

    /// Re-acquire credential for a connection
    Refresh {
        /// Connection name
        name: String,
    },

    /// Remove a connection
    ///
    /// Checks for references in sources.toml across registered projects.
    /// Use --force to skip the reference check.
    Remove {
        /// Connection name
        name: String,

        /// Skip reference check
        #[arg(long)]
        force: bool,
    },
}

/// Execute connect command from CLI.
pub fn execute_cli(command: Option<ConnectCommands>) -> Result<()> {
    match command {
        None => {
            println!("Connection management commands:\n");
            println!("  patina connect github      Connect to GitHub (OAuth device flow)");
            println!("  patina connect list         List all connections");
            println!("  patina connect show <name>  Show connection details");
            println!("  patina connect status       Health summary");
            println!("  patina connect refresh <n>  Re-acquire credential");
            println!("  patina connect remove <n>   Remove a connection\n");
            println!("Run 'patina connect --help' for details.");
            Ok(())
        }
        Some(ConnectCommands::Github { manual, name }) => connect_github(manual, name),
        Some(ConnectCommands::List) => list_connections(),
        Some(ConnectCommands::Show { name }) => show_connection(&name),
        Some(ConnectCommands::Status) => show_status(),
        Some(ConnectCommands::Refresh { name }) => refresh_connection(&name),
        Some(ConnectCommands::Remove { name, force }) => remove_connection(&name, force),
    }
}

/// Connect to GitHub via OAuth device flow or manual token.
fn connect_github(manual: bool, name: Option<String>) -> Result<()> {
    let conn_name = name.unwrap_or_else(|| "github".to_string());

    // Check if connection already exists
    if patina::connect::load(&conn_name).is_ok() {
        bail!(
            "connection '{}' already exists. Use `patina connect remove {}` first, \
             or `patina connect refresh {}` to re-acquire.",
            conn_name,
            conn_name,
            conn_name
        );
    }

    let provider = patina::connect::provider("github")
        .map_err(|e| anyhow::anyhow!("{}", e))?;

    let result = if manual {
        eprintln!("Enter a GitHub personal access token (PAT).");
        eprintln!("The token will be stored in the Patina vault.\n");

        eprint!("Token: ");
        std::io::stderr().flush()?;

        let mut token = String::new();
        std::io::BufReader::new(std::io::stdin()).read_line(&mut token)?;
        let token = token.trim().to_string();

        if token.is_empty() {
            bail!("no token provided");
        }

        provider
            .acquire_manual(&token)
            .map_err(|e| anyhow::anyhow!("{}", e))?
    } else {
        provider
            .acquire()
            .map_err(|e| anyhow::anyhow!("{}", e))?
    };

    // Build connection record
    let now = chrono::Utc::now().to_rfc3339();
    let record = patina::connect::ConnectionRecord {
        schema_version: 0,
        identity: patina::connect::ConnectionIdentity {
            name: conn_name.clone(),
            provider: provider.name().to_string(),
            account_id: result.account_id.clone(),
            auth_method: result.auth_method.clone(),
            scopes: result.scopes.clone(),
            is_default: true,
            created_at: now.clone(),
            updated_at: now,
            last_validated: None,
            scope: patina::connect::ConnectionScope::Global,
        },
        auth: patina::connect::AuthConfig {
            injection: provider.default_injection(),
            secret_ref: format!("{}:{}", provider.name(), conn_name),
            child: provider.default_child().to_string(),
            allowed_domains: provider.allowed_domains(),
            refresh_capable: !manual,
            expires_at: result.expires_at,
            last_error: None,
        },
    };

    // Create connection (writes TOML + vault entry)
    patina::connect::create(&record, &result.credential)
        .map_err(|e| anyhow::anyhow!("{}", e))?;

    println!(
        "Connection '{}' created successfully.",
        conn_name
    );
    if let Some(ref account) = record.identity.account_id {
        println!("  Account: {}", account);
    }
    println!("  Provider: {}", record.identity.provider);
    println!("  Child: {}", record.auth.child);

    Ok(())
}

/// List all connections with status.
fn list_connections() -> Result<()> {
    let connections = patina::connect::list()
        .map_err(|e| anyhow::anyhow!("{}", e))?;

    if connections.is_empty() {
        println!("No connections configured.");
        println!("  Run `patina connect github` to set up GitHub.");
        return Ok(());
    }

    println!(
        "{:<16} {:<12} {:<20} {}",
        "NAME", "PROVIDER", "ACCOUNT", "STATUS"
    );
    for conn in &connections {
        println!(
            "{:<16} {:<12} {:<20} {}",
            conn.name,
            conn.provider,
            conn.account_id.as_deref().unwrap_or("—"),
            format_status(&conn.status),
        );
    }

    Ok(())
}

/// Show detailed information for a connection.
fn show_connection(name: &str) -> Result<()> {
    let record = patina::connect::load(name)
        .map_err(|e| anyhow::anyhow!("{}", e))?;

    println!("Connection: {}", record.identity.name);
    println!("  Provider: {}", record.identity.provider);
    if let Some(ref account) = record.identity.account_id {
        println!("  Account: {}", account);
    }
    println!("  Auth method: {:?}", record.identity.auth_method);
    if !record.identity.scopes.is_empty() {
        println!("  Scopes: {}", record.identity.scopes.join(", "));
    }
    println!("  Default: {}", record.identity.is_default);
    println!("  Created: {}", record.identity.created_at);
    println!("  Updated: {}", record.identity.updated_at);
    if let Some(ref validated) = record.identity.last_validated {
        println!("  Last validated: {}", validated);
    }

    println!("\n  Child: {}", record.auth.child);
    println!("  Injection: {:?}", record.auth.injection);
    println!("  Secret ref: {}", record.auth.secret_ref);
    if !record.auth.allowed_domains.is_empty() {
        println!("  Domains: {}", record.auth.allowed_domains.join(", "));
    }
    println!("  Refresh capable: {}", record.auth.refresh_capable);
    if let Some(ref expires) = record.auth.expires_at {
        println!("  Expires: {}", expires);
    }
    if let Some(ref error) = record.auth.last_error {
        println!("  Last error: {}", error);
    }

    Ok(())
}

/// Show health summary for all connections.
fn show_status() -> Result<()> {
    let connections = patina::connect::list()
        .map_err(|e| anyhow::anyhow!("{}", e))?;

    if connections.is_empty() {
        println!("No connections configured.");
        return Ok(());
    }

    let mut connected = 0;
    let mut issues = 0;
    for conn in &connections {
        match conn.status {
            patina::connect::ConnectionStatus::Connected => connected += 1,
            _ => issues += 1,
        }
        if !matches!(conn.status, patina::connect::ConnectionStatus::Connected) {
            eprintln!(
                "  {}: {}",
                conn.name,
                format_status(&conn.status)
            );
        }
    }

    println!(
        "{} connection(s): {} connected, {} with issues",
        connections.len(),
        connected,
        issues
    );

    Ok(())
}

/// Refresh a connection's credential.
fn refresh_connection(name: &str) -> Result<()> {
    let record = patina::connect::load(name)
        .map_err(|e| anyhow::anyhow!("{}", e))?;

    if !record.auth.refresh_capable {
        bail!(
            "connection '{}' is not refresh-capable (manual token). \
             Remove and re-create instead.",
            name
        );
    }

    let provider = patina::connect::provider(&record.identity.provider)
        .map_err(|e| anyhow::anyhow!("{}", e))?;

    let result = provider
        .acquire()
        .map_err(|e| anyhow::anyhow!("{}", e))?;

    // Update the connection record with new credential
    let now = chrono::Utc::now().to_rfc3339();
    let mut updated = record.clone();
    updated.identity.updated_at = now;
    updated.identity.account_id = result.account_id.clone();
    updated.auth.last_error = None;

    patina::connect::create(&updated, &result.credential)
        .map_err(|e| anyhow::anyhow!("{}", e))?;

    println!("Connection '{}' refreshed.", name);
    if let Some(ref account) = result.account_id {
        println!("  Account: {}", account);
    }

    Ok(())
}

/// Remove a connection.
fn remove_connection(name: &str, force: bool) -> Result<()> {
    // Verify it exists first
    let _record = patina::connect::load(name)
        .map_err(|e| anyhow::anyhow!("{}", e))?;

    if !force {
        // Confirm with user
        eprint!("Remove connection '{}'? [y/N] ", name);
        std::io::stderr().flush()?;

        let mut input = String::new();
        std::io::BufReader::new(std::io::stdin()).read_line(&mut input)?;

        if !input.trim().eq_ignore_ascii_case("y") {
            println!("Aborted.");
            return Ok(());
        }
    }

    patina::connect::remove(name, force)
        .map_err(|e| anyhow::anyhow!("{}", e))?;

    println!("Connection '{}' removed.", name);
    Ok(())
}

/// Format a connection status for display.
fn format_status(status: &patina::connect::ConnectionStatus) -> &'static str {
    match status {
        patina::connect::ConnectionStatus::Connected => "connected",
        patina::connect::ConnectionStatus::Missing => "credential missing",
        patina::connect::ConnectionStatus::Expired => "expired",
        patina::connect::ConnectionStatus::Errored => "error",
        patina::connect::ConnectionStatus::Unchecked => "unchecked",
    }
}
