//! Mother command — the Patina daemon
//!
//! Mother is the always-running daemon that provides:
//! - Hot model caching (E5 embeddings)
//! - Cross-project knowledge access (scry API)
//! - Secrets caching (avoids repeated Touch ID prompts)
//! - Graph-based query routing
//!
//! # Command Structure
//!
//! ```text
//! patina mother                    # Show daemon status
//! patina mother start              # Start daemon (UDS default, TCP opt-in)
//! patina mother stop               # Graceful shutdown (not yet implemented)
//! patina mother status             # Health check (not yet implemented)
//! patina mother graph              # Graph operations (sync, link, unlink, stats, learn)
//! ```
//!
//! # Transport Model
//!
//! - Default: Unix domain socket at `~/.patina/run/serve.sock`
//! - Opt-in: TCP at `--host/--port` (bearer token required)
//!
//! # Example
//!
//! ```no_run
//! # fn main() -> anyhow::Result<()> {
//! // Start the daemon
//! // patina mother start
//!
//! // Show graph state
//! // patina mother graph
//!
//! // Add a relationship
//! // patina mother graph link patina dojo TESTS_WITH --evidence "benchmark subject"
//! # Ok(())
//! # }
//! ```

pub(crate) mod daemon;
pub(crate) mod graph;
pub(crate) mod microserver;

use anyhow::{bail, Context, Result};
use std::os::unix::fs::{FileTypeExt, PermissionsExt};
use std::path::Path;

use patina::paths;

// Re-export DaemonOptions for use in main.rs
pub use daemon::DaemonOptions;

/// Mother CLI subcommands
#[derive(Debug, Clone, clap::Subcommand)]
pub enum MotherCommands {
    /// Start the mother daemon
    ///
    /// Starts the daemon listening on Unix socket (default) or TCP (opt-in).
    /// The daemon provides scry API, secrets caching, and cross-project routing.
    Start {
        /// Bind to TCP host (enables network access; default: UDS only)
        #[arg(long)]
        host: Option<String>,

        /// TCP port (only used with --host)
        #[arg(long, default_value = "50051")]
        port: u16,

        /// Run as MCP server (JSON-RPC over stdio) instead of HTTP
        #[arg(long)]
        mcp: bool,
    },

    /// Stop the mother daemon (not yet implemented)
    ///
    /// Sends a graceful shutdown signal to the running daemon.
    Stop,

    /// Show daemon status (not yet implemented)
    ///
    /// Displays health, uptime, connected projects, and model cache state.
    Status,

    /// Graph operations — manage cross-project relationships
    #[command(subcommand)]
    Graph(GraphCommands),
}

/// Graph subcommands (nested under `patina mother graph`)
#[derive(Debug, Clone, clap::Subcommand)]
pub enum GraphCommands {
    /// Sync graph nodes from registry
    ///
    /// Creates nodes for all projects and repos in ~/.patina/registry.yaml.
    /// Run this after adding new repos with `patina repo add`.
    Sync,

    /// Show graph state
    ///
    /// Displays all nodes and edges in the relationship graph.
    Show {
        /// Show only nodes
        #[arg(long)]
        nodes: bool,

        /// Show only edges
        #[arg(long)]
        edges: bool,
    },

    /// Add a relationship between nodes
    ///
    /// Creates a directed edge from one node to another.
    /// Edge types: USES, LEARNS_FROM, TESTS_WITH, SIBLING, DOMAIN
    Link {
        /// Source node (e.g., "patina")
        from: String,

        /// Target node (e.g., "dojo")
        to: String,

        /// Relationship type (e.g., "TESTS_WITH")
        edge_type: String,

        /// Optional evidence/reason for this relationship
        #[arg(long)]
        evidence: Option<String>,
    },

    /// Remove a relationship
    Unlink {
        /// Source node
        from: String,

        /// Target node
        to: String,

        /// Relationship type
        edge_type: String,
    },

    /// Show edge usage statistics
    ///
    /// Displays usage statistics for all edges: how often each edge
    /// was used in graph routing, and how often it led to useful results.
    Stats,

    /// Learn edge weights from usage data
    ///
    /// Updates edge weights based on how often they led to useful results.
    /// Edges need at least 5 uses before their weights can be updated.
    Learn {
        /// Learning rate (0.0-1.0, default 0.1)
        ///
        /// Higher values make weights change faster but may oscillate.
        #[arg(long, default_value = "0.1")]
        alpha: f32,
    },
}

/// Execute mother command from CLI
pub fn execute_cli(
    command: Option<MotherCommands>,
    run_mcp: impl FnOnce() -> Result<()>,
) -> Result<()> {
    match command {
        None => {
            // Bare `patina mother` — show status (or help for now)
            println!("Mother daemon commands:\n");
            println!("  patina mother start    Start the daemon");
            println!("  patina mother stop     Stop the daemon (not yet implemented)");
            println!("  patina mother status   Show daemon status (not yet implemented)");
            println!("  patina mother graph    Graph operations\n");
            println!("Run 'patina mother --help' for details.");
            Ok(())
        }
        Some(MotherCommands::Start { host, port, mcp }) => {
            if mcp {
                run_mcp()
            } else {
                let options = DaemonOptions { host, port };
                daemon::run_server(options)
            }
        }
        Some(MotherCommands::Stop) => stop_daemon(),
        Some(MotherCommands::Status) => show_status(),
        Some(MotherCommands::Graph(graph_cmd)) => execute_graph(graph_cmd),
    }
}

/// Execute graph subcommand
fn execute_graph(command: GraphCommands) -> Result<()> {
    match command {
        GraphCommands::Sync => graph::sync_from_registry(),
        GraphCommands::Show { nodes, edges } => graph::show_graph(nodes, edges),
        GraphCommands::Link {
            from,
            to,
            edge_type,
            evidence,
        } => graph::add_link(&from, &to, &edge_type, evidence.as_deref()),
        GraphCommands::Unlink {
            from,
            to,
            edge_type,
        } => graph::remove_link(&from, &to, &edge_type),
        GraphCommands::Stats => graph::show_stats(),
        GraphCommands::Learn { alpha } => graph::learn_weights(alpha),
    }
}

// === Daemon lifecycle commands ===

/// Stop the mother daemon
fn stop_daemon() -> Result<()> {
    let pid_path = paths::serve::pid_path();

    // Check if PID file exists
    if !pid_path.exists() {
        println!("Mother daemon is not running (no PID file).");
        return Ok(());
    }

    // Read PID
    let pid_str = std::fs::read_to_string(&pid_path)
        .with_context(|| format!("reading PID file {}", pid_path.display()))?;
    let pid: i32 = pid_str
        .trim()
        .parse()
        .with_context(|| format!("parsing PID from '{}'", pid_str.trim()))?;

    // Check if process is running
    let is_running = unsafe { libc::kill(pid, 0) == 0 };
    if !is_running {
        // Stale PID file — clean up
        println!("Mother daemon is not running (stale PID file).");
        let _ = std::fs::remove_file(&pid_path);
        return Ok(());
    }

    println!("Stopping mother daemon (PID {})...", pid);

    // Send SIGTERM
    let result = unsafe { libc::kill(pid, libc::SIGTERM) };
    if result != 0 {
        let err = std::io::Error::last_os_error();
        bail!("Failed to send SIGTERM to PID {}: {}", pid, err);
    }

    // Wait for process to exit (poll up to 5 seconds)
    for i in 0..50 {
        std::thread::sleep(std::time::Duration::from_millis(100));
        let still_running = unsafe { libc::kill(pid, 0) == 0 };
        if !still_running {
            println!("Mother daemon stopped.");
            // Clean up files if daemon didn't (shouldn't happen with proper signal handling)
            let _ = std::fs::remove_file(&pid_path);
            cleanup_socket();
            return Ok(());
        }
        if i == 25 {
            println!("   Still waiting...");
        }
    }

    // Process didn't exit in time
    println!("Warning: daemon did not stop within 5 seconds.");
    println!("   You may need to: kill -9 {}", pid);
    Ok(())
}

/// Show daemon status
fn show_status() -> Result<()> {
    let pid_path = paths::serve::pid_path();
    let socket_path = paths::serve::socket_path();

    // Check PID file
    let pid = if pid_path.exists() {
        match std::fs::read_to_string(&pid_path) {
            Ok(s) => s.trim().parse::<i32>().ok(),
            Err(_) => None,
        }
    } else {
        None
    };

    // Check if process is running
    let is_running = pid
        .map(|p| unsafe { libc::kill(p, 0) == 0 })
        .unwrap_or(false);

    if !is_running {
        println!("Mother daemon: stopped");
        if pid.is_some() {
            println!("   (stale PID file exists — run `patina mother stop` to clean up)");
        }
        return Ok(());
    }

    let pid = pid.unwrap();
    println!("Mother daemon: running");
    println!("   PID: {}", pid);
    println!("   Socket: {}", socket_path.display());

    // Try to get health info via UDS
    match query_health() {
        Ok(health) => {
            println!("   Version: {}", health.version);
            println!("   Uptime: {}s", health.uptime_secs);
        }
        Err(e) => {
            println!("   Health check failed: {}", e);
        }
    }

    Ok(())
}

/// Health response from daemon
#[derive(serde::Deserialize)]
struct HealthInfo {
    version: String,
    uptime_secs: u64,
}

/// Query daemon health via UDS
fn query_health() -> Result<HealthInfo> {
    use std::io::{Read, Write};
    use std::os::unix::net::UnixStream;

    let socket_path = paths::serve::socket_path();
    let mut stream = UnixStream::connect(&socket_path)
        .with_context(|| format!("connecting to {}", socket_path.display()))?;

    // Set timeout
    stream.set_read_timeout(Some(std::time::Duration::from_secs(2)))?;

    // Send HTTP request
    let request = "GET /health HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n";
    stream.write_all(request.as_bytes())?;

    // Read response
    let mut response = Vec::new();
    stream.read_to_end(&mut response)?;

    // Parse HTTP response (simple extraction)
    let response_str = String::from_utf8_lossy(&response);
    let body_start = response_str.find("\r\n\r\n").map(|i| i + 4).unwrap_or(0);
    let body = &response_str[body_start..];

    serde_json::from_str(body).with_context(|| "parsing health response")
}

// === Socket management (shared with daemon) ===

/// Ensure the runtime directory exists with correct permissions.
///
/// Creates `~/.patina/run/` with 0o700 if it doesn't exist.
/// Refuses to start if the directory is world/group accessible.
fn ensure_run_dir() -> Result<()> {
    let run_dir = paths::serve::run_dir();

    if !run_dir.exists() {
        std::fs::create_dir_all(&run_dir)
            .with_context(|| format!("creating runtime directory {}", run_dir.display()))?;
        std::fs::set_permissions(&run_dir, std::fs::Permissions::from_mode(0o700))
            .with_context(|| format!("setting permissions on {}", run_dir.display()))?;
    } else {
        let meta = std::fs::metadata(&run_dir)
            .with_context(|| format!("reading metadata for {}", run_dir.display()))?;
        let mode = meta.permissions().mode() & 0o777;
        if mode & 0o077 != 0 {
            bail!(
                "Refusing to start: {} has permissions {:o} (group/world accessible).\n  \
                 Fix with: chmod 700 {}",
                run_dir.display(),
                mode,
                run_dir.display()
            );
        }
    }

    Ok(())
}

/// Remove a stale socket file safely.
///
/// Only unlinks if the path is a socket AND owned by the current user.
/// Refuses to remove non-socket files or files owned by other users.
fn cleanup_stale_socket(socket_path: &Path) -> Result<()> {
    if !socket_path.exists() {
        return Ok(());
    }

    let meta = std::fs::symlink_metadata(socket_path)
        .with_context(|| format!("reading metadata for {}", socket_path.display()))?;

    if !meta.file_type().is_socket() {
        bail!(
            "Refusing to start: {} exists but is not a socket.\n  \
             Remove manually if safe: rm {}",
            socket_path.display(),
            socket_path.display()
        );
    }

    use std::os::unix::fs::MetadataExt;
    let file_uid = meta.uid();
    let my_uid = unsafe { libc::getuid() };
    if file_uid != my_uid {
        bail!(
            "Refusing to start: {} is owned by uid {} (you are {}).\n  \
             This may indicate a security issue.",
            socket_path.display(),
            file_uid,
            my_uid
        );
    }

    std::fs::remove_file(socket_path)
        .with_context(|| format!("removing stale socket {}", socket_path.display()))?;

    Ok(())
}

/// Set up the Unix domain socket for serving.
///
/// 1. Ensure ~/.patina/run/ exists with 0o700
/// 2. Clean up stale socket (safe unlink)
/// 3. Bind UnixListener
/// 4. Set socket to 0o600
pub fn setup_unix_listener() -> Result<std::os::unix::net::UnixListener> {
    use std::os::unix::net::UnixListener;

    ensure_run_dir()?;

    let socket_path = paths::serve::socket_path();
    cleanup_stale_socket(&socket_path)?;

    let listener = UnixListener::bind(&socket_path)
        .with_context(|| format!("binding socket {}", socket_path.display()))?;

    std::fs::set_permissions(&socket_path, std::fs::Permissions::from_mode(0o600))
        .with_context(|| format!("setting permissions on {}", socket_path.display()))?;

    Ok(listener)
}

/// Remove the socket file on clean shutdown.
pub fn cleanup_socket() {
    let socket_path = paths::serve::socket_path();
    if socket_path.exists() {
        let _ = std::fs::remove_file(&socket_path);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mother_command_variants() {
        let start = MotherCommands::Start {
            host: None,
            port: 50051,
            mcp: false,
        };
        assert!(matches!(start, MotherCommands::Start { .. }));

        let graph = MotherCommands::Graph(GraphCommands::Sync);
        assert!(matches!(graph, MotherCommands::Graph(_)));
    }
}
