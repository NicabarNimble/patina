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
//! patina mother stop               # Graceful shutdown
//! patina mother status             # Health check
//! patina mother install            # Install launchd supervisor
//! patina mother uninstall          # Remove launchd supervisor
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

pub(crate) mod adapters;
pub(crate) mod daemon;
pub(crate) mod graph;
pub(crate) mod loader;
pub(crate) mod toys;

// Moved to mother crate — re-export for daemon.rs
pub(crate) use mother_crate::registry;

use anyhow::{bail, Context, Result};
use serde::Deserialize;
use std::path::Path;
#[cfg(target_os = "macos")]
use std::process::Command;

use patina::paths;
use patina::session::SessionManager;

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

    /// Stop the mother daemon
    Stop,

    /// Show daemon status
    Status,

    /// Install launchd supervisor (macOS)
    Install,

    /// Uninstall launchd supervisor (macOS)
    Uninstall,

    /// Graph operations — manage cross-project relationships
    #[command(subcommand)]
    Graph(GraphCommands),

    /// Search cross-project beliefs (beliefs + persona values)
    ///
    /// FTS5 search across all synced beliefs in graph.db.
    /// Run `patina mother graph sync` first to populate the index.
    Search {
        /// Search query
        query: String,

        /// Maximum results to return
        #[arg(long, default_value = "10")]
        limit: usize,
    },

    /// Run a source — fetch, validate, and route facts to destination
    ///
    /// Spawns the child for the named source, fetches facts, validates
    /// against the child manifest, and routes to the configured destination
    /// (project events.db or lake) with content-hash dedup and transactional
    /// cursor management. Auth is resolved via the connect subsystem.
    Run {
        /// Source name (as defined in .patina/sources.toml)
        name: String,

        /// Bypass OS sandbox (for debugging only)
        #[arg(long)]
        no_sandbox: bool,
    },

    /// Legacy parity command (retired after native DuckLake removal)
    Parity {
        /// Source name (as defined in .patina/sources.toml)
        name: String,

        /// Bypass OS sandbox (for debugging only)
        #[arg(long)]
        no_sandbox: bool,

        /// Optional fresh lake name for clean parity baseline
        #[arg(long)]
        fresh_lake: Option<String>,
    },

    /// Show configured sources with status
    ///
    /// Lists all sources from .patina/sources.toml with last run timestamp,
    /// fact count, and status. Use --prune to remove orphaned cursors.
    Sources {
        /// Remove orphaned cursors (cursors with no matching source)
        #[arg(long)]
        prune: bool,
    },

    /// Toy registry operations
    #[command(subcommand)]
    Toys(ToysCommands),
}

#[derive(Debug, Clone, clap::Subcommand)]
pub enum ToysCommands {
    /// Show toy registry status
    Status,

    /// Check local toy WIT files against pinned versions
    Check,

    /// Sync local toy WIT files against upstream WASI repos
    Sync,

    /// Pull pinned upstream WIT for one toy
    Pull {
        /// Toy id from registry (for example: wasi-http)
        name: Option<String>,

        /// Pull all WASI toys at pinned versions
        #[arg(long)]
        all: bool,
    },
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

    /// Query the belief graph
    ///
    /// Search beliefs, find supports/attacks edges, and discover which
    /// projects hold a belief.
    #[command(subcommand)]
    Query(QueryCommands),
}

/// Query subcommands (nested under `patina mother graph query`)
#[derive(Debug, Clone, clap::Subcommand)]
pub enum QueryCommands {
    /// Search beliefs by text (FTS5)
    ///
    /// Returns beliefs with metrics (health_score, evidence_count, etc.)
    Belief {
        /// Search query
        query: String,

        /// Maximum results
        #[arg(long, default_value = "10")]
        limit: usize,
    },

    /// Show beliefs that support a given belief
    ///
    /// Returns supporting beliefs across all projects.
    Supports {
        /// Belief ID to query
        belief_id: String,
    },

    /// Show beliefs that attack a given belief
    ///
    /// Returns attacking beliefs with defeated status.
    Attacks {
        /// Belief ID to query
        belief_id: String,
    },

    /// Show which projects have a given belief
    Projects {
        /// Belief ID to query
        belief_id: String,
    },
}

/// Execute mother command from CLI
pub fn execute_cli(command: Option<MotherCommands>) -> Result<()> {
    match command {
        None => {
            // Bare `patina mother` — show status (or help for now)
            println!("Mother daemon commands:\n");
            println!("  patina mother start    Start the daemon");
            println!("  patina mother stop     Stop the daemon");
            println!("  patina mother status   Show daemon status");
            println!("  patina mother install  Install launchd supervisor");
            println!("  patina mother uninstall Remove launchd supervisor");
            println!("  patina mother graph    Graph operations");
            println!("  patina mother toys     Toy registry operations");
            println!("  patina mother search   Cross-project belief search\n");
            println!("Run 'patina mother --help' for details.");
            Ok(())
        }
        Some(MotherCommands::Start { host, port, mcp }) => {
            if mcp {
                bail!("MCP server path has been retired; start daemon without --mcp")
            } else {
                let options = DaemonOptions { host, port };
                daemon::run_server(options)
            }
        }
        Some(MotherCommands::Stop) => stop_daemon(),
        Some(MotherCommands::Status) => show_status(),
        Some(MotherCommands::Install) => install_supervisor(),
        Some(MotherCommands::Uninstall) => uninstall_supervisor(),
        Some(MotherCommands::Graph(graph_cmd)) => execute_graph(graph_cmd),
        Some(MotherCommands::Search { query, limit }) => graph::search_beliefs_cli(&query, limit),
        Some(MotherCommands::Run { name, no_sandbox }) => run_source_cli(&name, no_sandbox),
        Some(MotherCommands::Parity {
            name,
            no_sandbox,
            fresh_lake,
        }) => run_source_parity_cli(&name, no_sandbox, fresh_lake.as_deref()),
        Some(MotherCommands::Sources { prune }) => show_sources_cli(prune),
        Some(MotherCommands::Toys(ToysCommands::Status)) => {
            let project_root = SessionManager::find_project_root()
                .context("`patina mother toys status` must run in a Patina project")?;
            toys::toys_status(&project_root)
        }
        Some(MotherCommands::Toys(ToysCommands::Check)) => {
            let project_root = SessionManager::find_project_root()
                .context("`patina mother toys check` must run in a Patina project")?;
            toys::toys_check(&project_root)
        }
        Some(MotherCommands::Toys(ToysCommands::Sync)) => {
            let project_root = SessionManager::find_project_root()
                .context("`patina mother toys sync` must run in a Patina project")?;
            toys::toys_sync(&project_root)
        }
        Some(MotherCommands::Toys(ToysCommands::Pull { name, all })) => {
            let project_root = SessionManager::find_project_root()
                .context("`patina mother toys pull` must run in a Patina project")?;
            if all {
                toys::toys_pull_all(&project_root)
            } else {
                let name = name.ok_or_else(|| {
                    anyhow::anyhow!("`patina mother toys pull` needs a toy name, or use `--all`")
                })?;
                toys::toys_pull(&project_root, &name)
            }
        }
    }
}

#[cfg(target_os = "macos")]
const MOTHER_LAUNCHD_LABEL: &str = "com.patina.mother";

#[cfg(target_os = "macos")]
fn launch_agents_dir() -> Result<std::path::PathBuf> {
    let home = dirs::home_dir().context("unable to resolve home directory")?;
    Ok(home.join("Library/LaunchAgents"))
}

#[cfg(target_os = "macos")]
fn launchd_plist_path() -> Result<std::path::PathBuf> {
    Ok(launch_agents_dir()?.join(format!("{}.plist", MOTHER_LAUNCHD_LABEL)))
}

#[cfg(target_os = "macos")]
fn xml_escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

#[cfg(target_os = "macos")]
fn render_launchd_plist(exe_path: &Path) -> String {
    let exe = xml_escape(&exe_path.display().to_string());
    format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<!DOCTYPE plist PUBLIC \"-//Apple//DTD PLIST 1.0//EN\" \"http://www.apple.com/DTDs/PropertyList-1.0.dtd\">\n<plist version=\"1.0\">\n<dict>\n  <key>Label</key>\n  <string>{}</string>\n  <key>ProgramArguments</key>\n  <array>\n    <string>{}</string>\n    <string>mother</string>\n    <string>start</string>\n  </array>\n  <key>RunAtLoad</key>\n  <true/>\n  <key>KeepAlive</key>\n  <true/>\n</dict>\n</plist>\n",
        MOTHER_LAUNCHD_LABEL, exe
    )
}

#[cfg(target_os = "macos")]
fn launchctl_domains() -> Vec<String> {
    let uid = unsafe { libc::geteuid() };
    vec![format!("gui/{uid}"), format!("user/{uid}")]
}

fn install_supervisor() -> Result<()> {
    #[cfg(not(target_os = "macos"))]
    {
        bail!("patina mother install is only supported on macOS");
    }

    #[cfg(target_os = "macos")]
    {
        fn run_launchctl(args: &[&str]) -> Result<()> {
            let status = Command::new("launchctl")
                .args(args)
                .status()
                .with_context(|| format!("running launchctl {}", args.join(" ")))?;
            if !status.success() {
                bail!("launchctl {} failed with status {}", args.join(" "), status);
            }
            Ok(())
        }

        let plist_path = launchd_plist_path()?;
        let parent = plist_path
            .parent()
            .context("invalid launchd plist parent path")?;
        std::fs::create_dir_all(parent)
            .with_context(|| format!("creating {}", parent.display()))?;

        let exe_path = std::env::current_exe().context("resolving current executable")?;
        let plist = render_launchd_plist(&exe_path);
        std::fs::write(&plist_path, plist)
            .with_context(|| format!("writing {}", plist_path.display()))?;

        for domain in launchctl_domains() {
            let service = format!("{}/{}", domain, MOTHER_LAUNCHD_LABEL);
            let _ = Command::new("launchctl")
                .arg("bootout")
                .arg(&service)
                .status();
        }
        let plist_arg = plist_path.to_string_lossy().to_string();
        let mut selected_domain = None;
        let mut last_error = None;
        for domain in launchctl_domains() {
            match run_launchctl(&["bootstrap", &domain, &plist_arg]) {
                Ok(()) => {
                    selected_domain = Some(domain);
                    break;
                }
                Err(error) => {
                    last_error = Some(error);
                }
            }
        }
        let domain = selected_domain.ok_or_else(|| {
            last_error.unwrap_or_else(|| anyhow::anyhow!("launchctl bootstrap failed"))
        })?;
        let service = format!("{}/{}", domain, MOTHER_LAUNCHD_LABEL);
        run_launchctl(&["enable", &service])?;

        println!("Installed launchd plist: {}", plist_path.display());
        println!("Service label: {}", MOTHER_LAUNCHD_LABEL);
        Ok(())
    }
}

fn uninstall_supervisor() -> Result<()> {
    #[cfg(not(target_os = "macos"))]
    {
        bail!("patina mother uninstall is only supported on macOS");
    }

    #[cfg(target_os = "macos")]
    {
        fn run_launchctl(args: &[&str]) -> Result<()> {
            let status = Command::new("launchctl")
                .args(args)
                .status()
                .with_context(|| format!("running launchctl {}", args.join(" ")))?;
            if !status.success() {
                bail!("launchctl {} failed with status {}", args.join(" "), status);
            }
            Ok(())
        }

        let plist_path = launchd_plist_path()?;
        for domain in launchctl_domains() {
            let service = format!("{}/{}", domain, MOTHER_LAUNCHD_LABEL);
            let _ = Command::new("launchctl")
                .arg("bootout")
                .arg(&service)
                .status();
            let _ = run_launchctl(&["disable", &service]);
        }

        if plist_path.exists() {
            std::fs::remove_file(&plist_path)
                .with_context(|| format!("removing {}", plist_path.display()))?;
            println!("Removed launchd plist: {}", plist_path.display());
        } else {
            println!("Launchd plist not found: {}", plist_path.display());
        }

        Ok(())
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
        GraphCommands::Query(query_cmd) => graph::query_beliefs_cli(query_cmd),
    }
}

// === Broker CLI commands ===

/// Run a source via the broker
fn run_source_cli(name: &str, no_sandbox: bool) -> Result<()> {
    let project_root = std::env::current_dir()?;

    // Find the source in this project's sources.toml
    let source = patina::mother::broker::sources::find_source(&project_root, name)?
        .with_context(|| format!("source '{}' not found in .patina/sources.toml", name))?;

    let result = patina::mother::broker::run_source(&source, &project_root, no_sandbox)?;

    println!(
        "{}: {} facts written, {} dedup skipped{}",
        name,
        result.inserted,
        result.dedup_skipped,
        result
            .cursor
            .as_ref()
            .map(|c| format!(", cursor: {}", c))
            .unwrap_or_default()
    );

    Ok(())
}

fn run_source_parity_cli(name: &str, no_sandbox: bool, fresh_lake: Option<&str>) -> Result<()> {
    let _ = (name, no_sandbox, fresh_lake);
    bail!(
        "legacy source parity runtime has been removed; `patina mother parity` is retired for this branch"
    )
}

/// Show configured sources with status
fn show_sources_cli(prune: bool) -> Result<()> {
    let project_root = std::env::current_dir()?;

    if prune {
        prune_orphaned_cursors(&project_root)?;
        return Ok(());
    }

    let statuses = patina::mother::broker::status(&project_root)?;

    if statuses.is_empty() {
        println!("No sources configured. Add sources to .patina/sources.toml");
        return Ok(());
    }

    println!("Sources:");
    for s in &statuses {
        println!(
            "  {:<20} last run: {:<28} facts: {:<6} status: {}",
            s.name,
            s.last_run.as_deref().unwrap_or("never"),
            s.fact_count,
            s.status,
        );
    }

    Ok(())
}

/// Remove orphaned cursors (cursors with no matching source in sources.toml)
fn prune_orphaned_cursors(project_root: &Path) -> Result<()> {
    let project_sources = patina::mother::broker::sources::load_project_sources(project_root)?;
    let source_names: std::collections::HashSet<String> = project_sources
        .map(|ps| ps.sources.iter().map(|s| s.name.clone()).collect())
        .unwrap_or_default();

    let conn = patina::eventlog::open_events_db_at(project_root)?;

    let mut stmt = conn.prepare("SELECT source_name FROM broker_cursors")?;
    let cursor_names: Vec<String> = stmt
        .query_map([], |row| row.get(0))?
        .filter_map(|r| r.ok())
        .collect();

    let orphaned: Vec<&String> = cursor_names
        .iter()
        .filter(|name| !source_names.contains(*name))
        .collect();

    if orphaned.is_empty() {
        println!("No orphaned cursors found.");
        return Ok(());
    }

    println!("Orphaned cursors:");
    for name in &orphaned {
        println!("  {} (no matching source in sources.toml)", name);
    }

    print!("Remove {} orphaned cursor(s)? [y/N] ", orphaned.len());
    std::io::Write::flush(&mut std::io::stdout())?;

    let mut input = String::new();
    std::io::BufRead::read_line(&mut std::io::BufReader::new(std::io::stdin()), &mut input)?;

    if input.trim().eq_ignore_ascii_case("y") {
        for name in &orphaned {
            conn.execute(
                "DELETE FROM broker_cursors WHERE source_name = ?1",
                [name.as_str()],
            )?;
        }
        println!("Removed {} orphaned cursor(s).", orphaned.len());
    } else {
        println!("Aborted.");
    }

    Ok(())
}

// === Daemon lifecycle commands ===

/// Stop the mother daemon
fn stop_daemon() -> Result<()> {
    let pid_path = paths::serve::pid_path();
    let socket_path = paths::serve::socket_path();
    match mother_crate::lifecycle::stop_daemon(&pid_path, &socket_path)? {
        mother_crate::lifecycle::StopResult::NotRunningNoPid => {
            println!("Mother daemon is not running (no PID file).");
        }
        mother_crate::lifecycle::StopResult::NotRunningStalePid => {
            println!("Mother daemon is not running (stale PID file).");
        }
        mother_crate::lifecycle::StopResult::Stopped => {
            println!("Mother daemon stopped.");
        }
        mother_crate::lifecycle::StopResult::TimedOut => {
            println!("Warning: daemon did not stop within 5 seconds.");
            println!("   You may need to inspect and kill the daemon manually.");
        }
    }
    Ok(())
}

/// Show daemon status
fn show_status() -> Result<()> {
    let pid_path = paths::serve::pid_path();
    let socket_path = paths::serve::socket_path();

    let status = mother_crate::lifecycle::probe_status(&pid_path, &socket_path)?;
    if !status.running {
        println!("Mother daemon: stopped");
        if status.stale_pid_file {
            println!("   (stale PID file exists — run `patina mother stop` to clean up)");
        }
        println!("\n   Tip: broker source status lives under `patina mother sources`.");
        return Ok(());
    }

    println!("Mother daemon: running");
    if let Some(pid) = status.pid {
        println!("   PID: {}", pid);
    }
    println!("   Socket: {}", socket_path.display());

    match status.health {
        Some(health) => {
            println!("   Version: {}", health.version);
            println!("   Uptime: {}s", health.uptime_secs);
            println!(
                "   Children loaded: {}",
                if health.child_count > 0 {
                    health.child_count
                } else {
                    health.children.len()
                }
            );
            println!("   Registered projects: {}", health.registered_projects);
            if let Some(state_db_bytes) = health.state_db_bytes {
                println!("   State DB bytes: {}", state_db_bytes);
            }
            if let Some(project_uid) = &health.active_project_uid {
                println!("   Active project: {}", project_uid);
            }
            if let Some(databases) = &health.active_project_databases {
                if let Some(bytes) = databases.events_db_bytes {
                    println!("   Active events.db bytes: {}", bytes);
                }
                if let Some(bytes) = databases.patina_db_bytes {
                    println!("   Active patina.db bytes: {}", bytes);
                }
                if let Some(bytes) = databases.runtime_db_bytes {
                    println!("   Active runtime.db bytes: {}", bytes);
                }
            }
            let loaded_children: std::collections::HashSet<String> =
                health.children.iter().map(|c| c.name.clone()).collect();
            if !health.children.is_empty() {
                println!("   Children:");
                for child in health.children {
                    println!("     {}: {}", child.name, child.status);
                }
            }

            if let Ok(project_root) = SessionManager::find_project_root() {
                match load_project_manifest(&project_root) {
                    Ok(manifest) => {
                        if !manifest.needs.children.is_empty() {
                            println!("   Project child needs:");
                            for child in &manifest.needs.children {
                                let marker = if loaded_children.contains(child) {
                                    "ok"
                                } else {
                                    "missing"
                                };
                                println!("     {}: {}", child, marker);
                            }
                        }
                    }
                    Err(error) => {
                        println!("   Project child manifest: {}", error);
                    }
                }
            }
        }
        None => {
            if let Some(error) = status.health_error {
                println!("   Health check failed: {}", error);
            }
        }
    }

    println!("\n   Tip: broker source status lives under `patina mother sources`.");

    Ok(())
}

#[derive(Debug, Deserialize)]
struct ProjectManifest {
    project: ManifestProject,
    needs: ManifestNeeds,
}

#[derive(Debug, Deserialize)]
struct ManifestProject {
    schema: u32,
}

#[derive(Debug, Deserialize)]
struct ManifestNeeds {
    children: Vec<String>,
}

fn load_project_manifest(project_root: &Path) -> Result<ProjectManifest> {
    let manifest_path = project_root.join(".patina/manifest.toml");
    let content = std::fs::read_to_string(&manifest_path)
        .with_context(|| format!("missing {}", manifest_path.display()))?;
    let manifest: ProjectManifest =
        toml::from_str(&content).with_context(|| format!("invalid {}", manifest_path.display()))?;
    if manifest.project.schema != 1 {
        bail!(
            "unsupported .patina/manifest.toml schema {} (expected 1)",
            manifest.project.schema
        );
    }
    Ok(manifest)
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

        let install = MotherCommands::Install;
        assert!(matches!(install, MotherCommands::Install));

        let uninstall = MotherCommands::Uninstall;
        assert!(matches!(uninstall, MotherCommands::Uninstall));

        let toys = MotherCommands::Toys(ToysCommands::Status);
        assert!(matches!(toys, MotherCommands::Toys(_)));
    }

    #[test]
    #[cfg(target_os = "macos")]
    fn launchd_plist_contains_required_fields() {
        let plist = render_launchd_plist(Path::new("/tmp/patina"));
        assert!(plist.contains("<key>Label</key>"));
        assert!(plist.contains(MOTHER_LAUNCHD_LABEL));
        assert!(plist.contains("<string>mother</string>"));
        assert!(plist.contains("<string>start</string>"));
        assert!(plist.contains("<key>RunAtLoad</key>"));
        assert!(plist.contains("<key>KeepAlive</key>"));
    }

    #[test]
    #[cfg(target_os = "macos")]
    fn xml_escape_escapes_special_characters() {
        let escaped = xml_escape("a&b<c>\"d\'e");
        assert_eq!(escaped, "a&amp;b&lt;c&gt;&quot;d&apos;e");
    }
}
