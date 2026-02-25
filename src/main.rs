use anyhow::Result;
use clap::{Args, Parser, Subcommand, ValueEnum};

mod commands;
mod mcp;
mod preflight;
mod retrieval;

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

/// LLM adapter for project initialization
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
    /// Adapter to launch (claude, gemini, codex). Default: from config.
    #[arg(long = "adapter", global = true)]
    adapter: Option<String>,

    /// Disable tmux session wrapping (launch adapter directly)
    #[arg(long = "no-tmux", global = true)]
    no_tmux: bool,

    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Initialize a new project (skeleton only - use 'patina adapter add' for LLM support)
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

    /// Manage WASM plugins
    Plugin {
        #[command(subcommand)]
        command: PluginCommands,
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
        #[arg(conflicts_with = "command")]
        query: Option<String>,

        /// File path for temporal/dependency queries (e.g., src/auth.rs)
        #[arg(long, conflicts_with = "command")]
        file: Option<String>,

        /// Belief ID for grounding queries — find nearest code/commits/sessions (E4.6a)
        #[arg(long, conflicts_with_all = ["command", "file"])]
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
        #[arg(long, value_name = "QUERY_ID", conflicts_with_all = ["command", "query", "file", "belief", "full"])]
        detail: Option<String>,

        /// Rank of the result to fetch (1-indexed, used with --detail)
        #[arg(long, default_value = "1", requires = "detail")]
        rank: usize,

        /// Return full content for all results (escape hatch, deprecated)
        #[arg(long, conflicts_with_all = ["command", "detail"])]
        full: bool,

        /// Use legacy single-oracle search (deprecated, removed in v0.12.0)
        #[arg(long, conflicts_with = "command")]
        legacy: bool,
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
        #[arg(conflicts_with = "command")]
        url: Option<String>,

        /// Enable contribution mode (create fork for PRs)
        #[arg(long, requires = "url")]
        contrib: bool,

        /// Also fetch and index GitHub issues
        #[arg(long, requires = "url")]
        with_issues: bool,
    },

    /// Manage embedding models in mother cache
    Model {
        #[command(subcommand)]
        command: Option<commands::model::ModelCommands>,
    },

    /// The Patina daemon — cross-project knowledge, caching, and routing
    ///
    /// Mother is the always-running daemon that provides hot model caching,
    /// cross-project knowledge access, secrets caching, and graph-based routing.
    Mother {
        #[command(subcommand)]
        command: Option<commands::mother::MotherCommands>,
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

    /// Manage AI adapters
    Adapter {
        #[command(subcommand)]
        command: Option<AdapterCommands>,
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

    /// Manage development sessions (start, update, note, end)
    Session {
        #[command(subcommand)]
        command: commands::session::SessionCommands,
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

    /// Manage spec lifecycle (archive completed specs)
    Spec {
        #[command(subcommand)]
        command: commands::spec::SpecCommands,
    },

    /// Manage fact schemas (install, list, show)
    Schema {
        #[command(subcommand)]
        command: commands::schema::SchemaCommands,
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
    /// Fetch issues and PRs from forge (GitHub, Gitea, etc.)
    Forge {
        /// Full rebuild (ignore incremental)
        #[arg(long)]
        full: bool,

        /// Show sync status without making changes
        #[arg(long)]
        status: bool,

        /// Fork to background and sync all pending refs
        #[arg(long)]
        sync: bool,

        /// Tail the sync log file
        #[arg(long)]
        log: bool,

        /// Foreground sync with limit (escape hatch)
        #[arg(long)]
        limit: Option<usize>,

        /// Target a ref repo instead of current project
        #[arg(long)]
        repo: Option<String>,
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
use commands::adapter::AdapterCommands;
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

    /// Sync adapter templates from resources
    SyncAdapters {
        /// Specific adapter to sync (claude, gemini, etc)
        adapter: Option<String>,

        /// Dry run - show what would change
        #[arg(short, long)]
        dry_run: bool,
    },

    /// Bump component versions
    BumpVersion {
        /// Component to bump (patina, claude-adapter, etc)
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
enum PluginCommands {
    /// List installed plugins
    List,
    /// Run a plugin by name
    Run {
        /// Plugin name (matches <name>.wasm in plugins dir)
        name: String,
        /// Arguments passed to the plugin
        #[arg(last = true)]
        args: Vec<String>,
    },
    /// Create a new plugin project from template
    Init {
        /// Plugin name (valid Rust crate name, e.g. "review-bot")
        name: String,
        /// Plugin world: mother-child, command, task, pipeline
        #[arg(long)]
        world: String,
        /// Build the plugin after scaffolding
        #[arg(long)]
        build: bool,
        /// Build in release mode (requires --build)
        #[arg(long, requires = "build")]
        release: bool,
    },
}

/// Build a query dispatch closure for command plugins.
///
/// Returns None if the plugin has no host_query grants. Otherwise,
/// returns a closure that dispatches to context/scry/assay engines.
fn make_query_dispatch(
    manifest: &patina::plugin::PluginManifest,
) -> Option<patina::plugin::QueryDispatchFn> {
    if manifest.host_query_kinds.is_empty() {
        return None;
    }

    // Lazy QueryEngine — created on first scry call
    let mut query_engine: Option<retrieval::QueryEngine> = None;

    Some(Box::new(move |kind: &str, params: &str| {
        let args: serde_json::Value =
            serde_json::from_str(params).map_err(|e| format!("invalid params: {}", e))?;

        match kind {
            "context" => {
                let topic = args.get("topic").and_then(|v| v.as_str());
                commands::context::get_project_context(topic).map_err(|e| format!("context: {}", e))
            }
            "scry" => {
                let query_str = args.get("query").and_then(|v| v.as_str()).unwrap_or("");
                if query_str.is_empty() {
                    return Err("scry requires 'query' parameter".to_string());
                }
                let limit = args.get("limit").and_then(|v| v.as_u64()).unwrap_or(10) as usize;
                let all_repos = args
                    .get("all_repos")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false);
                let repo = args.get("repo").and_then(|v| v.as_str()).map(String::from);

                let engine = query_engine.get_or_insert_with(retrieval::QueryEngine::new);
                let options = retrieval::QueryOptions { repo, all_repos };
                let results = engine
                    .query_with_options(query_str, limit, &options)
                    .map_err(|e| format!("scry: {}", e))?;

                // JSON array for structured guest consumption
                let json_results: Vec<serde_json::Value> = results
                    .iter()
                    .map(|r| {
                        serde_json::json!({
                            "score": r.fused_score,
                            "doc_id": r.doc_id,
                            "content": r.content,
                        })
                    })
                    .collect();
                serde_json::to_string(&json_results).map_err(|e| format!("serialize: {}", e))
            }
            "assay" => {
                use commands::assay::{AssayOptions, QueryType};

                let query_type = args
                    .get("query_type")
                    .and_then(|v| v.as_str())
                    .unwrap_or("inventory");
                let pattern = args
                    .get("pattern")
                    .and_then(|v| v.as_str())
                    .map(String::from);
                let limit = args.get("limit").and_then(|v| v.as_u64()).unwrap_or(50) as usize;
                let all_repos = args
                    .get("all_repos")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false);
                let repo = args.get("repo").and_then(|v| v.as_str()).map(String::from);
                let query = args.get("query").and_then(|v| v.as_str()).map(String::from);

                let qt = match query_type {
                    "imports" => QueryType::Imports,
                    "importers" => QueryType::Importers,
                    "functions" => QueryType::Functions,
                    "callers" => QueryType::Callers,
                    "callees" => QueryType::Callees,
                    "derive" => QueryType::Derive,
                    "search" => {
                        let q = query.or_else(|| pattern.clone()).unwrap_or_default();
                        if q.is_empty() {
                            return Err("assay search requires 'query' or 'pattern'".into());
                        }
                        QueryType::Search { query: q }
                    }
                    "cochange" => {
                        let file = pattern.clone().unwrap_or_default();
                        if file.is_empty() {
                            return Err("assay cochange requires 'pattern'".into());
                        }
                        QueryType::Cochange { file }
                    }
                    "belief" => {
                        let id = pattern.clone().unwrap_or_default();
                        if id.is_empty() {
                            return Err("assay belief requires 'pattern'".into());
                        }
                        QueryType::Belief { id }
                    }
                    _ => QueryType::Inventory,
                };

                let options = AssayOptions {
                    query_type: qt,
                    pattern,
                    limit,
                    json: true,
                    repo,
                    all_repos,
                    ..Default::default()
                };

                commands::assay::execute_query(&options).map_err(|e| format!("assay: {}", e))
            }
            _ => Err(format!("unknown query kind: {}", kind)),
        }
    }))
}

fn main() -> Result<()> {
    // Run migrations early (before any command)
    patina::migration::migrate_if_needed();
    commands::repo::migrate_registry_paths();

    // Preflight: clean up stale processes before normal operation
    preflight::ensure_clean_state();

    let cli = Cli::parse();

    match cli.command {
        // Launcher mode: no subcommand means launch adapter
        None => {
            let options = commands::launch::LaunchOptions {
                path: None,
                adapter: cli.adapter,
                auto_start_mother: true,
                auto_init: true,
                no_tmux: cli.no_tmux,
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
        Some(Commands::Upgrade { check, json }) => {
            commands::upgrade::execute(check, json)?;
        }
        #[cfg(feature = "dev")]
        Some(Commands::Dev { command }) => match command {
            DevCommands::Validate { json } => {
                commands::dev::validate::execute(json)?;
            }
            DevCommands::Release { bump, dry_run } => {
                commands::dev::release::execute(
                    bump.map(|b| match b {
                        BumpType::Major => "major",
                        BumpType::Minor => "minor",
                        BumpType::Patch => "patch",
                    }),
                    dry_run,
                )?;
            }
            DevCommands::SyncAdapters { adapter, dry_run } => {
                commands::dev::sync_adapters::execute(adapter.as_deref(), dry_run)?;
            }
            DevCommands::BumpVersion {
                component,
                bump_type,
                dry_run,
            } => {
                let bump_str = match bump_type {
                    BumpType::Major => "major",
                    BumpType::Minor => "minor",
                    BumpType::Patch => "patch",
                };
                commands::dev::bump_version::execute(&component, bump_str, dry_run)?;
            }
            DevCommands::UpdateFixtures { fixture } => {
                commands::dev::update_fixtures::execute(fixture.as_deref())?;
            }
        },
        Some(Commands::Scrape { command, rebuild }) => {
            if rebuild {
                commands::scrape::execute_rebuild()?;
            } else {
                match command {
                    None => commands::scrape::execute_all()?,
                    Some(ScrapeCommands::Code { args }) => {
                        commands::scrape::execute_code(args.init, args.force)?
                    }
                    Some(ScrapeCommands::Git { full }) => commands::scrape::execute_git(full)?,
                    Some(ScrapeCommands::Sessions { full }) => {
                        commands::scrape::execute_sessions(full)?
                    }
                    Some(ScrapeCommands::Layer { full }) => commands::scrape::execute_layer(full)?,
                    Some(ScrapeCommands::Forge {
                        full,
                        status,
                        sync,
                        log,
                        limit,
                        repo,
                    }) => commands::scrape::execute_forge(full, status, sync, log, limit, repo)?,
                }
            }
        }
        Some(Commands::Oxidize { repo }) => {
            if let Some(repo_name) = repo {
                commands::oxidize::oxidize_for_repo(&repo_name)?;
            } else {
                commands::oxidize::oxidize()?;
            }
        }
        Some(Commands::Rebuild {
            scrape,
            oxidize,
            force,
            dry_run,
        }) => {
            let options = commands::rebuild::RebuildOptions {
                scrape_only: scrape,
                oxidize_only: oxidize,
                force,
                dry_run,
            };
            commands::rebuild::execute(options)?;
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
            full,
            legacy,
        }) => {
            // Handle subcommands first
            if let Some(subcmd) = command {
                match subcmd {
                    ScryCommands::Orient { path, limit } => {
                        commands::scry::execute_orient(&path, limit)?;
                    }
                    ScryCommands::Recent { query, days, limit } => {
                        commands::scry::execute_recent(query.as_deref(), days, limit)?;
                    }
                    ScryCommands::Why { doc_id, query } => {
                        commands::scry::execute_why(&doc_id, &query)?;
                    }
                    ScryCommands::Open { query_id, rank } => {
                        commands::scry::execute_open(&query_id, rank)?;
                    }
                    ScryCommands::Copy { query_id, rank } => {
                        commands::scry::execute_copy(&query_id, rank)?;
                    }
                    ScryCommands::Feedback {
                        query_id,
                        signal,
                        comment,
                    } => {
                        commands::scry::execute_feedback(&query_id, &signal, comment.as_deref())?;
                    }
                }
            } else if let Some(ref query_id) = detail {
                // D3: --detail mode — fetch full content for one result
                commands::scry::execute_detail(query_id, rank)?;
            } else {
                let options = commands::scry::ScryOptions {
                    limit,
                    min_score,
                    dimension: None,
                    file,
                    repo,
                    all_repos,
                    include_issues,
                    include_persona: !no_persona,
                    explain,
                    belief,
                    content_type,
                    impact,
                    full,
                    legacy,
                };
                commands::scry::execute(query.as_deref(), options)?;
            }
        }
        Some(Commands::Context { topic }) => {
            commands::context::execute(topic.as_deref())?;
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
            if feedback {
                commands::eval::execute_feedback()?;
            } else if nl {
                commands::eval::execute_nl()?;
            } else if assay {
                commands::eval::execute_assay()?;
            } else if scry_raw {
                commands::eval::execute_scry_raw()?;
            } else if scry {
                commands::eval::execute_scry()?;
            } else if combined {
                commands::eval::execute_combined()?;
            } else {
                commands::eval::execute(dimension.map(|d| d.as_str().to_string()))?;
            }
        }
        Some(Commands::Bench { command }) => match command {
            BenchCommands::Grammar { files } => {
                commands::bench::execute_grammar(files)?;
            }
            BenchCommands::Retrieval {
                query_set,
                limit,
                json,
                verbose,
                rrf_k,
                fetch_multiplier,
                oracle,
                repo,
            } => {
                let options = commands::bench::BenchOptions {
                    query_set,
                    limit,
                    json,
                    verbose,
                    rrf_k,
                    fetch_multiplier,
                    oracle,
                    repo,
                };
                commands::bench::execute(options)?;
            }
            BenchCommands::Generate {
                from_commits: _,
                repo,
                limit,
                output,
            } => {
                let options = commands::bench::GenerateOptions {
                    repo,
                    limit,
                    output,
                };
                commands::bench::generate(options)?;
            }
        },
        Some(Commands::Persona { command }) => match command {
            PersonaCommands::Note {
                content,
                domains,
                supersedes,
            } => {
                commands::persona::execute_note(&content, domains, supersedes)?;
            }
            PersonaCommands::Query {
                query,
                limit,
                min_score,
                domains,
            } => {
                commands::persona::execute_query(&query, limit, min_score, domains)?;
            }
            PersonaCommands::List { limit, domains } => {
                commands::persona::execute_list(limit, domains)?;
            }
            PersonaCommands::Materialize => {
                commands::persona::execute_materialize()?;
            }
            PersonaCommands::Status => {
                commands::persona::execute_status()?;
            }
        },
        Some(Commands::Doctor { json }) => {
            let mut args = Vec::new();
            if json {
                args.push("--json".to_string());
            }

            // Try WASM plugin first
            let plugin_wasm = patina::paths::plugin::plugins_dir().join("patina-doctor.wasm");
            let plugin_toml = patina::paths::plugin::plugins_dir().join("patina-doctor.toml");

            let exit_code = if plugin_wasm.exists() {
                let manifest = if plugin_toml.exists() {
                    patina::plugin::PluginEngine::load_manifest(&plugin_toml)?
                } else {
                    // Default manifest for plugins without .toml
                    patina::plugin::PluginManifest {
                        name: "patina-doctor".into(),
                        version: "0.0.0".into(),
                        description: String::new(),
                        world: patina::plugin::PluginWorld::Command,
                        patina_min: "0.0.0".into(),
                        capabilities: vec!["host_log".into(), "host_layer".into()],
                        allowed_toy_commands: vec![],
                        host_query_kinds: vec![],
                        host_http_domains: vec![],
                        host_secrets: std::collections::HashMap::new(),
                        provides: patina::plugin::PluginProvides {
                            child: None,
                            commands: vec!["doctor".into()],
                            ..Default::default()
                        },
                        schemas: std::collections::HashMap::new(),
                    }
                };
                let engine = patina::plugin::CommandEngine::new()?;
                let wasm_bytes = std::fs::read(&plugin_wasm)?;
                let component = engine.load_component(&wasm_bytes)?;
                let query_fn = make_query_dispatch(&manifest);
                engine.run_command(&component, &manifest, &args, query_fn)?
            } else {
                // Fall back to compiled-in doctor
                #[cfg(feature = "bundled-doctor")]
                {
                    commands::doctor::execute(json)?
                }
                #[cfg(not(feature = "bundled-doctor"))]
                {
                    eprintln!("Doctor plugin not installed.");
                    eprintln!("Install: cp patina_doctor.wasm {}", plugin_wasm.display());
                    1
                }
            };

            if exit_code != 0 {
                std::process::exit(exit_code);
            }
        }
        Some(Commands::Plugin { command }) => match command {
            PluginCommands::List => commands::plugin::execute_list()?,
            PluginCommands::Init {
                name,
                world,
                build,
                release,
            } => {
                let world: patina::plugin::PluginWorld = world.parse()?;
                let cwd = std::env::current_dir()?;
                let project_dir = patina::plugin::scaffold::scaffold(&cwd, &name, &world)?;

                let profile = if release { "release" } else { "debug" };
                let artifact = project_dir
                    .join(format!("target/wasm32-wasip2/{}", profile))
                    .join(format!("{}.wasm", name.replace('-', "_")));

                println!("Created {} plugin: {}", world, project_dir.display());
                println!();
                println!("  cd {}", name);
                if release {
                    println!("  cargo build --target wasm32-wasip2 --release");
                } else {
                    println!("  cargo build --target wasm32-wasip2");
                }
                println!();
                println!("Artifact will be at: {}", artifact.display());

                if build {
                    // Proactive rustup check before attempting the build
                    let has_target = std::process::Command::new("rustup")
                        .args(["target", "list", "--installed"])
                        .output()
                        .map(|o| {
                            String::from_utf8_lossy(&o.stdout)
                                .lines()
                                .any(|l| l.trim() == "wasm32-wasip2")
                        })
                        .unwrap_or(false);

                    if !has_target {
                        eprintln!("Missing wasm32-wasip2 target. Install it:");
                        eprintln!("  rustup target add wasm32-wasip2");
                        std::process::exit(1);
                    }

                    println!();
                    println!("Building ({})...", profile);
                    let mut cargo_args = vec!["build", "--target", "wasm32-wasip2"];
                    if release {
                        cargo_args.push("--release");
                    }

                    let status = std::process::Command::new("cargo")
                        .args(&cargo_args)
                        .current_dir(&project_dir)
                        .status();

                    match status {
                        Ok(s) if s.success() => {
                            println!("Built: {}", artifact.display());
                        }
                        Ok(s) => {
                            eprintln!("Build failed (exit code {})", s.code().unwrap_or(-1));
                            std::process::exit(1);
                        }
                        Err(e) => {
                            eprintln!("Failed to run cargo: {}", e);
                            std::process::exit(1);
                        }
                    }
                }
            }
            PluginCommands::Run { name, args } => {
                let plugins_dir = patina::paths::plugin::plugins_dir();
                let wasm_path = plugins_dir.join(format!("{}.wasm", name));
                let toml_path = plugins_dir.join(format!("{}.toml", name));

                if !wasm_path.exists() {
                    anyhow::bail!(
                        "plugin '{}' not found at {}\nInstall: cp {}.wasm {}",
                        name,
                        wasm_path.display(),
                        name,
                        plugins_dir.display()
                    );
                }

                let manifest = if toml_path.exists() {
                    patina::plugin::PluginEngine::load_manifest(&toml_path)?
                } else {
                    anyhow::bail!(
                        "plugin manifest not found at {}\nTask and command plugins require a plugin.toml",
                        toml_path.display()
                    );
                };

                let wasm_bytes = std::fs::read(&wasm_path)?;

                // Auto-detect world from manifest and dispatch
                match &manifest.world {
                    patina::plugin::PluginWorld::Task => {
                        let engine = patina::plugin::TaskEngine::new()?;
                        let component = engine.load_component(&wasm_bytes)?;
                        let query_fn = make_query_dispatch(&manifest);
                        let (exit_code, toys) =
                            engine.run_task(&component, &manifest, &args, query_fn)?;

                        // Execute approved toys
                        for toy in &toys {
                            eprintln!(
                                "[task:{}] executing toy '{}': {} {}",
                                name,
                                toy.name,
                                toy.command,
                                toy.args.join(" ")
                            );
                            let status = std::process::Command::new(&toy.command)
                                .args(&toy.args)
                                .status();
                            match status {
                                Ok(s) if s.success() => {
                                    eprintln!("[task:{}] toy '{}' succeeded", name, toy.name);
                                }
                                Ok(s) => {
                                    eprintln!(
                                        "[task:{}] toy '{}' failed with exit code {}",
                                        name,
                                        toy.name,
                                        s.code().unwrap_or(-1)
                                    );
                                }
                                Err(e) => {
                                    eprintln!(
                                        "[task:{}] toy '{}' failed to execute: {}",
                                        name, toy.name, e
                                    );
                                }
                            }
                        }

                        if exit_code != 0 {
                            std::process::exit(exit_code);
                        }
                    }
                    patina::plugin::PluginWorld::Command => {
                        let engine = patina::plugin::CommandEngine::new()?;
                        let component = engine.load_component(&wasm_bytes)?;
                        let query_fn = make_query_dispatch(&manifest);
                        let exit_code =
                            engine.run_command(&component, &manifest, &args, query_fn)?;
                        if exit_code != 0 {
                            std::process::exit(exit_code);
                        }
                    }
                    other => {
                        anyhow::bail!(
                            "plugin '{}' has world '{}' — only 'task' and 'command' are supported by `plugin run`",
                            name, other

                        );
                    }
                }
            }
        },
        Some(Commands::Repo {
            command,
            url,
            contrib,
            with_issues,
        }) => commands::repo::execute_cli(command, url, contrib, with_issues)?,
        Some(Commands::Model { command }) => commands::model::execute_cli(command)?,
        Some(Commands::Mother { command }) => {
            commands::mother::execute_cli(command, mcp::run_mcp_server)?
        }
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
        Some(Commands::Session { command }) => {
            commands::session::execute(command)?;
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
        Some(Commands::Spec { command }) => match command {
            commands::spec::SpecCommands::Create {
                r#type,
                id,
                title,
                description,
                blocked_by,
                related,
                json,
            } => {
                commands::spec::create(
                    &r#type,
                    &id,
                    title.as_deref(),
                    description.as_deref(),
                    blocked_by,
                    related,
                    json,
                )?;
            }
            commands::spec::SpecCommands::Archive { id, dry_run, stale } => {
                if stale {
                    commands::spec::archive_stale(dry_run)?;
                } else if let Some(id) = id {
                    commands::spec::archive(&id, dry_run)?;
                } else {
                    anyhow::bail!(
                        "Spec ID required. Usage:\n  \
                         patina spec archive <id>\n  \
                         patina spec archive --stale"
                    );
                }
            }
            commands::spec::SpecCommands::Ready { json } => {
                commands::spec::ready(json)?;
            }
            commands::spec::SpecCommands::Blocked { json } => {
                commands::spec::blocked(json)?;
            }
            commands::spec::SpecCommands::List {
                status,
                target,
                json,
            } => {
                commands::spec::list(status, target, json)?;
            }
            commands::spec::SpecCommands::Promote { id, json } => {
                commands::spec::promote(&id, json)?;
            }
            commands::spec::SpecCommands::Complete { id, major, json } => {
                commands::spec::complete(&id, major, json)?;
            }
            commands::spec::SpecCommands::Abandon { id, reason, json } => {
                commands::spec::abandon(&id, reason.as_deref(), json)?;
            }
            commands::spec::SpecCommands::Pause { id, reason, json } => {
                commands::spec::pause(&id, &reason, json)?;
            }
            commands::spec::SpecCommands::Resume { id, force, json } => {
                commands::spec::resume(&id, force, json)?;
            }
            commands::spec::SpecCommands::Block {
                id,
                by,
                reason,
                json,
            } => {
                commands::spec::block(&id, &by, &reason, json)?;
            }
            commands::spec::SpecCommands::Split {
                id,
                new_id,
                description,
                json,
            } => {
                commands::spec::split(&id, new_id.as_deref(), description.as_deref(), json)?;
            }
            commands::spec::SpecCommands::Next { json } => {
                commands::spec::next(json)?;
            }
        },
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
        },
        Some(Commands::Serve { host, port, mcp }) => {
            // Deprecated: delegate to mother start with warning
            eprintln!("Warning: `patina serve` is deprecated, use `patina mother start` instead.");
            if mcp {
                mcp::run_mcp_server()?;
            } else {
                let options = commands::mother::DaemonOptions { host, port };
                commands::mother::daemon::run_server(options)?;
            }
        }
        Some(Commands::Adapter { command }) => commands::adapter::execute(command)?,
        Some(Commands::Report { output, repo, json }) => {
            let options = commands::report::ReportOptions { output, repo, json };
            commands::report::execute(options)?;
        }
        Some(Commands::Assay {
            command,
            pattern,
            limit,
            json,
            repo,
            all_repos,
        }) => {
            let options = match command {
                None => commands::assay::AssayOptions {
                    query_type: commands::assay::QueryType::Inventory,
                    pattern,
                    limit,
                    json,
                    repo,
                    all_repos,
                    ..Default::default()
                },
                Some(AssayCommands::Inventory {
                    pattern,
                    limit,
                    json,
                }) => commands::assay::AssayOptions {
                    query_type: commands::assay::QueryType::Inventory,
                    pattern,
                    limit,
                    json,
                    repo,
                    all_repos,
                    ..Default::default()
                },
                Some(AssayCommands::Imports {
                    module,
                    limit,
                    json,
                }) => commands::assay::AssayOptions {
                    query_type: commands::assay::QueryType::Imports,
                    pattern: Some(module),
                    limit,
                    json,
                    repo,
                    all_repos,
                    ..Default::default()
                },
                Some(AssayCommands::Importers {
                    module,
                    limit,
                    json,
                }) => commands::assay::AssayOptions {
                    query_type: commands::assay::QueryType::Importers,
                    pattern: Some(module),
                    limit,
                    json,
                    repo,
                    all_repos,
                    ..Default::default()
                },
                Some(AssayCommands::Functions {
                    pattern,
                    limit,
                    json,
                }) => commands::assay::AssayOptions {
                    query_type: commands::assay::QueryType::Functions,
                    pattern,
                    limit,
                    json,
                    repo,
                    all_repos,
                    ..Default::default()
                },
                Some(AssayCommands::Callers {
                    function,
                    limit,
                    json,
                }) => commands::assay::AssayOptions {
                    query_type: commands::assay::QueryType::Callers,
                    pattern: Some(function),
                    limit,
                    json,
                    repo,
                    all_repos,
                    ..Default::default()
                },
                Some(AssayCommands::Callees {
                    function,
                    limit,
                    json,
                }) => commands::assay::AssayOptions {
                    query_type: commands::assay::QueryType::Callees,
                    pattern: Some(function),
                    limit,
                    json,
                    repo,
                    all_repos,
                    ..Default::default()
                },
                Some(AssayCommands::Derive { json }) => commands::assay::AssayOptions {
                    query_type: commands::assay::QueryType::Derive,
                    pattern: None,
                    limit: 0,
                    json,
                    repo,
                    all_repos,
                    ..Default::default()
                },
                Some(AssayCommands::DeriveMoments { json }) => commands::assay::AssayOptions {
                    query_type: commands::assay::QueryType::DeriveMoments,
                    pattern: None,
                    limit: 0,
                    json,
                    repo,
                    all_repos,
                    ..Default::default()
                },
                Some(AssayCommands::Search {
                    query: search_query,
                    limit: search_limit,
                    json: search_json,
                    include_issues,
                }) => commands::assay::AssayOptions {
                    query_type: commands::assay::QueryType::Search {
                        query: search_query,
                    },
                    pattern: None,
                    limit: search_limit,
                    json: search_json,
                    repo,
                    all_repos,
                    include_issues,
                },
                Some(AssayCommands::Cochange {
                    file,
                    limit: cochange_limit,
                    json: cochange_json,
                }) => commands::assay::AssayOptions {
                    query_type: commands::assay::QueryType::Cochange { file },
                    pattern: None,
                    limit: cochange_limit,
                    json: cochange_json,
                    repo,
                    all_repos,
                    ..Default::default()
                },
                Some(AssayCommands::Belief {
                    id,
                    limit: belief_limit,
                    json: belief_json,
                }) => commands::assay::AssayOptions {
                    query_type: commands::assay::QueryType::Belief { id },
                    pattern: None,
                    limit: belief_limit,
                    json: belief_json,
                    repo,
                    all_repos,
                    ..Default::default()
                },
            };
            commands::assay::execute(options)?;
        }
    }

    Ok(())
}
