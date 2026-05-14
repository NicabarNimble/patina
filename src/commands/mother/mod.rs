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
//! patina mother restart            # Supervisor-aware restart
//! patina mother status             # Health check
//! patina mother install            # Install system supervisor (launchd/systemd-user)
//! patina mother uninstall          # Remove system supervisor
//! patina mother graph              # Graph operations (sync, link, unlink, stats, learn)
//! patina mother children           # Child registry source/sync operations
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
pub(crate) mod audit;
pub(crate) mod children;
pub(crate) mod daemon;
pub(crate) mod federation;
pub(crate) mod graph;
pub(crate) mod integrity;
pub(crate) mod loader;
pub(crate) mod skills_lifecycle;
pub(crate) mod skills_sandbox;
pub(crate) mod toys;

// Moved to mother crate — re-export for daemon.rs
pub(crate) use mother_crate::registry;

use anyhow::{bail, Context, Result};
use rusqlite::OptionalExtension;
use serde::Deserialize;
use std::io::IsTerminal;
use std::path::{Path, PathBuf};
#[cfg(unix)]
use std::process::{Command, Stdio};
use walkdir::{DirEntry, WalkDir};

use patina::paths;
use patina::session::SessionManager;

// Re-export daemon option types for use in main.rs
pub use daemon::{DaemonOptions, DaemonStartupProfile, RivetIntegrationProfile};

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

        /// Startup profile: `full` auto-warms children, `core` keeps control-plane only
        #[arg(long, value_enum, default_value_t = DaemonStartupProfile::Full)]
        profile: DaemonStartupProfile,

        /// Rivet integration profile (`disabled` preserves current behavior)
        #[arg(long, value_enum, default_value_t = RivetIntegrationProfile::Disabled)]
        rivet: RivetIntegrationProfile,

        /// Run as MCP server (JSON-RPC over stdio) instead of HTTP
        #[arg(long)]
        mcp: bool,
    },

    /// Stop the mother daemon
    Stop,

    /// Restart mother using configured supervisor backend when available
    Restart,

    /// Show daemon status
    Status,

    /// Install system supervisor (launchd on macOS, systemd --user on Linux)
    Install,

    /// Uninstall system supervisor
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

    /// Federation query surface operations
    #[command(subcommand)]
    Federation(FederationCommands),

    /// Child registry control-plane operations
    #[command(subcommand)]
    Children(ChildrenCommands),

    /// Lifecycle operations
    #[command(subcommand)]
    Lifecycle(LifecycleCommands),

    /// Project registration operations (check-in/sync/list)
    #[command(subcommand)]
    Projects(ProjectsCommands),

    /// Discover skills exposed by installed children
    #[command(subcommand)]
    Skills(SkillsCommands),

    /// Mother-owned view buffers and display shapes
    #[command(subcommand)]
    View(ViewCommands),
}

#[derive(Debug, Clone, clap::Subcommand)]
#[command(disable_help_subcommand = true)]
pub enum SkillsCommands {
    /// List installed children with skill packages
    List,

    /// Report child skill projection status for the selected/inferred HITL
    Status {
        /// Optional child name; omitted means all fixture children in the selected scope
        child: Option<String>,

        /// Check global/user HITL scope instead of project/workspace scope
        #[arg(long)]
        global: bool,

        /// Output raw JSON
        #[arg(long)]
        json: bool,
    },

    /// Plan or apply sync for stale/conflicted child skill projections
    Sync {
        /// Optional child name; omitted means all stale/conflicted fixture children in scope
        child: Option<String>,

        /// Check global/user HITL scope instead of project/workspace scope
        #[arg(long)]
        global: bool,

        /// Preview actions without writing projection files
        #[arg(long)]
        dry_run: bool,

        /// Apply force-required conflict actions after review
        #[arg(long)]
        force: bool,

        /// Output raw JSON
        #[arg(long)]
        json: bool,
    },

    /// Plan or apply install for one child skill projection bundle
    Install {
        /// Child name, e.g. fixture-skill-app
        child: String,

        /// Check global/user HITL scope instead of project/workspace scope
        #[arg(long)]
        global: bool,

        /// Preview actions without writing projection files
        #[arg(long)]
        dry_run: bool,

        /// Apply force-required conflict actions after review
        #[arg(long)]
        force: bool,

        /// Output raw JSON
        #[arg(long)]
        json: bool,
    },

    /// Plan or apply uninstall for one child skill projection bundle
    Uninstall {
        /// Child name, e.g. fixture-skill-app
        child: String,

        /// Check global/user HITL scope instead of project/workspace scope
        #[arg(long)]
        global: bool,

        /// Preview actions without writing projection files
        #[arg(long)]
        dry_run: bool,

        /// Apply force-required conflict actions after review
        #[arg(long)]
        force: bool,

        /// Output raw JSON
        #[arg(long)]
        json: bool,
    },

    /// Show skill packages for one child
    Show {
        /// Child name, e.g. slate-manager
        child: String,
    },

    /// Print help for a child skill package
    Help {
        /// Child name, e.g. slate-manager
        child: String,
        /// Skill package name, e.g. slate-code
        skill: String,
    },

    /// Create and manage isolated Mother skill sandboxes for MCT harness development
    #[command(subcommand)]
    Sandbox(skills_sandbox::SandboxCommands),
}

#[derive(Debug, Clone, clap::Subcommand)]
pub enum ViewCommands {
    /// List available Mother view shapes
    Shapes {
        /// Output raw JSON
        #[arg(long)]
        json: bool,
    },

    /// List live/stale/blocked Mother view buffers
    Buffers {
        /// Output raw JSON
        #[arg(long)]
        json: bool,
    },

    /// Open a Mother-owned view buffer from a shape or friendly target
    Open {
        /// Friendly target (`README.md`, `readme`) or shape id
        target: String,

        /// Open the rendered buffer in a new tmux pane instead of current stdout
        #[arg(long)]
        tmux: bool,

        /// With --tmux, split below (Doom/vi-style `s`)
        #[arg(short = 's', long, conflicts_with = "right")]
        below: bool,

        /// With --tmux, split right (Doom/vi-style `v`)
        #[arg(short = 'v', long, conflicts_with = "below")]
        right: bool,

        /// Output raw JSON
        #[arg(long, conflicts_with = "tmux")]
        json: bool,
    },

    /// Fetch an opened buffer payload
    Payload {
        /// View buffer id
        buffer_id: String,

        /// Output raw JSON
        #[arg(long)]
        json: bool,
    },
}

#[derive(Debug, Clone, clap::Subcommand)]
pub enum ToysCommands {
    /// Show toy registry status
    Status,

    /// List toy registry entries with optional governance filters
    List {
        /// Filter by governance state (local|community-experimental|candidate|approved|deprecated|retired)
        #[arg(long)]
        state: Option<String>,

        /// Filter by tier (wasi-preview2|patina)
        #[arg(long)]
        tier: Option<String>,
    },

    /// Check local toy WIT files against pinned versions
    Check,

    /// Sync Preview 2 pin against WASI monorepo releases
    Sync,

    /// Pull pinned WIT for one toy or Preview 2 as a unit
    Pull {
        /// Toy id from registry (for example: wasi-http)
        name: Option<String>,

        /// Pull all upstream toys at pinned versions (legacy alias)
        #[arg(long)]
        all: bool,

        /// Pull WASI Preview 2 proposals as a unit
        #[arg(long)]
        preview2: bool,
    },
}

#[derive(Debug, Clone, clap::Subcommand)]
pub enum FederationCommands {
    /// Show federation availability and attached project state
    Status,
    /// Re-scan registry and refresh federation attach state
    Refresh,
    /// Execute a read-only federation SQL query
    Query {
        /// SQL query string (SELECT-only)
        sql: String,
        /// Optional row limit (default 1000, max 10000)
        #[arg(long)]
        limit: Option<usize>,
        /// Optional timeout in milliseconds (default 30000)
        #[arg(long)]
        timeout_ms: Option<u64>,
    },
    /// Install required federation extensions into DuckDB
    InstallExtensions,
}

#[derive(Debug, Clone, clap::Subcommand)]
pub enum ChildrenCommands {
    /// Child registry source operations
    Sources {
        /// Source operation (defaults to list)
        #[command(subcommand)]
        command: Option<ChildrenSourcesCommands>,

        /// Output as JSON (list/add operations)
        #[arg(long)]
        json: bool,
    },

    /// Sync child registry entries from provider sources
    Sync {
        /// Sync a specific source id (defaults to all configured sources)
        #[arg(long)]
        source: Option<String>,

        /// Output as JSON
        #[arg(long)]
        json: bool,
    },

    /// Show details for one entry (`entry_id` or `child@version`)
    Show {
        target: String,

        /// Output as JSON
        #[arg(long)]
        json: bool,
    },

    /// Search/list registry entries
    Search {
        /// Filter by child name
        #[arg(long)]
        child: Option<String>,

        /// Filter by state
        #[arg(long)]
        state: Option<String>,

        /// Filter by source id
        #[arg(long)]
        source: Option<String>,

        /// Output as JSON
        #[arg(long)]
        json: bool,
    },

    /// Approve an entry (`entry_id` or `child@version`)
    Approve {
        target: String,

        /// Reason for transition
        #[arg(long)]
        reason: Option<String>,

        /// Allow explicit override for deprecated -> approved
        #[arg(long)]
        force: bool,

        /// Output as JSON
        #[arg(long)]
        json: bool,
    },

    /// Block an entry (`entry_id` or `child@version`)
    Block {
        target: String,

        /// Reason for transition
        #[arg(long)]
        reason: Option<String>,

        /// Output as JSON
        #[arg(long)]
        json: bool,
    },

    /// Deprecate an entry (`entry_id` or `child@version`)
    Deprecate {
        target: String,

        /// Reason for transition
        #[arg(long)]
        reason: Option<String>,

        /// Output as JSON
        #[arg(long)]
        json: bool,
    },

    /// Install an approved entry (`entry_id` or `child@version`)
    Install {
        target: String,

        /// Installer identity for provenance (defaults to local user)
        #[arg(long)]
        installed_by: Option<String>,

        /// Output as JSON
        #[arg(long)]
        json: bool,
    },

    /// Assign approved entry to a project (`entry_id` or `child@version`)
    Assign {
        target: String,

        /// Project UID or project path
        #[arg(long)]
        project: String,

        /// Assignment reason
        #[arg(long)]
        reason: Option<String>,

        /// Output as JSON
        #[arg(long)]
        json: bool,
    },

    /// Revoke active assignment for a project child
    Unassign {
        /// Child name to revoke
        #[arg(long)]
        child: String,

        /// Project UID or project path
        #[arg(long)]
        project: String,

        /// Revoke reason
        #[arg(long)]
        reason: Option<String>,

        /// Output as JSON
        #[arg(long)]
        json: bool,
    },

    /// Show control-plane status
    Status {
        /// Optional project UID/path filter
        #[arg(long)]
        project: Option<String>,

        /// Output as JSON
        #[arg(long)]
        json: bool,
    },
}

#[derive(Debug, Clone, clap::Subcommand)]
pub enum ChildrenSourcesCommands {
    /// Add a child registry source
    Add {
        #[command(subcommand)]
        provider: ChildrenSourceProviderCommands,
    },

    /// Disable a child registry source
    Disable {
        /// Source id to disable
        source_id: String,
    },

    /// Enable a child registry source
    Enable {
        /// Source id to enable
        source_id: String,
    },
}

#[derive(Debug, Clone, clap::Subcommand)]
pub enum ChildrenSourceProviderCommands {
    /// Add a GitHub source using owner/repo
    Github {
        /// Repository in owner/repo format
        repo: String,

        /// Override generated source id (default: src_github_<owner>_<repo>)
        #[arg(long)]
        source_id: Option<String>,

        /// Optional canonical child name hint
        #[arg(long)]
        child_name: Option<String>,

        /// Add as disabled source
        #[arg(long)]
        disabled: bool,
    },
}

#[derive(Debug, Clone, clap::Subcommand)]
pub enum LifecycleCommands {
    /// Load a pando composition
    LoadPando {
        /// Pando name
        name: String,
    },
    /// Refresh all pando compositions
    Refresh,
    /// Reload a single child by canonical name
    ReloadChild {
        /// Child name
        name: String,
    },
    /// Warm up children explicitly (primarily used with `--profile core`)
    WarmupChildren,
    /// Write/refresh SHA-256 sidecars for strict integrity mode
    SyncHashes {
        /// Optional pando name (defaults to all installed pandos)
        #[arg(long)]
        pando: Option<String>,
    },
}

#[derive(Debug, Clone, clap::Subcommand)]
pub enum ProjectsCommands {
    /// List projects Mother knows on this node (registry + identity bindings)
    List,

    /// Check in a single project (current directory by default)
    CheckIn {
        /// Project path or any sub-path inside a Patina project
        path: Option<String>,
    },

    /// Discover and check in Patina projects under a root directory
    Sync {
        /// Root directory to scan (default: ~/Projects)
        #[arg(long)]
        root: Option<String>,

        /// Maximum traversal depth from root
        #[arg(long, default_value_t = 6)]
        max_depth: usize,
    },

    /// Prune stale ephemeral project registrations and artifacts.
    ///
    /// Ephemeral projects are temp paths under /tmp or /private/var/folders.
    /// Missing ephemeral paths are pruned immediately; existing ephemeral paths
    /// are pruned when last check-in is older than the TTL.
    Prune {
        /// Ephemeral TTL in days (existing temp paths older than this are pruned)
        #[arg(long, default_value_t = 3)]
        ephemeral_ttl_days: i64,

        /// Show what would be deleted without deleting
        #[arg(long)]
        dry_run: bool,
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
pub fn execute_cli(command: Option<MotherCommands>, interface: Option<String>) -> Result<()> {
    match command {
        None => {
            // Bare `patina mother` — show status (or help for now)
            println!("Mother daemon commands:\n");
            println!("  patina mother start    Start the daemon");
            println!("  patina mother stop     Stop the daemon");
            println!("  patina mother restart  Restart via supervisor when configured");
            println!("  patina mother status   Show daemon status");
            println!("  patina mother install  Install system supervisor");
            println!("  patina mother uninstall Remove system supervisor");
            println!("  patina mother graph    Graph operations");
            println!("  patina mother toys     Toy registry operations");
            println!("  patina mother federation Federation query surface");
            println!("  patina mother children Child registry control-plane");
            println!("  patina mother lifecycle Lifecycle operations");
            println!("  patina mother projects Project check-in/list/sync");
            println!("  patina mother skills   Discover child skill packages");
            println!("  patina mother view     Mother-owned view buffers");
            println!("  patina mother search   Cross-project belief search\n");
            println!("Run 'patina mother --help' for details.");
            Ok(())
        }
        Some(MotherCommands::Start {
            host,
            port,
            profile,
            rivet,
            mcp,
        }) => {
            if mcp {
                bail!("MCP server path has been retired; start daemon without --mcp")
            } else {
                enforce_start_conflict_guard()?;
                let options = DaemonOptions {
                    host,
                    port,
                    profile,
                    rivet,
                };
                daemon::run_server(options)
            }
        }
        Some(MotherCommands::Stop) => stop_daemon(),
        Some(MotherCommands::Restart) => restart_daemon(),
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
        Some(MotherCommands::Toys(ToysCommands::List { state, tier })) => {
            let project_root = SessionManager::find_project_root()
                .context("`patina mother toys list` must run in a Patina project")?;
            toys::toys_list(&project_root, state.as_deref(), tier.as_deref())
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
        Some(MotherCommands::Toys(ToysCommands::Pull {
            name,
            all,
            preview2,
        })) => {
            let project_root = SessionManager::find_project_root()
                .context("`patina mother toys pull` must run in a Patina project")?;
            if preview2 || all {
                toys::toys_pull_preview2(&project_root)
            } else {
                let name = name.ok_or_else(|| {
                    anyhow::anyhow!(
                        "`patina mother toys pull` needs a toy name, or use `--preview2`"
                    )
                })?;
                toys::toys_pull(&project_root, &name)
            }
        }
        Some(MotherCommands::Federation(command)) => execute_federation(command),
        Some(MotherCommands::Children(command)) => children::execute_children(command),
        Some(MotherCommands::Lifecycle(command)) => execute_lifecycle(command),
        Some(MotherCommands::Projects(command)) => execute_projects(command),
        Some(MotherCommands::Skills(command)) => execute_skills(command, interface.as_deref()),
        Some(MotherCommands::View(command)) => execute_view(command),
    }
}

fn execute_skills(command: SkillsCommands, interface: Option<&str>) -> Result<()> {
    match command {
        SkillsCommands::List => {
            let rows = installed_child_skill_rows()?;
            if rows.is_empty() {
                println!("No installed child skill packages found.");
                return Ok(());
            }
            println!("Mother child skills:\n");
            for (child, skills) in rows {
                if skills.is_empty() {
                    println!("  {}  —", child);
                } else {
                    println!("  {}  {}", child, skills.join(", "));
                }
            }
            Ok(())
        }
        SkillsCommands::Status {
            child,
            global,
            json,
        } => skills_lifecycle::status(child.as_deref(), interface, global, json),
        SkillsCommands::Sync {
            child,
            global,
            dry_run,
            force,
            json,
        } => skills_lifecycle::sync(child.as_deref(), interface, global, dry_run, force, json),
        SkillsCommands::Install {
            child,
            global,
            dry_run,
            force,
            json,
        } => skills_lifecycle::install(&child, interface, global, dry_run, force, json),
        SkillsCommands::Uninstall {
            child,
            global,
            dry_run,
            force,
            json,
        } => skills_lifecycle::uninstall(&child, interface, global, dry_run, force, json),
        SkillsCommands::Show { child } => {
            let skills = child_skills(&child)?;
            if skills.is_empty() {
                println!("No skills found for child '{}'.", child);
                return Ok(());
            }
            println!("{} skills:\n", child);
            for skill in skills {
                println!("  {:<28} {}", skill.name, skill.description);
            }
            Ok(())
        }
        SkillsCommands::Help { child, skill } => {
            let path = child_skill_path(&child, &skill);
            if !path.exists() {
                anyhow::bail!(
                    "skill '{}' for child '{}' not found at {}",
                    skill,
                    child,
                    path.display()
                );
            }
            println!("{}", std::fs::read_to_string(path)?);
            Ok(())
        }
        SkillsCommands::Sandbox(command) => skills_sandbox::execute(command),
    }
}

#[derive(Debug, Clone)]
struct ChildSkillSummary {
    name: String,
    description: String,
}

fn installed_child_skill_rows() -> Result<Vec<(String, Vec<String>)>> {
    let mut rows = Vec::new();
    let dir = paths::child::command_children_dir();
    if !dir.exists() {
        return Ok(rows);
    }
    for entry in std::fs::read_dir(&dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.extension().and_then(|ext| ext.to_str()) != Some("toml") {
            continue;
        }
        let Some(stem) = path.file_stem().and_then(|stem| stem.to_str()) else {
            continue;
        };
        let skill_names = child_skills(stem)?
            .into_iter()
            .map(|skill| skill.name)
            .collect::<Vec<_>>();
        rows.push((stem.to_string(), skill_names));
    }
    rows.sort_by(|a, b| a.0.cmp(&b.0));
    Ok(rows)
}

fn child_skills(child: &str) -> Result<Vec<ChildSkillSummary>> {
    let skills_dir = paths::child::command_children_dir()
        .join(child)
        .join("skills");
    if !skills_dir.exists() {
        return Ok(Vec::new());
    }
    let mut skills = Vec::new();
    for entry in std::fs::read_dir(skills_dir)? {
        let entry = entry?;
        if !entry.file_type()?.is_dir() {
            continue;
        }
        let path = entry.path().join("SKILL.md");
        if !path.exists() {
            continue;
        }
        skills.push(read_skill_summary(&path)?);
    }
    skills.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(skills)
}

fn child_skill_path(child: &str, skill: &str) -> PathBuf {
    paths::child::command_children_dir()
        .join(child)
        .join("skills")
        .join(skill)
        .join("SKILL.md")
}

fn read_skill_summary(path: &Path) -> Result<ChildSkillSummary> {
    let raw = std::fs::read_to_string(path)?;
    let mut name = path
        .parent()
        .and_then(|parent| parent.file_name())
        .and_then(|name| name.to_str())
        .unwrap_or("unknown")
        .to_string();
    let mut description = String::new();

    if raw.starts_with("---\n") {
        for line in raw.lines().skip(1) {
            if line.trim() == "---" {
                break;
            }
            if let Some(value) = line.strip_prefix("name:") {
                name = value.trim().trim_matches('"').to_string();
            } else if let Some(value) = line.strip_prefix("description:") {
                description = value.trim().trim_matches('"').to_string();
            }
        }
    }

    Ok(ChildSkillSummary { name, description })
}

fn execute_view(command: ViewCommands) -> Result<()> {
    let client = patina::mother::control_plane_client();
    match command {
        ViewCommands::Shapes { json } => {
            let payload = client.get_json("/api/view-shapes")?;
            if json {
                println!("{}", serde_json::to_string_pretty(&payload)?);
                return Ok(());
            }
            let shapes = payload
                .get("shapes")
                .and_then(|value| value.as_array())
                .cloned()
                .unwrap_or_default();
            if shapes.is_empty() {
                println!("No Mother view shapes registered.");
                return Ok(());
            }
            println!("Mother view shapes:\n");
            for shape in shapes {
                println!(
                    "  {:<48} {:<10} {}",
                    shape
                        .get("shape_id")
                        .and_then(|v| v.as_str())
                        .unwrap_or("-"),
                    shape
                        .get("major_mode")
                        .and_then(|v| v.as_str())
                        .unwrap_or("-"),
                    shape.get("title").and_then(|v| v.as_str()).unwrap_or("-")
                );
            }
            Ok(())
        }
        ViewCommands::Buffers { json } => {
            let payload = client.get_json("/api/view-buffers")?;
            if json {
                println!("{}", serde_json::to_string_pretty(&payload)?);
                return Ok(());
            }
            let buffers = payload
                .get("buffers")
                .and_then(|value| value.as_array())
                .cloned()
                .unwrap_or_default();
            if buffers.is_empty() {
                println!("No Mother view buffers are open.");
                return Ok(());
            }
            println!("Mother view buffers:\n");
            for buffer in buffers {
                println!(
                    "  {:<52} {:<10} {:<10} {}",
                    buffer
                        .get("buffer_id")
                        .and_then(|v| v.as_str())
                        .unwrap_or("-"),
                    buffer.get("state").and_then(|v| v.as_str()).unwrap_or("-"),
                    buffer
                        .get("major_mode")
                        .and_then(|v| v.as_str())
                        .unwrap_or("-"),
                    buffer.get("name").and_then(|v| v.as_str()).unwrap_or("-")
                );
            }
            Ok(())
        }
        ViewCommands::Open {
            target,
            tmux,
            below,
            right,
            json,
        } => {
            let shape_id = resolve_view_target(&target);
            let payload = client.post_json(
                "/api/view-buffers/open",
                &serde_json::json!({ "shape_id": shape_id }),
            )?;
            if json {
                println!("{}", serde_json::to_string_pretty(&payload)?);
                return Ok(());
            }
            if tmux || below || right {
                let buffer_id = payload
                    .get("buffer")
                    .and_then(|buffer| buffer.get("buffer_id"))
                    .and_then(|value| value.as_str())
                    .ok_or_else(|| anyhow::anyhow!("Mother open response missing buffer_id"))?;
                let split = if below {
                    TmuxSplit::Below
                } else {
                    TmuxSplit::Right
                };
                return open_view_in_tmux(buffer_id, split);
            }
            print_opened_view_payload(&payload)
        }
        ViewCommands::Payload { buffer_id, json } => {
            let payload = client.get_json(&format!("/api/view-buffers/{}/payload", buffer_id))?;
            if json {
                println!("{}", serde_json::to_string_pretty(&payload)?);
                return Ok(());
            }
            let opened = payload
                .get("opened")
                .ok_or_else(|| anyhow::anyhow!("Mother payload response missing `opened`"))?;
            print_opened_view_payload(opened)
        }
    }
}

fn resolve_view_target(target: &str) -> String {
    let normalized = target.trim().to_ascii_lowercase();
    if normalized == "readme" || normalized == "readme.md" || normalized.ends_with("/readme.md") {
        mother_crate::PROJECT_README_SHAPE_ID.to_string()
    } else {
        target.to_string()
    }
}

#[derive(Debug, Clone, Copy)]
enum TmuxSplit {
    Right,
    Below,
}

fn open_view_in_tmux(buffer_id: &str, split: TmuxSplit) -> Result<()> {
    if std::env::var_os("TMUX").is_none() {
        anyhow::bail!("--tmux requires running inside tmux");
    }

    let cwd = std::env::current_dir().context("resolve current directory for tmux pane")?;
    let command = format!(
        "cd {} && patina mother view payload {} | less -R -X",
        shell_quote(&cwd.to_string_lossy()),
        shell_quote(buffer_id)
    );

    let mut tmux = Command::new("tmux");
    tmux.arg("split-window");
    match split {
        TmuxSplit::Right => {
            tmux.arg("-h").arg("-l").arg("50%");
        }
        TmuxSplit::Below => {
            tmux.arg("-v").arg("-l").arg("40%");
        }
    }
    tmux.arg(command);

    let status = tmux.status().context("run tmux split-window")?;
    if !status.success() {
        anyhow::bail!("tmux split-window failed with status {}", status);
    }

    println!(
        "Opened Mother view buffer {} in tmux {} pane",
        buffer_id,
        match split {
            TmuxSplit::Right => "right",
            TmuxSplit::Below => "below",
        }
    );
    Ok(())
}

fn shell_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\\''"))
}

fn print_opened_view_payload(payload: &serde_json::Value) -> Result<()> {
    let buffer = payload
        .get("buffer")
        .ok_or_else(|| anyhow::anyhow!("Mother open response missing `buffer`"))?;
    let framed_payload = payload
        .get("payload")
        .ok_or_else(|| anyhow::anyhow!("Mother open response missing `payload`"))?;
    let payload_json = framed_payload
        .get("json")
        .ok_or_else(|| anyhow::anyhow!("Mother open response missing `payload.json`"))?;

    let name = buffer.get("name").and_then(|v| v.as_str()).unwrap_or("-");
    let buffer_id = buffer
        .get("buffer_id")
        .and_then(|v| v.as_str())
        .unwrap_or("-");
    let mode = buffer
        .get("major_mode")
        .and_then(|v| v.as_str())
        .unwrap_or("-");
    let shape_id = buffer
        .get("shape_id")
        .and_then(|v| v.as_str())
        .unwrap_or("-");

    println!("Opened Mother view buffer: {}", name);
    println!("  buffer_id: {}", buffer_id);
    println!("  shape_id:  {}", shape_id);
    println!("  mode:      {}", mode);

    if mode == "markdown" {
        if let Some(path) = payload_json.get("path").and_then(|v| v.as_str()) {
            println!("  path:      {}", path);
        }
        if let Some(status) = payload_json.get("git_status").and_then(|v| v.as_str()) {
            println!("  git:       {}", status);
        }
        if let Some(content) = payload_json.get("content").and_then(|v| v.as_str()) {
            println!("\n--- {} ---\n{}", name, content);
            return Ok(());
        }
    }

    println!("\n{}", serde_json::to_string_pretty(payload_json)?);
    Ok(())
}

fn execute_federation(command: FederationCommands) -> Result<()> {
    let client = patina::mother::control_plane_client();
    match command {
        FederationCommands::Status => {
            let payload = client.federation_status()?;
            println!("{}", serde_json::to_string_pretty(&payload)?);
            Ok(())
        }
        FederationCommands::Refresh => {
            let payload = client.federation_refresh()?;
            println!("{}", serde_json::to_string_pretty(&payload)?);
            Ok(())
        }
        FederationCommands::Query {
            sql,
            limit,
            timeout_ms,
        } => {
            let payload =
                client.federation_query(mother_crate::protocol::FederationQueryPayload {
                    sql,
                    params: vec![],
                    limit,
                    timeout_ms,
                })?;
            println!("{}", serde_json::to_string_pretty(&payload)?);
            Ok(())
        }
        FederationCommands::InstallExtensions => {
            federation::install_extensions()?;
            println!("Installed DuckLake extension for Mother federation.");
            Ok(())
        }
    }
}

fn execute_lifecycle(command: LifecycleCommands) -> Result<()> {
    let client = patina::mother::control_plane_client();
    match command {
        LifecycleCommands::LoadPando { name } => {
            let payload = client
                .lifecycle_load_pando(mother_crate::protocol::LifecycleNamePayload { name })?;
            println!("{}", serde_json::to_string_pretty(&payload)?);
            Ok(())
        }
        LifecycleCommands::Refresh => {
            let payload =
                client.lifecycle_refresh(mother_crate::protocol::LifecycleRefreshPayload {})?;
            println!("{}", serde_json::to_string_pretty(&payload)?);
            Ok(())
        }
        LifecycleCommands::ReloadChild { name } => {
            let payload = client
                .lifecycle_reload_child(mother_crate::protocol::LifecycleNamePayload { name })?;
            println!("{}", serde_json::to_string_pretty(&payload)?);
            Ok(())
        }
        LifecycleCommands::WarmupChildren => {
            let payload = client
                .lifecycle_warmup_children(mother_crate::protocol::LifecycleWarmupPayload {})?;
            println!("{}", serde_json::to_string_pretty(&payload)?);
            Ok(())
        }
        LifecycleCommands::SyncHashes { pando } => {
            let root = patina::paths::pando::pandos_dir();
            if let Some(name) = pando {
                let pando_dir = root.join(&name);
                let written = integrity::write_pando_hashes(&pando_dir)?;
                println!(
                    "Refreshed {} SHA-256 sidecars for pando '{}'.",
                    written, name
                );
                return Ok(());
            }

            let (pandos, files) = integrity::write_all_pando_hashes(&root)?;
            println!(
                "Refreshed {} SHA-256 sidecars across {} pando(s).",
                files, pandos
            );
            Ok(())
        }
    }
}

#[cfg(target_os = "macos")]
const MOTHER_LAUNCHD_LABEL: &str = "com.patina.mother";
#[cfg(target_os = "macos")]
const MOTHER_HOMEBREW_LAUNCHD_LABEL: &str = "homebrew.mxcl.patina";
const MOTHER_SUPERVISED_ENV: &str = "PATINA_SUPERVISED";

#[cfg(target_os = "macos")]
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
fn launchd_homebrew_plist_path() -> Result<std::path::PathBuf> {
    Ok(launch_agents_dir()?.join(format!("{}.plist", MOTHER_HOMEBREW_LAUNCHD_LABEL)))
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
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<!DOCTYPE plist PUBLIC \"-//Apple//DTD PLIST 1.0//EN\" \"http://www.apple.com/DTDs/PropertyList-1.0.dtd\">\n<plist version=\"1.0\">\n<dict>\n  <key>Label</key>\n  <string>{}</string>\n  <key>ProgramArguments</key>\n  <array>\n    <string>{}</string>\n    <string>mother</string>\n    <string>start</string>\n  </array>\n  <key>EnvironmentVariables</key>\n  <dict>\n    <key>{}</key>\n    <string>1</string>\n  </dict>\n  <key>RunAtLoad</key>\n  <true/>\n  <key>KeepAlive</key>\n  <true/>\n</dict>\n</plist>\n",
        MOTHER_LAUNCHD_LABEL, exe, MOTHER_SUPERVISED_ENV
    )
}

#[cfg(target_os = "macos")]
fn launchctl_domains() -> Vec<String> {
    let uid = unsafe { libc::geteuid() };
    vec![format!("gui/{uid}"), format!("user/{uid}")]
}

#[cfg(target_os = "linux")]
const MOTHER_SYSTEMD_UNIT: &str = "patina-mother.service";
#[cfg(target_os = "linux")]
const MOTHER_SYSTEMCTL_BIN_ENV: &str = "PATINA_SYSTEMCTL_BIN";

#[cfg(target_os = "linux")]
fn systemd_user_dir() -> Result<std::path::PathBuf> {
    let home = dirs::home_dir().context("unable to resolve home directory")?;
    Ok(home.join(".config/systemd/user"))
}

#[cfg(target_os = "linux")]
fn systemd_unit_path() -> Result<std::path::PathBuf> {
    Ok(systemd_user_dir()?.join(MOTHER_SYSTEMD_UNIT))
}

#[cfg(target_os = "linux")]
fn render_systemd_unit(exe_path: &Path) -> String {
    format!(
        "[Unit]\nDescription=Patina Mother daemon\nAfter=default.target\n\n[Service]\nType=simple\nEnvironment={}=1\nExecStart={} mother start\nRestart=always\nRestartSec=2\n\n[Install]\nWantedBy=default.target\n",
        MOTHER_SUPERVISED_ENV,
        exe_path.display()
    )
}

#[cfg(target_os = "linux")]
fn run_systemctl_user(args: &[&str]) -> Result<()> {
    let bin = std::env::var_os(MOTHER_SYSTEMCTL_BIN_ENV)
        .unwrap_or_else(|| std::ffi::OsString::from("systemctl"));
    let status = Command::new(&bin)
        .arg("--user")
        .args(args)
        .status()
        .with_context(|| {
            format!(
                "running {} --user {}",
                std::path::Path::new(&bin).display(),
                args.join(" ")
            )
        })?;
    if !status.success() {
        bail!(
            "systemctl --user {} failed with status {}",
            args.join(" "),
            status
        );
    }
    Ok(())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SupervisorBackend {
    Manual,
    LaunchdPatina,
    LaunchdHomebrew,
    SystemdUser,
}

impl SupervisorBackend {
    fn label(self) -> &'static str {
        match self {
            SupervisorBackend::Manual => "manual",
            SupervisorBackend::LaunchdPatina => "launchd (patina mother install)",
            SupervisorBackend::LaunchdHomebrew => "launchd (homebrew services)",
            SupervisorBackend::SystemdUser => "systemd --user",
        }
    }
}

fn classify_supervisor_backend(
    launchd_patina_present: bool,
    launchd_homebrew_present: bool,
    systemd_user_present: bool,
) -> SupervisorBackend {
    if launchd_homebrew_present {
        SupervisorBackend::LaunchdHomebrew
    } else if launchd_patina_present {
        SupervisorBackend::LaunchdPatina
    } else if systemd_user_present {
        SupervisorBackend::SystemdUser
    } else {
        SupervisorBackend::Manual
    }
}

fn detect_supervisor_backend() -> SupervisorBackend {
    let launchd_patina_present = {
        #[cfg(target_os = "macos")]
        {
            launchd_plist_path()
                .map(|path| path.exists())
                .unwrap_or(false)
        }
        #[cfg(not(target_os = "macos"))]
        {
            false
        }
    };

    let launchd_homebrew_present = {
        #[cfg(target_os = "macos")]
        {
            launchd_homebrew_plist_path()
                .map(|path| path.exists())
                .unwrap_or(false)
        }
        #[cfg(not(target_os = "macos"))]
        {
            false
        }
    };

    let systemd_user_present = {
        #[cfg(target_os = "linux")]
        {
            systemd_unit_path()
                .map(|path| path.exists())
                .unwrap_or(false)
        }
        #[cfg(not(target_os = "linux"))]
        {
            false
        }
    };

    classify_supervisor_backend(
        launchd_patina_present,
        launchd_homebrew_present,
        systemd_user_present,
    )
}

fn manual_start_block_reason(
    supervisor: SupervisorBackend,
    supervised_invocation: bool,
) -> Option<&'static str> {
    if supervised_invocation {
        return None;
    }

    match supervisor {
        SupervisorBackend::LaunchdPatina => Some(
            "Mother is managed by launchd (patina mother install). Use `patina mother restart` or `patina mother uninstall` before manual start.",
        ),
        SupervisorBackend::SystemdUser => Some(
            "Mother is managed by systemd --user. Use `patina mother restart` or `patina mother uninstall` before manual start.",
        ),
        SupervisorBackend::Manual | SupervisorBackend::LaunchdHomebrew => None,
    }
}

fn enforce_start_conflict_guard() -> Result<()> {
    let supervisor = detect_supervisor_backend();
    let supervised_invocation = std::env::var_os(MOTHER_SUPERVISED_ENV).is_some();

    if let Some(reason) = manual_start_block_reason(supervisor, supervised_invocation) {
        bail!(reason);
    }

    if supervisor == SupervisorBackend::LaunchdHomebrew
        && !supervised_invocation
        && std::io::stdin().is_terminal()
    {
        eprintln!(
            "Warning: Homebrew launchd service is installed; manual start may conflict. Prefer `brew services restart patina`."
        );
    }

    Ok(())
}

#[cfg(target_os = "macos")]
fn launchd_service_loaded(domain: &str, label: &str) -> bool {
    let service = format!("{}/{}", domain, label);
    Command::new("launchctl")
        .arg("print")
        .arg(&service)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
}

#[cfg(target_os = "macos")]
fn wait_for_launchd_service_absent(label: &str, timeout: std::time::Duration) -> Result<()> {
    let domains = launchctl_domains();
    let deadline = std::time::Instant::now() + timeout;

    loop {
        if !domains
            .iter()
            .any(|domain| launchd_service_loaded(domain, label))
        {
            return Ok(());
        }
        if std::time::Instant::now() >= deadline {
            bail!(
                "launchd service '{}' did not finish bootout within {}s",
                label,
                timeout.as_secs()
            );
        }
        std::thread::sleep(std::time::Duration::from_millis(100));
    }
}

#[cfg(target_os = "macos")]
fn restart_launchd_label(label: &str) -> Result<()> {
    let mut last_error = None;
    for domain in launchctl_domains() {
        let service = format!("{}/{}", domain, label);
        match run_launchctl(&["kickstart", "-k", &service]) {
            Ok(()) => return Ok(()),
            Err(error) => last_error = Some(error),
        }
    }

    Err(last_error.unwrap_or_else(|| anyhow::anyhow!("launchctl kickstart failed")))
}

fn manual_restart_log_path() -> PathBuf {
    paths::patina_home()
        .join("mother")
        .join("logs")
        .join("manual-restart.log")
}

#[cfg(unix)]
fn spawn_manual_daemon_detached() -> Result<PathBuf> {
    let log_path = manual_restart_log_path();
    if let Some(parent) = log_path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("creating {}", parent.display()))?;
    }

    let stdout = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_path)
        .with_context(|| format!("opening {}", log_path.display()))?;
    let stderr = stdout
        .try_clone()
        .with_context(|| format!("cloning {}", log_path.display()))?;

    let exe = std::env::current_exe().context("resolving current executable")?;
    Command::new(exe)
        .arg("mother")
        .arg("start")
        .stdin(Stdio::null())
        .stdout(Stdio::from(stdout))
        .stderr(Stdio::from(stderr))
        .spawn()
        .context("starting Mother in background")?;

    Ok(log_path)
}

fn wait_for_daemon_ready(
    timeout: std::time::Duration,
) -> Result<mother_crate::lifecycle::StatusReport> {
    let pid_path = paths::serve::pid_path();
    let socket_path = paths::serve::socket_path();
    let deadline = std::time::Instant::now() + timeout;
    let mut last_health_error = None;

    loop {
        let status = mother_crate::lifecycle::probe_status(&pid_path, &socket_path)?;
        if status.running && status.health.is_some() {
            return Ok(status);
        }
        if let Some(error) = status.health_error {
            last_health_error = Some(error);
        }
        if std::time::Instant::now() >= deadline {
            let detail = last_health_error
                .map(|error| format!(" last health error: {error}"))
                .unwrap_or_default();
            bail!(
                "Mother did not become ready within {}s.{}",
                timeout.as_secs(),
                detail
            );
        }
        std::thread::sleep(std::time::Duration::from_millis(200));
    }
}

fn restart_manual_daemon_detached() -> Result<()> {
    stop_daemon()?;
    println!("Restarting Mother in background (manual backend)...");

    #[cfg(unix)]
    {
        let log_path = spawn_manual_daemon_detached()?;
        let status = wait_for_daemon_ready(std::time::Duration::from_secs(15))?;
        println!("Mother daemon restarted.");
        if let Some(pid) = status.pid {
            println!("   PID: {}", pid);
        }
        println!("   Socket: {}", paths::serve::socket_path().display());
        println!("   Logs: {}", log_path.display());
        Ok(())
    }

    #[cfg(not(unix))]
    {
        bail!("manual background restart is unsupported on this platform")
    }
}

fn restart_daemon() -> Result<()> {
    match detect_supervisor_backend() {
        SupervisorBackend::LaunchdPatina => {
            #[cfg(target_os = "macos")]
            {
                restart_launchd_label(MOTHER_LAUNCHD_LABEL)?;
                println!("Requested restart via launchd: {}", MOTHER_LAUNCHD_LABEL);
                Ok(())
            }
            #[cfg(not(target_os = "macos"))]
            {
                bail!("launchd supervisor is unavailable on this platform");
            }
        }
        SupervisorBackend::LaunchdHomebrew => {
            #[cfg(target_os = "macos")]
            {
                restart_launchd_label(MOTHER_HOMEBREW_LAUNCHD_LABEL)?;
                println!(
                    "Requested restart via launchd: {}",
                    MOTHER_HOMEBREW_LAUNCHD_LABEL
                );
                Ok(())
            }
            #[cfg(not(target_os = "macos"))]
            {
                bail!("launchd supervisor is unavailable on this platform");
            }
        }
        SupervisorBackend::SystemdUser => {
            #[cfg(target_os = "linux")]
            {
                run_systemctl_user(&["restart", MOTHER_SYSTEMD_UNIT])?;
                println!(
                    "Requested restart via systemd --user: {}",
                    MOTHER_SYSTEMD_UNIT
                );
                Ok(())
            }
            #[cfg(not(target_os = "linux"))]
            {
                bail!("systemd --user supervisor is unavailable on this platform");
            }
        }
        SupervisorBackend::Manual => restart_manual_daemon_detached(),
    }
}

fn install_supervisor() -> Result<()> {
    #[cfg(target_os = "macos")]
    {
        let supervisor = detect_supervisor_backend();
        if supervisor == SupervisorBackend::LaunchdHomebrew {
            bail!(
                "Homebrew manages Mother via launchd. Use `brew services restart patina` (or `brew services stop patina`) before `patina mother install`."
            );
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
        wait_for_launchd_service_absent(MOTHER_LAUNCHD_LABEL, std::time::Duration::from_secs(5))?;
        let plist_arg = plist_path.to_string_lossy().to_string();
        let mut selected_domain = None;
        let mut last_error = None;
        for domain in launchctl_domains() {
            // `patina mother uninstall` disables the service after bootout. On
            // macOS, a disabled user service may fail bootstrap with error 119
            // (surfaced by launchctl as I/O error 5), so clear that marker
            // before attempting to bootstrap the freshly written plist.
            let service = format!("{}/{}", domain, MOTHER_LAUNCHD_LABEL);
            let _ = run_launchctl(&["enable", &service]);

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

    #[cfg(target_os = "linux")]
    {
        let unit_path = systemd_unit_path()?;
        let parent = unit_path
            .parent()
            .context("invalid systemd user unit parent path")?;
        std::fs::create_dir_all(parent)
            .with_context(|| format!("creating {}", parent.display()))?;

        let exe_path = std::env::current_exe().context("resolving current executable")?;
        let unit = render_systemd_unit(&exe_path);
        std::fs::write(&unit_path, unit)
            .with_context(|| format!("writing {}", unit_path.display()))?;

        run_systemctl_user(&["daemon-reload"])?;
        run_systemctl_user(&["enable", "--now", MOTHER_SYSTEMD_UNIT])?;

        println!("Installed systemd user unit: {}", unit_path.display());
        println!("Unit name: {}", MOTHER_SYSTEMD_UNIT);
        Ok(())
    }

    #[cfg(not(any(target_os = "macos", target_os = "linux")))]
    {
        bail!("patina mother install is unsupported on this platform")
    }
}

fn uninstall_supervisor() -> Result<()> {
    #[cfg(target_os = "macos")]
    {
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

    #[cfg(target_os = "linux")]
    {
        let unit_path = systemd_unit_path()?;
        let _ = run_systemctl_user(&["disable", "--now", MOTHER_SYSTEMD_UNIT]);

        if unit_path.exists() {
            std::fs::remove_file(&unit_path)
                .with_context(|| format!("removing {}", unit_path.display()))?;
            println!("Removed systemd user unit: {}", unit_path.display());
        } else {
            println!("Systemd user unit not found: {}", unit_path.display());
        }

        let _ = run_systemctl_user(&["daemon-reload"]);
        Ok(())
    }

    #[cfg(not(any(target_os = "macos", target_os = "linux")))]
    {
        bail!("patina mother uninstall is unsupported on this platform")
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

#[derive(Debug, Clone)]
struct ProjectStatusContext {
    uid: String,
    events_db_bytes: Option<u64>,
    patina_db_bytes: Option<u64>,
    runtime_db_bytes: Option<u64>,
}

fn file_size_if_exists(path: &Path) -> Option<u64> {
    std::fs::metadata(path).ok().map(|metadata| metadata.len())
}

fn project_status_context_from_current_dir() -> Option<ProjectStatusContext> {
    let project_root = SessionManager::find_project_root().ok()?;
    let uid = patina::project::get_uid(&project_root)?;
    Some(ProjectStatusContext {
        events_db_bytes: paths::mother::projects::events_db(&uid)
            .ok()
            .and_then(|path| file_size_if_exists(&path)),
        patina_db_bytes: paths::mother::projects::patina_db(&uid)
            .ok()
            .and_then(|path| file_size_if_exists(&path)),
        runtime_db_bytes: paths::mother::projects::runtime_db(&uid)
            .ok()
            .and_then(|path| file_size_if_exists(&path)),
        uid,
    })
}

fn print_project_status_context(label: &str, context: &ProjectStatusContext) {
    println!("   {}: {}", label, context.uid);
    if let Some(bytes) = context.events_db_bytes {
        println!("   Project events.db bytes: {}", bytes);
    }
    if let Some(bytes) = context.patina_db_bytes {
        println!("   Project patina.db bytes: {}", bytes);
    }
    if let Some(bytes) = context.runtime_db_bytes {
        println!("   Project runtime.db bytes: {}", bytes);
    }
}

/// Show daemon status
fn show_status() -> Result<()> {
    let pid_path = paths::serve::pid_path();
    let socket_path = paths::serve::socket_path();
    let current_project = project_status_context_from_current_dir();

    let status = mother_crate::lifecycle::probe_status(&pid_path, &socket_path)?;
    let supervisor = detect_supervisor_backend();

    if !status.running {
        println!("Mother daemon: stopped");
        println!("   Supervisor: {}", supervisor.label());
        if status.stale_pid_file {
            println!("   (stale PID file exists — run `patina mother stop` to clean up)");
        }
        if let Some(failure) = status.startup_failure {
            println!(
                "   Last startup failure: stage '{}' at {}",
                failure.stage, failure.updated_at
            );
            if let Some(error_excerpt) = failure.error_excerpt {
                println!("   Error: {}", error_excerpt);
            }
            let log_path = paths::patina_home().join("mother/logs/mother.jsonl");
            println!("   Logs: {}", log_path.display());
        }
        println!("\n   Tip: broker source status lives under `patina mother sources`.");
        return Ok(());
    }

    println!("Mother daemon: running");
    if let Some(pid) = status.pid {
        println!("   PID: {}", pid);
    }
    println!("   Socket: {}", socket_path.display());
    println!("   Supervisor: {}", supervisor.label());

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
            if let Some(profile) = &health.startup_profile {
                println!("   Startup profile: {}", profile);
            }
            if let Some(rivet_integration) = &health.rivet_integration {
                println!("   Rivet integration: {}", rivet_integration);
            }
            if let Some(warmup) = &health.child_warmup {
                if warmup.mode.is_empty() && warmup.state.is_empty() {
                    println!("   Child warmup: unavailable");
                } else {
                    println!(
                        "   Child warmup: mode={} state={}",
                        warmup.mode, warmup.state
                    );
                    if let Some(error) = &warmup.last_error {
                        println!("   Child warmup last error: {}", error);
                    }
                }
            }
            if let Some(memory) = &health.memory {
                let pressure = memory.pressure.as_deref().unwrap_or("unknown");
                println!("   Memory pressure: {}", pressure);
                if let Some(bytes) = memory.rss_bytes {
                    println!("   Memory RSS bytes: {}", bytes);
                }
                if let Some(bytes) = memory.max_rss_bytes {
                    println!("   Memory max RSS bytes: {}", bytes);
                }
                if let Some(bytes) = memory.soft_limit_bytes {
                    println!("   Memory soft limit bytes: {}", bytes);
                }
            }
            println!("   Control plane ready: {}", health.control_plane_ready);
            println!(
                "   Children readiness: {}/{}",
                health.children_ready_count, health.children_total
            );
            if !health.children_degraded.is_empty() {
                println!("   Children degraded:");
                for entry in &health.children_degraded {
                    println!("     {}: {}", entry.name, entry.reason);
                }
            }
            if let Some(state_db_bytes) = health.state_db_bytes {
                println!("   State DB bytes: {}", state_db_bytes);
            }
            if let Some(project) = &current_project {
                print_project_status_context("Project context", project);
            } else if let Some(project_uid) = &health.active_project_uid {
                println!("   Daemon startup project: {}", project_uid);
                if let Some(databases) = &health.active_project_databases {
                    if let Some(bytes) = databases.events_db_bytes {
                        println!("   Project events.db bytes: {}", bytes);
                    }
                    if let Some(bytes) = databases.patina_db_bytes {
                        println!("   Project patina.db bytes: {}", bytes);
                    }
                    if let Some(bytes) = databases.runtime_db_bytes {
                        println!("   Project runtime.db bytes: {}", bytes);
                    }
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

#[derive(Debug)]
struct ProjectIdentityBinding {
    project_uid: String,
    project_id: String,
    user_id: String,
    vision_id: String,
    node_id: String,
    status: String,
}

#[derive(Debug)]
struct ProjectBeliefBinding {
    project_uid: String,
    status: String,
    source_belief_count: Option<i64>,
    source_value_count: Option<i64>,
    indexed_belief_count: Option<i64>,
    indexed_value_count: Option<i64>,
    source_commit_sha: Option<String>,
    last_verified_at: String,
}

fn execute_projects(command: ProjectsCommands) -> Result<()> {
    match command {
        ProjectsCommands::List => list_projects_cli(),
        ProjectsCommands::CheckIn { path } => check_in_project_cli(path.as_deref()),
        ProjectsCommands::Sync { root, max_depth } => sync_projects_cli(root.as_deref(), max_depth),
        ProjectsCommands::Prune {
            ephemeral_ttl_days,
            dry_run,
        } => prune_projects_cli(ephemeral_ttl_days, dry_run),
    }
}

fn list_projects_cli() -> Result<()> {
    let store = patina::mother::MotherRuntimeStore::default();
    let mut projects = store.list_registered_projects()?;
    projects.sort_by(|a, b| a.project_path.cmp(&b.project_path));

    let conn = rusqlite::Connection::open(store.path()).with_context(|| {
        format!(
            "opening mother state db for project identity list: {}",
            store.path().display()
        )
    })?;

    let mut stmt = conn.prepare(
        "SELECT project_uid, project_id, user_id, vision_id, node_id, status
         FROM mother_project_identities",
    )?;
    let rows = stmt.query_map([], |row| {
        Ok(ProjectIdentityBinding {
            project_uid: row.get(0)?,
            project_id: row.get(1)?,
            user_id: row.get(2)?,
            vision_id: row.get(3)?,
            node_id: row.get(4)?,
            status: row.get(5)?,
        })
    })?;

    let mut bindings = std::collections::HashMap::new();
    for row in rows {
        let binding = row?;
        bindings.insert(binding.project_uid.clone(), binding);
    }

    let mut belief_stmt = conn.prepare(
        "SELECT project_uid, status, source_belief_count, source_value_count,
                indexed_belief_count, indexed_value_count, source_commit_sha, last_verified_at
         FROM mother_project_belief_state",
    )?;
    let belief_rows = belief_stmt.query_map([], |row| {
        Ok(ProjectBeliefBinding {
            project_uid: row.get(0)?,
            status: row.get(1)?,
            source_belief_count: row.get(2)?,
            source_value_count: row.get(3)?,
            indexed_belief_count: row.get(4)?,
            indexed_value_count: row.get(5)?,
            source_commit_sha: row.get(6)?,
            last_verified_at: row.get(7)?,
        })
    })?;

    let mut belief_bindings = std::collections::HashMap::new();
    for row in belief_rows {
        let binding = row?;
        belief_bindings.insert(binding.project_uid.clone(), binding);
    }

    println!(
        "Mother registered projects: {} (identity-bound: {}, belief-verified: {})",
        projects.len(),
        bindings.len(),
        belief_bindings.len()
    );
    println!();

    for project in projects {
        if let Some(binding) = bindings.get(&project.project_uid) {
            println!(
                "- {}\n    uid={} project_id={} status={} checked_in={}\n    user={} vision={} node={}",
                project.project_path,
                project.project_uid,
                binding.project_id,
                binding.status,
                project.updated_at,
                binding.user_id,
                binding.vision_id,
                binding.node_id
            );
        } else {
            println!(
                "- {}\n    uid={} project_id=<missing> status=<unbound> checked_in={}",
                project.project_path, project.project_uid, project.updated_at
            );
        }

        if let Some(beliefs) = belief_bindings.get(&project.project_uid) {
            println!(
                "    beliefs: status={} source={}/{} indexed={}/{} verified={} commit={}",
                beliefs.status,
                beliefs.source_belief_count.unwrap_or(0),
                beliefs.source_value_count.unwrap_or(0),
                beliefs.indexed_belief_count.unwrap_or(0),
                beliefs.indexed_value_count.unwrap_or(0),
                beliefs.last_verified_at,
                beliefs.source_commit_sha.as_deref().unwrap_or("<none>")
            );
        } else {
            println!("    beliefs: status=<unverified>");
        }
    }

    Ok(())
}

fn check_in_project_cli(path: Option<&str>) -> Result<()> {
    let candidate = match path {
        Some(value) => PathBuf::from(value),
        None => std::env::current_dir()?,
    };
    let root = resolve_project_root_from_path(&candidate)?;
    let uid = patina::project::register_with_mother(&root)?;

    let store = patina::mother::MotherRuntimeStore::default();
    let conn = rusqlite::Connection::open(store.path())?;
    let identity = conn
        .query_row(
            "SELECT project_id, user_id, vision_id, node_id, status
             FROM mother_project_identities WHERE project_uid = ?1",
            rusqlite::params![&uid],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, String>(4)?,
                ))
            },
        )
        .optional()?;

    let belief_state = conn
        .query_row(
            "SELECT status, source_belief_count, source_value_count,
                    indexed_belief_count, indexed_value_count, last_verified_at, source_commit_sha
             FROM mother_project_belief_state WHERE project_uid = ?1",
            rusqlite::params![&uid],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, Option<i64>>(1)?,
                    row.get::<_, Option<i64>>(2)?,
                    row.get::<_, Option<i64>>(3)?,
                    row.get::<_, Option<i64>>(4)?,
                    row.get::<_, String>(5)?,
                    row.get::<_, Option<String>>(6)?,
                ))
            },
        )
        .optional()?;

    println!("✓ Checked in: {}", root.display());
    println!("  project_uid: {}", uid);
    if let Some((project_id, user_id, vision_id, node_id, status)) = identity {
        println!("  project_id: {}", project_id);
        println!("  user_id: {}", user_id);
        println!("  vision_id: {}", vision_id);
        println!("  node_id: {}", node_id);
        println!("  status: {}", status);
    } else {
        println!("  identity binding: missing (configure mother user/node/vision tables)");
    }

    if let Some((
        status,
        source_beliefs,
        source_values,
        indexed_beliefs,
        indexed_values,
        verified_at,
        commit_sha,
    )) = belief_state
    {
        println!(
            "  beliefs: status={} source={}/{} indexed={}/{} verified={} commit={}",
            status,
            source_beliefs.unwrap_or(0),
            source_values.unwrap_or(0),
            indexed_beliefs.unwrap_or(0),
            indexed_values.unwrap_or(0),
            verified_at,
            commit_sha.as_deref().unwrap_or("<none>")
        );
    } else {
        println!("  beliefs: status=<unverified>");
    }

    Ok(())
}

fn sync_projects_cli(root: Option<&str>, max_depth: usize) -> Result<()> {
    let scan_root = match root {
        Some(value) => PathBuf::from(value),
        None => dirs::home_dir()
            .map(|home| home.join("Projects"))
            .unwrap_or_else(|| PathBuf::from(".")),
    };

    if !scan_root.exists() {
        bail!("scan root does not exist: {}", scan_root.display());
    }

    let discovered = discover_projects(&scan_root, max_depth)?;
    if discovered.is_empty() {
        println!(
            "No Patina projects discovered under {} (max_depth={}).",
            scan_root.display(),
            max_depth
        );
        return Ok(());
    }

    let mut success = 0usize;
    let mut failed = 0usize;
    for project_root in discovered {
        match patina::project::register_with_mother(&project_root) {
            Ok(uid) => {
                success += 1;
                println!("✓ {} ({})", project_root.display(), uid);
            }
            Err(error) => {
                failed += 1;
                println!("✗ {} ({})", project_root.display(), error);
            }
        }
    }

    println!();
    println!(
        "Project sync complete: {} checked in, {} failed (root={}, max_depth={})",
        success,
        failed,
        scan_root.display(),
        max_depth
    );

    Ok(())
}

fn is_ephemeral_project_path(path: &str) -> bool {
    path.starts_with("/tmp/")
        || path.starts_with("/private/tmp/")
        || path.starts_with("/private/var/folders/")
        || path.contains("/.tmp")
        || path.contains("/tmp.")
}

fn parse_rfc3339_utc(value: &str) -> Option<chrono::DateTime<chrono::Utc>> {
    chrono::DateTime::parse_from_rfc3339(value)
        .ok()
        .map(|dt| dt.with_timezone(&chrono::Utc))
}

fn prune_projects_cli(ephemeral_ttl_days: i64, dry_run: bool) -> Result<()> {
    if ephemeral_ttl_days < 0 {
        bail!("ephemeral TTL must be >= 0 days");
    }

    let store = patina::mother::MotherRuntimeStore::default();
    let projects = store.list_registered_projects()?;
    let cutoff = chrono::Utc::now() - chrono::Duration::days(ephemeral_ttl_days);

    let mut delete_uids = Vec::new();
    let mut reasons = std::collections::BTreeMap::new();

    for project in projects {
        if !is_ephemeral_project_path(&project.project_path) {
            continue;
        }

        let exists = Path::new(&project.project_path).exists();
        if !exists {
            reasons.insert(
                project.project_uid.clone(),
                format!("missing ephemeral path {}", project.project_path),
            );
            delete_uids.push(project.project_uid);
            continue;
        }

        let updated_at = parse_rfc3339_utc(&project.updated_at)
            .or_else(|| parse_rfc3339_utc(&project.registered_at));

        if updated_at.is_some_and(|ts| ts < cutoff) {
            reasons.insert(
                project.project_uid.clone(),
                format!(
                    "ephemeral TTL exceeded (updated_at={}, ttl_days={})",
                    project.updated_at, ephemeral_ttl_days
                ),
            );
            delete_uids.push(project.project_uid);
        }
    }

    delete_uids.sort();
    delete_uids.dedup();

    if delete_uids.is_empty() {
        println!(
            "No ephemeral projects eligible for prune (ttl_days={}, dry_run={}).",
            ephemeral_ttl_days, dry_run
        );
        return Ok(());
    }

    let mut conn = rusqlite::Connection::open(store.path()).with_context(|| {
        format!(
            "opening mother state db for prune: {}",
            store.path().display()
        )
    })?;
    conn.execute("PRAGMA busy_timeout = 10000", [])?;

    let uid_placeholders = std::iter::repeat_n("?", delete_uids.len())
        .collect::<Vec<_>>()
        .join(",");

    let runtime_ids: Vec<String> = conn
        .prepare(&format!(
            "SELECT runtime_id FROM mother_sessions WHERE project_uid IN ({})",
            uid_placeholders
        ))?
        .query_map(rusqlite::params_from_iter(delete_uids.iter()), |row| {
            row.get::<_, String>(0)
        })?
        .collect::<std::result::Result<Vec<_>, _>>()?;

    let linked_sessions = runtime_ids.len();
    let linked_participants: i64 = if runtime_ids.is_empty() {
        0
    } else {
        let rt_placeholders = std::iter::repeat_n("?", runtime_ids.len())
            .collect::<Vec<_>>()
            .join(",");
        conn.query_row(
            &format!(
                "SELECT COUNT(*) FROM mother_session_participants WHERE session_runtime_id IN ({})",
                rt_placeholders
            ),
            rusqlite::params_from_iter(runtime_ids.iter()),
            |row| row.get(0),
        )?
    };

    println!(
        "Prune candidates: {} ephemeral projects (ttl_days={}, dry_run={})",
        delete_uids.len(),
        ephemeral_ttl_days,
        dry_run
    );
    println!(
        "Linked records: sessions={} participants={}",
        linked_sessions, linked_participants
    );
    for uid in &delete_uids {
        if let Some(reason) = reasons.get(uid) {
            println!("- {} ({})", uid, reason);
        } else {
            println!("- {}", uid);
        }
    }

    if dry_run {
        return Ok(());
    }

    let tx = conn.transaction()?;

    if !runtime_ids.is_empty() {
        let rt_placeholders = std::iter::repeat_n("?", runtime_ids.len())
            .collect::<Vec<_>>()
            .join(",");

        tx.execute(
            &format!(
                "DELETE FROM mother_session_participants WHERE session_runtime_id IN ({})",
                rt_placeholders
            ),
            rusqlite::params_from_iter(runtime_ids.iter()),
        )?;

        let handoff_params = runtime_ids
            .iter()
            .chain(runtime_ids.iter())
            .cloned()
            .collect::<Vec<_>>();
        tx.execute(
            &format!(
                "DELETE FROM mother_session_handoffs
                 WHERE from_runtime_id IN ({0}) OR to_runtime_id IN ({0})",
                rt_placeholders
            ),
            rusqlite::params_from_iter(handoff_params.iter()),
        )?;
    }

    tx.execute(
        &format!(
            "DELETE FROM mother_sessions WHERE project_uid IN ({})",
            uid_placeholders
        ),
        rusqlite::params_from_iter(delete_uids.iter()),
    )?;

    tx.execute(
        &format!(
            "DELETE FROM mother_project_belief_state WHERE project_uid IN ({})",
            uid_placeholders
        ),
        rusqlite::params_from_iter(delete_uids.iter()),
    )?;

    tx.execute(
        &format!(
            "DELETE FROM mother_project_identities WHERE project_uid IN ({})",
            uid_placeholders
        ),
        rusqlite::params_from_iter(delete_uids.iter()),
    )?;

    tx.execute(
        &format!(
            "DELETE FROM project_registry WHERE project_uid IN ({})",
            uid_placeholders
        ),
        rusqlite::params_from_iter(delete_uids.iter()),
    )?;

    tx.commit()?;

    let projects_root = paths::mother::data_dir().join("projects");
    let mut removed_dirs = 0usize;
    for uid in &delete_uids {
        if let Ok(dir) = paths::mother::projects::project_dir(uid) {
            if dir.exists() {
                std::fs::remove_dir_all(&dir)
                    .with_context(|| format!("removing pruned project dir {}", dir.display()))?;
                removed_dirs += 1;
            }
        }
    }

    // Safety sweep: remove leftover project dirs no longer present in registry.
    let remaining_uids = store
        .list_registered_projects()?
        .into_iter()
        .map(|entry| entry.project_uid)
        .collect::<std::collections::HashSet<_>>();
    let mut removed_orphans = 0usize;
    if projects_root.exists() {
        for entry in std::fs::read_dir(&projects_root)? {
            let entry = entry?;
            let path = entry.path();
            if !path.is_dir() {
                continue;
            }
            let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
                continue;
            };
            if !remaining_uids.contains(name) {
                std::fs::remove_dir_all(&path)
                    .with_context(|| format!("removing orphan project dir {}", path.display()))?;
                removed_orphans += 1;
            }
        }
    }

    println!();
    println!(
        "Prune complete: removed {} projects, {} project dirs, {} orphan dirs.",
        delete_uids.len(),
        removed_dirs,
        removed_orphans
    );

    Ok(())
}

fn resolve_project_root_from_path(path: &Path) -> Result<PathBuf> {
    let mut current = if path.is_file() {
        path.parent()
            .map(Path::to_path_buf)
            .ok_or_else(|| anyhow::anyhow!("path has no parent: {}", path.display()))?
    } else {
        path.to_path_buf()
    };

    loop {
        if patina::project::is_patina_project(&current) {
            let canonical = std::fs::canonicalize(&current).unwrap_or(current.clone());
            return Ok(canonical);
        }
        let Some(parent) = current.parent().map(Path::to_path_buf) else {
            break;
        };
        current = parent;
    }

    bail!(
        "not inside a Patina project: {}\nRun `patina init .` in the project root first.",
        path.display()
    )
}

fn discover_projects(root: &Path, max_depth: usize) -> Result<Vec<PathBuf>> {
    let mut projects = std::collections::BTreeSet::new();

    for entry in WalkDir::new(root)
        .max_depth(max_depth)
        .follow_links(false)
        .into_iter()
        .filter_entry(should_descend)
    {
        let entry = entry?;
        if !entry.file_type().is_dir() {
            continue;
        }
        let path = entry.path();
        if patina::project::is_patina_project(path) {
            let canonical = std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());
            projects.insert(canonical);
        }
    }

    Ok(projects.into_iter().collect())
}

fn should_descend(entry: &DirEntry) -> bool {
    if entry.depth() == 0 {
        return true;
    }
    let name = entry.file_name().to_string_lossy();
    !matches!(
        name.as_ref(),
        ".git" | ".patina" | "target" | "node_modules" | ".direnv" | "dist" | "build"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(target_os = "linux")]
    use std::ffi::{OsStr, OsString};
    #[cfg(target_os = "linux")]
    use std::os::unix::fs::PermissionsExt;
    #[cfg(target_os = "linux")]
    use std::sync::{Mutex, OnceLock};
    #[cfg(target_os = "linux")]
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn test_mother_command_variants() {
        let start = MotherCommands::Start {
            host: None,
            port: 50051,
            profile: DaemonStartupProfile::Full,
            rivet: RivetIntegrationProfile::Disabled,
            mcp: false,
        };
        assert!(matches!(start, MotherCommands::Start { .. }));

        let graph = MotherCommands::Graph(GraphCommands::Sync);
        assert!(matches!(graph, MotherCommands::Graph(_)));

        let install = MotherCommands::Install;
        assert!(matches!(install, MotherCommands::Install));

        let restart = MotherCommands::Restart;
        assert!(matches!(restart, MotherCommands::Restart));

        let uninstall = MotherCommands::Uninstall;
        assert!(matches!(uninstall, MotherCommands::Uninstall));

        let toys = MotherCommands::Toys(ToysCommands::Status);
        assert!(matches!(toys, MotherCommands::Toys(_)));

        let toys_list = MotherCommands::Toys(ToysCommands::List {
            state: Some("approved".to_string()),
            tier: Some("patina".to_string()),
        });
        assert!(matches!(toys_list, MotherCommands::Toys(_)));

        let projects = MotherCommands::Projects(ProjectsCommands::List);
        assert!(matches!(projects, MotherCommands::Projects(_)));

        let children = MotherCommands::Children(ChildrenCommands::Sources {
            command: None,
            json: false,
        });
        assert!(matches!(children, MotherCommands::Children(_)));

        let children_disable = MotherCommands::Children(ChildrenCommands::Sources {
            command: Some(ChildrenSourcesCommands::Disable {
                source_id: "src_github_demo".to_string(),
            }),
            json: true,
        });
        assert!(matches!(children_disable, MotherCommands::Children(_)));

        let children_show = MotherCommands::Children(ChildrenCommands::Show {
            target: "slate-manager@0.1.0".to_string(),
            json: false,
        });
        assert!(matches!(children_show, MotherCommands::Children(_)));

        let children_install = MotherCommands::Children(ChildrenCommands::Install {
            target: "entry_abc".to_string(),
            installed_by: Some("usr_test".to_string()),
            json: true,
        });
        assert!(matches!(children_install, MotherCommands::Children(_)));

        let children_assign = MotherCommands::Children(ChildrenCommands::Assign {
            target: "entry_abc".to_string(),
            project: "2bdc808e".to_string(),
            reason: Some("test".to_string()),
            json: true,
        });
        assert!(matches!(children_assign, MotherCommands::Children(_)));

        let children_unassign = MotherCommands::Children(ChildrenCommands::Unassign {
            child: "slate-manager".to_string(),
            project: "2bdc808e".to_string(),
            reason: None,
            json: false,
        });
        assert!(matches!(children_unassign, MotherCommands::Children(_)));

        let children_status = MotherCommands::Children(ChildrenCommands::Status {
            project: Some("2bdc808e".to_string()),
            json: true,
        });
        assert!(matches!(children_status, MotherCommands::Children(_)));

        let projects_prune = MotherCommands::Projects(ProjectsCommands::Prune {
            ephemeral_ttl_days: 3,
            dry_run: true,
        });
        assert!(matches!(projects_prune, MotherCommands::Projects(_)));

        let skills_install = MotherCommands::Skills(SkillsCommands::Install {
            child: "fixture-skill-app".to_string(),
            global: false,
            dry_run: true,
            force: false,
            json: true,
        });
        assert!(matches!(skills_install, MotherCommands::Skills(_)));

        let skills_uninstall = MotherCommands::Skills(SkillsCommands::Uninstall {
            child: "fixture-skill-app".to_string(),
            global: false,
            dry_run: true,
            force: false,
            json: true,
        });
        assert!(matches!(skills_uninstall, MotherCommands::Skills(_)));

        let skills_sandbox = MotherCommands::Skills(SkillsCommands::Sandbox(
            skills_sandbox::SandboxCommands::List { json: true },
        ));
        assert!(matches!(skills_sandbox, MotherCommands::Skills(_)));
    }

    #[test]
    fn resolve_project_root_from_nested_path() {
        let temp = tempfile::tempdir().unwrap();
        let project = temp.path().join("alpha");
        std::fs::create_dir_all(project.join(".patina")).unwrap();
        std::fs::write(
            project.join(".patina/config.toml"),
            "[project]\nname='alpha'\n",
        )
        .unwrap();
        std::fs::create_dir_all(project.join("layer")).unwrap();
        std::fs::create_dir_all(project.join("src/bin")).unwrap();

        let nested = project.join("src/bin");
        let resolved = resolve_project_root_from_path(&nested).unwrap();
        assert_eq!(resolved, std::fs::canonicalize(project).unwrap());
    }

    #[test]
    fn ephemeral_project_path_detection_matches_temp_conventions() {
        assert!(is_ephemeral_project_path("/tmp/my-proj"));
        assert!(is_ephemeral_project_path("/private/tmp/my-proj"));
        assert!(is_ephemeral_project_path(
            "/private/var/folders/aa/bb/T/.tmpXYZ/project"
        ));
        assert!(is_ephemeral_project_path("/Users/x/.tmp123/project"));
        assert!(!is_ephemeral_project_path(
            "/Users/nicabar/Projects/Patina/patina"
        ));
    }

    #[test]
    fn discover_projects_finds_patina_roots() {
        let temp = tempfile::tempdir().unwrap();

        let project_a = temp.path().join("a");
        std::fs::create_dir_all(project_a.join(".patina")).unwrap();
        std::fs::write(
            project_a.join(".patina/config.toml"),
            "[project]\nname='a'\n",
        )
        .unwrap();
        std::fs::create_dir_all(project_a.join("layer")).unwrap();

        let project_b = temp.path().join("nested/b");
        std::fs::create_dir_all(project_b.join(".patina")).unwrap();
        std::fs::write(
            project_b.join(".patina/config.toml"),
            "[project]\nname='b'\n",
        )
        .unwrap();
        std::fs::create_dir_all(project_b.join("layer")).unwrap();

        let discovered = discover_projects(temp.path(), 6).unwrap();
        let canonical: std::collections::HashSet<_> = discovered.into_iter().collect();

        assert!(canonical.contains(&std::fs::canonicalize(project_a).unwrap()));
        assert!(canonical.contains(&std::fs::canonicalize(project_b).unwrap()));
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
        assert!(plist.contains(MOTHER_SUPERVISED_ENV));
    }

    #[test]
    #[cfg(target_os = "macos")]
    fn xml_escape_escapes_special_characters() {
        let escaped = xml_escape("a&b<c>\"d\'e");
        assert_eq!(escaped, "a&amp;b&lt;c&gt;&quot;d&apos;e");
    }

    #[test]
    #[cfg(target_os = "linux")]
    fn systemd_unit_contains_required_fields() {
        let unit = render_systemd_unit(Path::new("/tmp/patina"));
        assert!(unit.contains("[Unit]"));
        assert!(unit.contains("Description=Patina Mother daemon"));
        assert!(unit.contains("Environment=PATINA_SUPERVISED=1"));
        assert!(unit.contains("ExecStart=/tmp/patina mother start"));
        assert!(unit.contains("Restart=always"));
        assert!(unit.contains("WantedBy=default.target"));
    }

    #[cfg(target_os = "linux")]
    static LINUX_ENV_MUTEX: OnceLock<Mutex<()>> = OnceLock::new();

    #[cfg(target_os = "linux")]
    fn linux_env_lock() -> &'static Mutex<()> {
        LINUX_ENV_MUTEX.get_or_init(|| Mutex::new(()))
    }

    #[cfg(target_os = "linux")]
    struct EnvRestore {
        key: &'static str,
        previous: Option<OsString>,
    }

    #[cfg(target_os = "linux")]
    impl EnvRestore {
        fn set<K: Into<&'static str>>(key: K, value: &OsStr) -> Self {
            let key = key.into();
            let previous = std::env::var_os(key);
            std::env::set_var(key, value);
            Self { key, previous }
        }
    }

    #[cfg(target_os = "linux")]
    impl Drop for EnvRestore {
        fn drop(&mut self) {
            if let Some(previous) = &self.previous {
                std::env::set_var(self.key, previous);
            } else {
                std::env::remove_var(self.key);
            }
        }
    }

    #[test]
    #[cfg(target_os = "linux")]
    fn linux_supervisor_install_uninstall_uses_systemd_user_contract() {
        let _lock = linux_env_lock().lock().expect("env lock poisoned");

        let ts = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock before unix epoch")
            .as_nanos();
        let sandbox = std::env::temp_dir().join(format!("patina-mother-systemd-test-{ts}"));
        let home = sandbox.join("home");
        let bin_dir = sandbox.join("bin");
        let systemctl_script = bin_dir.join("systemctl");
        let systemctl_log = sandbox.join("systemctl.log");

        std::fs::create_dir_all(&home).expect("create home dir");
        std::fs::create_dir_all(&bin_dir).expect("create bin dir");

        let script = format!(
            "#!/usr/bin/env bash\nset -euo pipefail\necho \"$*\" >> \"{}\"\n",
            systemctl_log.display()
        );
        std::fs::write(&systemctl_script, script).expect("write fake systemctl script");
        let mut perms = std::fs::metadata(&systemctl_script)
            .expect("read fake systemctl metadata")
            .permissions();
        perms.set_mode(0o755);
        std::fs::set_permissions(&systemctl_script, perms).expect("chmod fake systemctl");

        let _home = EnvRestore::set("HOME", home.as_os_str());
        let _systemctl = EnvRestore::set(MOTHER_SYSTEMCTL_BIN_ENV, systemctl_script.as_os_str());

        install_supervisor().expect("linux install supervisor should succeed");

        let unit_path = home.join(".config/systemd/user/patina-mother.service");
        assert!(unit_path.exists(), "expected systemd user unit to exist");
        let unit_body = std::fs::read_to_string(&unit_path).expect("read systemd user unit");
        assert!(unit_body.contains("Environment=PATINA_SUPERVISED=1"));

        uninstall_supervisor().expect("linux uninstall supervisor should succeed");
        assert!(
            !unit_path.exists(),
            "expected systemd user unit to be removed"
        );

        let log = std::fs::read_to_string(&systemctl_log).expect("read fake systemctl log");
        assert!(log.contains("--user daemon-reload"));
        assert!(log.contains("--user enable --now patina-mother.service"));
        assert!(log.contains("--user disable --now patina-mother.service"));

        let _ = std::fs::remove_dir_all(&sandbox);
    }

    #[test]
    fn manual_start_block_reason_blocks_launchd_without_supervised_marker() {
        let reason = manual_start_block_reason(SupervisorBackend::LaunchdPatina, false);
        assert!(reason.is_some());
    }

    #[test]
    fn manual_start_block_reason_blocks_systemd_without_supervised_marker() {
        let reason = manual_start_block_reason(SupervisorBackend::SystemdUser, false);
        assert!(reason.is_some());
    }

    #[test]
    fn manual_start_block_reason_allows_supervised_invocation() {
        let reason = manual_start_block_reason(SupervisorBackend::LaunchdPatina, true);
        assert!(reason.is_none());
    }

    #[test]
    fn classify_supervisor_backend_defaults_to_manual() {
        let backend = classify_supervisor_backend(false, false, false);
        assert_eq!(backend, SupervisorBackend::Manual);
    }

    #[test]
    fn classify_supervisor_backend_prefers_homebrew_when_both_launchd_markers_exist() {
        let backend = classify_supervisor_backend(true, true, false);
        assert_eq!(backend, SupervisorBackend::LaunchdHomebrew);
    }

    #[test]
    fn classify_supervisor_backend_detects_systemd_user() {
        let backend = classify_supervisor_backend(false, false, true);
        assert_eq!(backend, SupervisorBackend::SystemdUser);
    }
}
