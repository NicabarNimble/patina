#![recursion_limit = "256"]

use anyhow::Result;
use clap::{Args, Parser, Subcommand, ValueEnum};

mod commands;
mod main_dispatch;
mod preflight;
mod retrieval;
#[cfg(test)]
mod test_support;

// ============================================================================
// Typed CLI enums (Phase 0d: type safety for string args)
// ============================================================================

/// Search dimension for scry and eval commands
#[derive(Clone, Copy, Debug, PartialEq, Eq, ValueEnum)]
pub enum Dimension {
    /// Semantic similarity search
    Semantic,
    /// Temporal/co-change relationships
    Temporal,
    /// Code dependency relationships
    Dependency,
}

impl Dimension {
    pub fn as_str(&self) -> &'static str {
        match self {
            Dimension::Semantic => "semantic",
            Dimension::Temporal => "temporal",
            Dimension::Dependency => "dependency",
        }
    }
}

/// LLM interface for project initialization
#[derive(Clone, Copy, Debug, PartialEq, Eq, ValueEnum)]
pub enum Llm {
    /// Claude Code (Anthropic)
    Claude,
    /// Gemini CLI (Google)
    Gemini,
    /// OpenCode
    OpenCode,
    /// Local LLM
    Local,
}

impl Llm {
    pub fn as_str(&self) -> &'static str {
        match self {
            Llm::Claude => "claude",
            Llm::Gemini => "gemini",
            Llm::OpenCode => "opencode",
            Llm::Local => "local",
        }
    }
}

#[derive(Parser)]
#[command(author, version = env!("CARGO_PKG_VERSION"), about = "Context management for AI-assisted development", long_about = None)]
struct Cli {
    /// AI interface to launch (claude, gemini, opencode). Default: from config.
    #[arg(long = "interface", alias = "adapter", global = true)]
    interface: Option<String>,

    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Initialize a new project skeleton, then prepare Patina AI with `patina ai setup`
    Init {
        /// Project name or "." for current directory
        name: String,

        /// Force initialization, backup and replace existing patina branch
        #[arg(long)]
        force: bool,

        /// Local-only mode (skip GitHub integration)
        #[arg(long)]
        local: bool,

        /// Skip automatic git commit
        #[arg(long)]
        no_commit: bool,
    },

    /// Check for new Patina CLI versions
    #[cfg(feature = "dev")]
    Upgrade {
        /// Only check for updates, don't show instructions
        #[arg(short, long)]
        check: bool,

        /// Output results as JSON
        #[arg(short, long)]
        json: bool,
    },

    /// Developer commands (only available with --features dev)
    #[cfg(feature = "dev")]
    Dev {
        #[command(subcommand)]
        command: DevCommands,
    },

    /// Check project health and environment
    Doctor {
        /// Output results as JSON
        #[arg(short, long)]
        json: bool,
    },

    /// Manage WASM children
    #[command(name = "child", visible_alias = "plugin")]
    Child {
        #[command(subcommand)]
        command: ChildCommands,
    },

    /// Manage project versioning (semver: MAJOR.MINOR.PATCH)
    Version {
        #[command(subcommand)]
        command: Option<commands::version::VersionCommands>,

        /// Output as JSON (for default 'show' behavior)
        #[arg(short, long)]
        json: bool,

        /// Show component versions (for default 'show' behavior)
        #[arg(short, long)]
        components: bool,
    },

    /// Build semantic knowledge database
    Scrape {
        #[command(subcommand)]
        command: Option<ScrapeCommands>,

        /// Rebuild database from scratch (for ref repos: removes old eventlog bloat)
        #[arg(long)]
        rebuild: bool,
    },

    /// Build embeddings and projections from recipe
    Oxidize {
        /// Build for a registered external repo (e.g., clawdbot/clawdbot)
        #[arg(long)]
        repo: Option<String>,
    },

    /// Rebuild .patina/ from layer/ and local sources (portability)
    Rebuild {
        /// Only run scrape step (skip oxidize)
        #[arg(long)]
        scrape: bool,

        /// Only run oxidize step (assume db exists)
        #[arg(long)]
        oxidize: bool,

        /// Delete existing data before rebuild
        #[arg(long)]
        force: bool,

        /// Show what would be rebuilt without doing it
        #[arg(long)]
        dry_run: bool,
    },

    /// Search codebase knowledge — semantic vector search over beliefs,
    /// patterns, and commit messages (knowledge domain)
    Scry {
        #[command(subcommand)]
        command: Option<ScryCommands>,

        /// Query text to search for (optional if --file is provided)
        query: Option<String>,

        /// File path for temporal/dependency queries (e.g., src/auth.rs)
        #[arg(long)]
        file: Option<String>,

        /// Belief ID for grounding queries — find nearest code/commits/sessions (E4.6a)
        #[arg(long, conflicts_with = "file")]
        belief: Option<String>,

        /// Filter results by content type (used with --belief): code, commits, sessions, patterns, beliefs
        #[arg(long, value_name = "TYPE")]
        content_type: Option<String>,

        /// Maximum number of results (default: 10)
        #[arg(long, default_value = "10")]
        limit: usize,

        /// Minimum similarity score (0.0-1.0, default: 0.0)
        #[arg(long, default_value = "0.0")]
        min_score: f32,

        /// Query a specific external repo (registered via 'patina repo')
        #[arg(long)]
        repo: Option<String>,

        /// Query all registered repos (current project + reference repos)
        #[arg(long)]
        all_repos: bool,

        /// Include GitHub issues in search results
        #[arg(long)]
        include_issues: bool,

        /// Exclude persona knowledge from results
        #[arg(long)]
        no_persona: bool,

        /// Show detailed oracle contributions for each result
        #[arg(long)]
        explain: bool,

        /// Show belief impact for code results — which beliefs may be affected (E4.6a)
        #[arg(long)]
        impact: bool,

        /// Fetch full content for a single result from a previous query (D3 scan-then-focus)
        #[arg(long, value_name = "QUERY_ID", conflicts_with_all = ["query", "file", "belief"])]
        detail: Option<String>,

        /// Rank of the result to fetch (1-indexed, used with --detail)
        #[arg(long, default_value = "1", requires = "detail")]
        rank: usize,
    },

    /// Get project patterns and conventions — USE THIS to understand design rules
    /// before making architectural changes. Returns core patterns (eternal principles),
    /// surface patterns (active architecture), and project beliefs.
    Context {
        /// Optional topic to focus on (e.g., 'error handling', 'testing', 'architecture')
        #[arg(long)]
        topic: Option<String>,
    },

    /// Evaluate retrieval quality across dimensions
    Eval {
        /// Specific dimension to evaluate (semantic, temporal)
        #[arg(long, value_enum)]
        dimension: Option<Dimension>,

        /// Show real-world precision from session feedback loop (Phase 3)
        #[arg(long)]
        feedback: bool,

        /// Run natural-language query eval from curated test set
        #[arg(long)]
        nl: bool,

        /// Independent assay eval (factual/FTS5 retrieval)
        #[arg(long)]
        assay: bool,

        /// Independent scry eval (semantic/vector retrieval) + scry-vs-assay comparison
        #[arg(long)]
        scry: bool,

        /// Raw E5 diagnostic: brute-force cosine without projection (Phase 5d)
        #[arg(long)]
        scry_raw: bool,

        /// Combined eval (full pipeline: assay + scry together)
        #[arg(long)]
        combined: bool,
    },

    /// Benchmark retrieval quality with ground truth
    Bench {
        #[command(subcommand)]
        command: BenchCommands,
    },

    /// Cross-project user knowledge (preferences, style, history)
    Persona {
        #[command(subcommand)]
        command: PersonaCommands,
    },

    /// Manage external repositories for cross-project knowledge
    Repo {
        #[command(subcommand)]
        command: Option<RepoCommands>,

        /// Repository URL (shorthand for 'patina repo add <url>')
        url: Option<String>,

        /// Enable contribution mode (create fork for PRs)
        #[arg(long, requires = "url")]
        contrib: bool,

        /// Sparse checkout path (repeatable; shorthand mode)
        #[arg(long = "sparse", requires = "url", action = clap::ArgAction::Append)]
        sparse: Vec<String>,
    },

    /// Manage embedding models in mother cache
    Model {
        #[command(subcommand)]
        command: Option<commands::model::ModelCommands>,
    },

    /// Manage external service connections (OAuth, tokens, credentials)
    ///
    /// Create, list, and manage connections to external services like GitHub.
    /// Connections store credentials in the vault and are consumed by the broker.
    Connect {
        #[command(subcommand)]
        command: Option<commands::connect::ConnectCommands>,
    },

    /// Manage DuckLake data lakes
    ///
    /// Create and list data lakes. Lakes are DuckDB + DuckLake storage
    /// backed by autonomous child processes.
    Lake {
        #[command(subcommand)]
        command: Option<commands::lake::LakeCommands>,
    },

    /// The Patina daemon — cross-project knowledge, caching, and routing
    ///
    /// Mother is the always-running daemon that provides hot model caching,
    /// cross-project knowledge access, secrets caching, and graph-based routing.
    Mother {
        #[command(subcommand)]
        command: Option<commands::mother::MotherCommands>,
    },

    /// Manage composed pando products
    Pando {
        #[command(subcommand)]
        command: Option<commands::pando::PandoCommands>,
    },

    /// Secure secret management with age encryption
    Secrets {
        #[command(subcommand)]
        command: Option<commands::secrets::SecretsCommands>,

        #[command(flatten)]
        flags: commands::secrets::SecretsFlags,
    },

    /// Generate YOLO devcontainer for autonomous AI development
    Yolo {
        /// Use interactive mode to choose options
        #[arg(short, long)]
        interactive: bool,

        /// Use all defaults without prompting
        #[arg(short, long, conflicts_with = "interactive")]
        defaults: bool,

        /// Additional tools to include (e.g., --with cairo,solidity)
        #[arg(long, value_delimiter = ',')]
        with: Option<Vec<String>>,

        /// Tools to exclude from detection (e.g., --without python)
        #[arg(long, value_delimiter = ',')]
        without: Option<Vec<String>>,

        /// Output results as JSON
        #[arg(short, long)]
        json: bool,
    },

    /// Start the Mother daemon (DEPRECATED — use `patina mother start`)
    #[command(hide = true)]
    Serve {
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

    /// Manage AI interfaces (list, add, remove, refresh, doctor)
    #[command(alias = "adapter")]
    Interface {
        #[command(subcommand)]
        command: Option<InterfaceManageCommands>,
    },

    /// Generate project state report using patina's own tools
    Report {
        /// Output path (default: layer/surface/reports/YYYY-MM-DD-state.md)
        #[arg(long, short)]
        output: Option<String>,

        /// Query a specific registered repo
        #[arg(long)]
        repo: Option<String>,

        /// Output as JSON instead of markdown
        #[arg(long)]
        json: bool,
    },

    /// Show project health from measurement data
    Measure {
        /// Show raw metrics and history (maintainer view)
        #[arg(long)]
        system: bool,

        /// Output as machine-readable JSON
        #[arg(long)]
        json: bool,

        /// Drill-down into a specific verb with history
        #[arg(long)]
        verb: Option<String>,

        /// Show full health report with freshness, diagnostics, and health summary
        #[arg(long)]
        full: bool,
    },

    /// Patina AI interface surface over Mother-backed sessions
    Ai {
        #[command(subcommand)]
        command: Option<commands::ai::AiCommands>,
    },

    /// Git hook handlers (post-commit, post-merge)
    Hook {
        #[command(subcommand)]
        command: commands::hook::HookCommands,
    },

    /// Audit epistemic beliefs — show use/truth metrics
    Belief {
        #[command(subcommand)]
        command: Option<commands::belief::BeliefCommands>,
    },

    /// First-run setup for components (grammars, etc.)
    Setup {
        #[command(subcommand)]
        command: SetupCommands,
    },

    /// Manage Slate work transactions
    Slate {
        #[command(subcommand)]
        command: commands::slate::SlateCommands,
    },

    /// Manage spec lifecycle (archive completed specs)
    Spec {
        /// Target Patina project path or project UID for cross-project spec operations
        #[arg(long)]
        project: Option<String>,

        #[command(subcommand)]
        command: commands::spec::SpecCommands,
    },

    /// Manage fact schemas (install, list, show)
    Schema {
        #[command(subcommand)]
        command: commands::schema::SchemaCommands,
    },

    /// Manage event store (export/import JSONL replica)
    Events {
        #[command(subcommand)]
        command: EventsCommands,
    },

    /// Query codebase structure (modules, imports, call graph)
    Assay {
        #[command(subcommand)]
        command: Option<AssayCommands>,

        /// Pattern to filter results (for default inventory mode)
        pattern: Option<String>,

        /// Maximum number of results (default: 50)
        #[arg(long, default_value = "50")]
        limit: usize,

        /// Output as JSON
        #[arg(long)]
        json: bool,

        /// Query a specific external repo (registered via 'patina repo')
        #[arg(long)]
        repo: Option<String>,

        /// Query all registered repos (current project + reference repos)
        #[arg(long)]
        all_repos: bool,
    },
}

#[derive(Subcommand)]
enum EventsCommands {
    /// Export new events to layer/events.jsonl (incremental, at-least-once)
    Export,

    /// Import events from JSONL file (disaster recovery)
    Import {
        /// Path to JSONL file to import
        path: String,
    },
}

#[derive(Subcommand)]
enum AssayCommands {
    /// Module inventory with line counts and stats (default)
    Inventory {
        /// Path pattern to filter modules
        pattern: Option<String>,

        /// Maximum number of results
        #[arg(long, default_value = "50")]
        limit: usize,

        /// Output as JSON
        #[arg(long)]
        json: bool,
    },
    /// What a module imports
    Imports {
        /// Module path pattern
        module: String,

        /// Maximum number of results
        #[arg(long, default_value = "100")]
        limit: usize,

        /// Output as JSON
        #[arg(long)]
        json: bool,
    },
    /// What modules import a given module
    Importers {
        /// Module name to search for
        module: String,

        /// Maximum number of results
        #[arg(long, default_value = "100")]
        limit: usize,

        /// Output as JSON
        #[arg(long)]
        json: bool,
    },
    /// List functions in the codebase
    Functions {
        /// Pattern to filter functions
        pattern: Option<String>,

        /// Maximum number of results
        #[arg(long, default_value = "100")]
        limit: usize,

        /// Output as JSON
        #[arg(long)]
        json: bool,
    },
    /// What functions call a given function
    Callers {
        /// Function name to search for
        function: String,

        /// Maximum number of results
        #[arg(long, default_value = "100")]
        limit: usize,

        /// Output as JSON
        #[arg(long)]
        json: bool,
    },
    /// What functions a given function calls
    Callees {
        /// Function name to search for
        function: String,

        /// Maximum number of results
        #[arg(long, default_value = "100")]
        limit: usize,

        /// Output as JSON
        #[arg(long)]
        json: bool,
    },
    /// Compute structural signals for all modules (is_used, activity, centrality)
    Derive {
        /// Output as JSON
        #[arg(long)]
        json: bool,
    },
    /// Derive temporal moments from git history (genesis, breaking, migration, etc.)
    #[command(name = "derive-moments")]
    DeriveMoments {
        /// Output as JSON
        #[arg(long)]
        json: bool,
    },
    /// Ranked factual search across code, commits, and patterns (FTS5)
    Search {
        /// Search query text
        query: String,

        /// Maximum number of results
        #[arg(long, default_value = "10")]
        limit: usize,

        /// Output as JSON
        #[arg(long)]
        json: bool,

        /// Include GitHub issues in results
        #[arg(long)]
        include_issues: bool,
    },
    /// Co-change analysis — find files that frequently change together
    Cochange {
        /// File path to analyze
        file: String,

        /// Maximum number of results
        #[arg(long, default_value = "20")]
        limit: usize,

        /// Output as JSON
        #[arg(long)]
        json: bool,
    },
    /// Belief grounding — find evidence and reached code for a belief
    Belief {
        /// Belief ID to ground
        id: String,

        /// Maximum results per section
        #[arg(long, default_value = "10")]
        limit: usize,

        /// Output as JSON
        #[arg(long)]
        json: bool,
    },
}

/// Common arguments for all scrape subcommands
#[derive(Args)]
struct ScrapeArgs {
    /// Initialize the knowledge database
    #[arg(long)]
    init: bool,

    /// Force full re-index (ignore incremental updates)
    #[arg(long)]
    force: bool,
}

#[derive(Subcommand)]
enum ScrapeCommands {
    /// Extract semantic information using modular architecture
    Code {
        #[command(flatten)]
        args: ScrapeArgs,
    },
    /// Extract git commit history and co-change relationships
    Git {
        /// Full rebuild (ignore incremental)
        #[arg(long)]
        full: bool,
    },
    /// Extract sessions (deprecated — use `scrape layer` instead)
    Sessions {
        /// Full rebuild (ignore incremental)
        #[arg(long)]
        full: bool,
    },
    /// Scrape all layer content: patterns, sessions, specs
    Layer {
        /// Full rebuild (ignore incremental)
        #[arg(long)]
        full: bool,
    },
}

#[derive(Subcommand)]
enum BenchCommands {
    /// Benchmark grammar plugin overhead (compiled-in vs WASM)
    Grammar {
        /// Maximum number of files to benchmark
        #[arg(long)]
        files: Option<usize>,
    },

    /// Benchmark retrieval quality
    Retrieval {
        /// Path to query set JSON file
        #[arg(long, short = 'q')]
        query_set: String,

        /// Number of results per query (default: 10)
        #[arg(long, default_value = "10")]
        limit: usize,

        /// Output as JSON
        #[arg(long, short)]
        json: bool,

        /// Show detailed per-query analysis (expected vs retrieved docs)
        #[arg(long, short)]
        verbose: bool,

        /// Override RRF k value (default: from config or 60)
        #[arg(long)]
        rrf_k: Option<usize>,

        /// Override fetch multiplier (default: from config or 2)
        #[arg(long)]
        fetch_multiplier: Option<usize>,

        /// Filter to specific oracle(s) for ablation testing (semantic, lexical, persona)
        #[arg(long)]
        oracle: Option<Vec<String>>,

        /// Query a specific registered repo instead of current project
        #[arg(long)]
        repo: Option<String>,
    },

    /// Generate queryset from git commits (deterministic ground truth)
    Generate {
        /// Generate from commits (default source)
        #[arg(long, default_value = "true")]
        from_commits: bool,

        /// Repository name (omit for current project)
        #[arg(long)]
        repo: Option<String>,

        /// Maximum number of queries to generate
        #[arg(long, default_value = "100")]
        limit: usize,

        /// Output file path (omit for stdout)
        #[arg(long, short)]
        output: Option<String>,
    },
}

#[derive(Subcommand)]
enum ScryCommands {
    /// Orient to a directory - show important files ranked by structural signals
    Orient {
        /// Directory path to orient (e.g., src/retrieval/)
        path: String,

        /// Maximum number of results (default: 10)
        #[arg(long, default_value = "10")]
        limit: usize,
    },

    /// Recent changes - show files that changed recently, optionally filtered by query
    Recent {
        /// Optional query to filter files (e.g., "retrieval" to show recent retrieval changes)
        query: Option<String>,

        /// Number of days to look back (default: 7)
        #[arg(long, default_value = "7")]
        days: u32,

        /// Maximum number of results (default: 10)
        #[arg(long, default_value = "10")]
        limit: usize,
    },

    /// Explain why a specific result was returned
    Why {
        /// Document ID to explain (e.g., "src/retrieval/engine.rs")
        doc_id: String,

        /// The query that returned this result
        query: String,
    },

    /// Open a result file and log usage (Phase 3 feedback)
    Open {
        /// Query ID from previous scry command
        query_id: String,

        /// Result rank to open (1-based)
        rank: usize,
    },

    /// Copy a result to clipboard and log usage (Phase 3 feedback)
    Copy {
        /// Query ID from previous scry command
        query_id: String,

        /// Result rank to copy (1-based)
        rank: usize,
    },

    /// Record explicit feedback on query results (Phase 3 feedback)
    Feedback {
        /// Query ID from previous scry command
        query_id: String,

        /// Feedback signal: "good" or "bad"
        signal: String,

        /// Optional comment explaining the feedback
        #[arg(long)]
        comment: Option<String>,
    },
}

#[derive(Subcommand)]
enum PersonaCommands {
    /// Capture knowledge directly
    Note {
        /// Content to capture
        content: String,

        /// Domains this applies to (comma-separated, e.g., rust,error-handling)
        #[arg(long, value_delimiter = ',')]
        domains: Option<Vec<String>>,

        /// Event ID this supersedes (replaces old knowledge)
        #[arg(long)]
        supersedes: Option<String>,
    },

    /// Search persona knowledge
    Query {
        /// Search query
        query: String,

        /// Maximum results (default: 10)
        #[arg(long, default_value = "10")]
        limit: usize,

        /// Minimum similarity score (0.0-1.0, default: 0.0)
        #[arg(long, default_value = "0.0")]
        min_score: f32,

        /// Filter by domains (comma-separated)
        #[arg(long, value_delimiter = ',')]
        domains: Option<Vec<String>>,
    },

    /// List captured knowledge
    List {
        /// Maximum entries to show (default: 10)
        #[arg(long, default_value = "10")]
        limit: usize,

        /// Filter by domains (comma-separated)
        #[arg(long, value_delimiter = ',')]
        domains: Option<Vec<String>>,
    },

    /// Process events into searchable index
    Materialize,

    /// Check persona oracle status
    Status,
}

// CLI subcommand enums are defined in their respective command modules
use commands::interface::InterfaceManageCommands;
use commands::repo::RepoCommands;

#[cfg(feature = "dev")]
#[derive(Subcommand)]
enum DevCommands {
    /// Validate resources and patterns
    Validate {
        /// Output results as JSON
        #[arg(short, long)]
        json: bool,
    },

    /// Prepare for a new release
    Release {
        /// Version bump type
        #[arg(value_enum)]
        bump: Option<BumpType>,

        /// Dry run - don't make changes
        #[arg(short, long)]
        dry_run: bool,
    },

    /// Sync interface templates from resources
    SyncInterfaces {
        /// Specific interface to sync (claude, gemini, opencode)
        interface: Option<String>,

        /// Dry run - show what would change
        #[arg(short, long)]
        dry_run: bool,
    },

    /// Bump component versions
    BumpVersion {
        /// Component to bump (patina, claude-interface, etc)
        component: String,

        /// Version bump type
        #[arg(value_enum)]
        bump_type: BumpType,

        /// Dry run - don't make changes
        #[arg(short, long)]
        dry_run: bool,
    },

    /// Update test fixtures
    UpdateFixtures {
        /// Specific fixture to update
        fixture: Option<String>,
    },
}

#[cfg(feature = "dev")]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, clap::ValueEnum)]
enum BumpType {
    Major,
    Minor,
    Patch,
}

#[derive(Subcommand)]
enum SetupCommands {
    /// Install default grammar plugins to ~/.patina/pipeline/
    Grammars {
        /// Show what would be installed without doing it
        #[arg(long)]
        list: bool,

        /// Install only specific grammars (comma-separated)
        #[arg(long, value_delimiter = ',')]
        only: Option<Vec<String>>,

        /// Force reinstall even if already present
        #[arg(long)]
        force: bool,
    },
}

#[derive(Subcommand)]
enum ChildCommands {
    /// List installed children
    List,
    /// Run a child by name
    Run {
        /// Child name (matches <name>.wasm in command-children dir)
        name: String,
        /// Arguments passed to the child
        #[arg(last = true)]
        args: Vec<String>,
    },
    /// Call a typed WIT business operation on a child
    Call {
        /// Child name (matches <name>.wasm in command-children dir)
        name: String,
        /// Fully-qualified operation id (`package:interface.function`)
        operation_id: String,
        /// JSON args payload (positional array)
        #[arg(default_value = "[]")]
        args_json: String,
    },
    /// Create a new child project from template
    Init {
        /// Child name (valid Rust crate name, e.g. "review-bot")
        name: String,
        /// Child world: child, pipeline (default: child)
        #[arg(long, default_value = "child")]
        world: String,
        /// Use legacy scaffold lane (maintenance only)
        #[arg(long)]
        legacy: bool,
        /// Build the child after scaffolding
        #[arg(long)]
        build: bool,
        /// Build in release mode (requires --build)
        #[arg(long, requires = "build")]
        release: bool,
    },
    /// Install a packaged child into the local Patina plugin directory
    Install {
        /// Path to child package directory containing child.toml
        path: String,
        /// Path to built component .wasm; if omitted, common target paths are probed
        #[arg(long)]
        wasm: Option<String>,
        /// Overwrite an existing installed child
        #[arg(long)]
        force: bool,
        /// Do not preserve local-only scope additions from an existing manifest
        #[arg(long)]
        no_preserve_local_scopes: bool,
    },
}

fn main() -> Result<()> {
    // Run migrations early (before any command)
    patina::migration::migrate_if_needed();
    commands::repo::migrate_registry_paths();

    // Preflight: clean up stale processes before normal operation
    preflight::ensure_clean_state();

    let cli = Cli::parse();

    if !matches!(cli.command.as_ref(), Some(Commands::Mother { .. })) {
        commands::pando::init_registry_best_effort();
    }

    match cli.command {
        // Launcher mode: no subcommand means launch interface
        None => {
            let options = commands::launch::LaunchOptions {
                path: None,
                interface: cli.interface,
                auto_start_mother: true,
                auto_init: true,
            };
            commands::launch::execute(options)?;
        }

        Some(Commands::Init {
            name,
            force,
            local,
            no_commit,
        }) => {
            commands::init::execute(name, force, local, no_commit)?;
        }
        #[cfg(feature = "dev")]
        Some(Commands::Upgrade { check, json }) => {
            main_dispatch::dev::dispatch_upgrade(check, json)?;
        }
        #[cfg(feature = "dev")]
        Some(Commands::Dev { command }) => {
            main_dispatch::dev::dispatch_dev(command)?;
        }
        Some(Commands::Scrape { command, rebuild }) => {
            main_dispatch::scrape::dispatch_scrape(command, rebuild)?;
        }
        Some(Commands::Oxidize { repo }) => {
            main_dispatch::scrape::dispatch_oxidize(repo)?;
        }
        Some(Commands::Rebuild {
            scrape,
            oxidize,
            force,
            dry_run,
        }) => {
            main_dispatch::scrape::dispatch_rebuild(scrape, oxidize, force, dry_run)?;
        }
        Some(Commands::Scry {
            command,
            query,
            file,
            belief,
            content_type,
            limit,
            min_score,
            repo,
            all_repos,
            include_issues,
            no_persona,
            explain,
            impact,
            detail,
            rank,
        }) => {
            main_dispatch::scrape::dispatch_scry(
                command,
                query,
                file,
                belief,
                content_type,
                limit,
                min_score,
                repo,
                all_repos,
                include_issues,
                no_persona,
                explain,
                impact,
                detail,
                rank,
            )?;
        }
        Some(Commands::Context { topic }) => {
            main_dispatch::scrape::dispatch_context(topic)?;
        }
        Some(Commands::Eval {
            dimension,
            feedback,
            nl,
            assay,
            scry,
            scry_raw,
            combined,
        }) => {
            main_dispatch::scrape::dispatch_eval(
                dimension, feedback, nl, assay, scry, scry_raw, combined,
            )?;
        }
        Some(Commands::Bench { command }) => {
            main_dispatch::scrape::dispatch_bench(command)?;
        }
        Some(Commands::Persona { command }) => {
            main_dispatch::scrape::dispatch_persona(command)?;
        }
        Some(Commands::Doctor { json }) => {
            commands::doctor::execute_cli(json)?;
        }
        Some(Commands::Child { command }) => {
            main_dispatch::child::dispatch(command)?;
        }
        Some(Commands::Repo {
            command,
            url,
            contrib,
            sparse,
        }) => commands::repo::execute_cli(command, url, contrib, sparse)?,
        Some(Commands::Model { command }) => commands::model::execute_cli(command)?,
        Some(Commands::Connect { command }) => commands::connect::execute_cli(command)?,
        Some(Commands::Lake { command }) => commands::lake::execute_cli(command)?,
        Some(Commands::Mother { command }) => {
            main_dispatch::mother::dispatch_mother(command)?;
        }
        Some(Commands::Pando { command }) => commands::pando::execute_cli(command)?,
        Some(Commands::Secrets { command, flags }) => {
            commands::secrets::execute_cli(command, flags)?
        }
        Some(Commands::Yolo {
            interactive,
            defaults,
            with,
            without,
            json,
        }) => {
            commands::yolo::execute(interactive, defaults, with, without, json)?;
        }
        Some(Commands::Version {
            command,
            json,
            components,
        }) => {
            if let Some(subcmd) = command {
                commands::version::execute_subcommand(subcmd)?;
            } else {
                // Default behavior: show version
                commands::version::execute(json, components)?;
            }
        }
        Some(Commands::Ai { command }) => {
            commands::ai::execute(command)?;
        }
        Some(Commands::Hook { command }) => {
            commands::hook::execute(command)?;
        }
        Some(Commands::Belief { command }) => {
            commands::belief::execute(command)?;
        }
        Some(Commands::Setup { command }) => match command {
            SetupCommands::Grammars { list, only, force } => {
                let options = commands::setup::GrammarOptions { list, only, force };
                commands::setup::execute_grammars(options)?;
            }
        },
        Some(Commands::Slate { command }) => {
            commands::slate::execute(command)?;
        }
        Some(Commands::Spec { project, command }) => {
            main_dispatch::spec::dispatch_spec(project, command)?;
        }
        Some(Commands::Schema { command }) => match command {
            commands::schema::SchemaCommands::Install { path } => {
                commands::schema::install(&path)?;
            }
            commands::schema::SchemaCommands::List { json } => {
                commands::schema::list(json)?;
            }
            commands::schema::SchemaCommands::Show { name, json } => {
                commands::schema::show(&name, json)?;
            }
            commands::schema::SchemaCommands::New {
                name,
                version,
                description,
                facts,
            } => {
                commands::schema::new_schema(&name, &version, &description, facts.as_deref())?;
            }
            commands::schema::SchemaCommands::Generate {
                types,
                migrations,
                embeddings,
                schema,
            } => {
                commands::schema::generate(types, migrations, embeddings, schema.as_deref())?;
            }
            commands::schema::SchemaCommands::Check => {
                commands::schema::check()?;
            }
            commands::schema::SchemaCommands::Build {
                name,
                types,
                migrations,
                embeddings,
            } => {
                commands::schema::build(&name, types, migrations, embeddings)?;
            }
        },
        Some(Commands::Serve { host, port, mcp }) => {
            // MCP server path has been retired; legacy `serve --mcp` is rejected in dispatch.
            main_dispatch::mother::dispatch_serve(host, port, mcp)?;
        }
        Some(Commands::Interface { command }) => commands::interface::execute(command)?,
        Some(Commands::Report { output, repo, json }) => {
            let options = commands::report::ReportOptions { output, repo, json };
            commands::report::execute(options)?;
        }
        Some(Commands::Measure {
            system,
            json,
            verb,
            full,
        }) => {
            let options = commands::measure::MeasureOptions {
                system,
                json,
                verb,
                full,
            };
            commands::measure::execute(options)?;
        }
        Some(Commands::Events { command }) => match command {
            EventsCommands::Export => {
                commands::events::export()?;
            }
            EventsCommands::Import { path } => {
                commands::events::import(&path)?;
            }
        },
        Some(Commands::Assay {
            command,
            pattern,
            limit,
            json,
            repo,
            all_repos,
        }) => {
            main_dispatch::scrape::dispatch_assay(command, pattern, limit, json, repo, all_repos)?;
        }
    }

    Ok(())
}
