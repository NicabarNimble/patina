use anyhow::Result;
use clap::{Args, Parser, Subcommand, ValueEnum};

mod commands;

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

/// LLM frontend for project initialization
#[derive(Clone, Copy, Debug, PartialEq, Eq, ValueEnum)]
pub enum Llm {
    /// Claude Code (Anthropic)
    Claude,
    /// Gemini CLI (Google)
    Gemini,
    /// Codex (OpenAI)
    Codex,
    /// Local LLM
    Local,
}

impl Llm {
    pub fn as_str(&self) -> &'static str {
        match self {
            Llm::Claude => "claude",
            Llm::Gemini => "gemini",
            Llm::Codex => "codex",
            Llm::Local => "local",
        }
    }
}

/// Development environment type
#[derive(Clone, Copy, Debug, PartialEq, Eq, ValueEnum)]
pub enum DevEnv {
    /// Docker containerized builds
    Docker,
    /// Dagger CI/CD pipelines
    Dagger,
    /// Native local development
    Native,
}

impl DevEnv {
    pub fn as_str(&self) -> &'static str {
        match self {
            DevEnv::Docker => "docker",
            DevEnv::Dagger => "dagger",
            DevEnv::Native => "native",
        }
    }
}

#[derive(Parser)]
#[command(author, version = env!("CARGO_PKG_VERSION"), about = "Context management for AI-assisted development", long_about = None)]
struct Cli {
    /// Frontend to launch (claude, gemini, codex). Default: from config.
    #[arg(short = 'f', long = "frontend", global = true)]
    frontend: Option<String>,

    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Initialize a new project
    Init {
        /// Project name
        name: String,

        /// LLM to use (claude, gemini, codex, local)
        #[arg(long, value_enum)]
        llm: Llm,

        /// Development environment (docker, dagger, native)
        #[arg(long, value_enum)]
        dev: Option<DevEnv>,

        /// Force initialization, backup and replace existing patina branch
        #[arg(long)]
        force: bool,

        /// Local-only mode (skip GitHub integration)
        #[arg(long)]
        local: bool,
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

    /// Build project with Docker
    Build,

    /// Run tests in configured environment
    Test,

    /// Check project health and environment
    Doctor {
        /// Output results as JSON
        #[arg(short, long)]
        json: bool,

        /// Check reference repositories in layer/dust/repos
        #[arg(long)]
        repos: bool,

        /// Update stale repositories (requires --repos)
        #[arg(long, requires = "repos")]
        update: bool,

        /// Audit project files and directories for cleanup
        #[arg(long)]
        audit: bool,
    },

    /// Show version information
    Version {
        /// Output as JSON
        #[arg(short, long)]
        json: bool,

        /// Show component versions
        #[arg(short, long)]
        components: bool,
    },

    /// Build semantic knowledge database
    Scrape {
        #[command(subcommand)]
        command: Option<ScrapeCommands>,
    },

    /// Build embeddings and projections from recipe
    Oxidize,

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

    /// Search knowledge base using vector similarity
    Scry {
        /// Query text to search for (optional if --file is provided)
        query: Option<String>,

        /// File path for temporal/dependency queries (e.g., src/auth.rs)
        #[arg(long)]
        file: Option<String>,

        /// Maximum number of results (default: 10)
        #[arg(long, default_value = "10")]
        limit: usize,

        /// Minimum similarity score (0.0-1.0, default: 0.0)
        #[arg(long, default_value = "0.0")]
        min_score: f32,

        /// Dimension to search (semantic, temporal, dependency)
        #[arg(long, value_enum)]
        dimension: Option<Dimension>,

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
    },

    /// Evaluate retrieval quality across dimensions
    Eval {
        /// Specific dimension to evaluate (semantic, temporal)
        #[arg(long, value_enum)]
        dimension: Option<Dimension>,
    },

    /// Generate and manage semantic embeddings
    Embeddings {
        #[command(subcommand)]
        command: EmbeddingsCommands,
    },

    /// Query observations and beliefs using semantic search
    Query {
        #[command(subcommand)]
        command: QueryCommands,
    },

    /// Validate beliefs using neuro-symbolic reasoning
    Belief {
        #[command(subcommand)]
        command: BeliefCommands,
    },

    /// Cross-project user knowledge (preferences, style, history)
    Persona {
        #[command(subcommand)]
        command: PersonaCommands,
    },

    /// Ask questions about the codebase
    Ask {
        #[command(flatten)]
        args: commands::ask::AskCommand,
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

    /// Start the Mothership daemon (Ollama-style HTTP server)
    Serve {
        /// Host to bind to (default: 127.0.0.1, use 0.0.0.0 for container access)
        #[arg(long, default_value = "127.0.0.1")]
        host: String,

        /// Port to bind to
        #[arg(long, default_value = "50051")]
        port: u16,
    },

    /// Open project in AI frontend (like 'code .' for VS Code)
    Launch {
        /// Project path (default: current directory)
        path: Option<String>,

        /// Frontend to use (claude, gemini, codex)
        #[arg(short, long)]
        frontend: Option<String>,

        /// Don't auto-start mothership
        #[arg(long)]
        no_serve: bool,
    },

    /// List available AI frontends
    Adapter {
        #[command(subcommand)]
        command: Option<AdapterCommands>,
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
    /// Extract sessions, goals, and observations from session files
    Sessions {
        /// Full rebuild (ignore incremental)
        #[arg(long)]
        full: bool,
    },
}

#[derive(Subcommand)]
enum EmbeddingsCommands {
    /// Generate embeddings for all beliefs and observations
    Generate {
        /// Force regeneration of all embeddings
        #[arg(long)]
        force: bool,
    },

    /// Show embedding coverage status
    Status,
}

#[derive(Subcommand)]
enum QueryCommands {
    /// Search observations using semantic similarity
    Semantic {
        /// Query text to search for
        query: String,

        /// Filter by observation types (comma-separated: pattern,technology,decision,challenge)
        #[arg(long, value_delimiter = ',')]
        r#type: Option<Vec<String>>,

        /// Minimum similarity score (0.0-1.0, default: 0.35)
        #[arg(long, default_value = "0.35")]
        min_score: f32,

        /// Maximum number of results (default: 10)
        #[arg(long, default_value = "10")]
        limit: usize,
    },
}

#[derive(Subcommand)]
enum BeliefCommands {
    /// Validate a belief using semantic evidence and symbolic reasoning
    Validate {
        /// Belief statement to validate
        query: String,

        /// Minimum similarity score for evidence (0.0-1.0, default: 0.50)
        #[arg(long, default_value = "0.50")]
        min_score: f32,

        /// Maximum number of observations to consider (default: 20)
        #[arg(long, default_value = "20")]
        limit: usize,
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

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        // Launcher mode: no subcommand means launch frontend
        None => {
            let options = commands::launch::LaunchOptions {
                path: None,
                frontend: cli.frontend,
                auto_start_mothership: true,
                auto_init: true,
            };
            commands::launch::execute(options)?;
        }

        Some(Commands::Init {
            name,
            llm,
            dev,
            force,
            local,
        }) => {
            commands::init::execute(
                name,
                llm.as_str().to_string(),
                dev.map(|d| d.as_str().to_string()),
                force,
                local,
            )?;
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
        Some(Commands::Build) => {
            commands::build::execute()?;
        }
        Some(Commands::Test) => {
            commands::test::execute()?;
        }
        Some(Commands::Scrape { command }) => match command {
            None => commands::scrape::execute_all()?,
            Some(ScrapeCommands::Code { args }) => {
                commands::scrape::execute_code(args.init, args.force)?
            }
            Some(ScrapeCommands::Git { full }) => commands::scrape::execute_git(full)?,
            Some(ScrapeCommands::Sessions { full }) => commands::scrape::execute_sessions(full)?,
        },
        Some(Commands::Oxidize) => {
            commands::oxidize::oxidize()?;
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
            query,
            file,
            limit,
            min_score,
            dimension,
            repo,
            all_repos,
            include_issues,
            no_persona,
        }) => {
            let options = commands::scry::ScryOptions {
                limit,
                min_score,
                dimension: dimension.map(|d| d.as_str().to_string()),
                file,
                repo,
                all_repos,
                include_issues,
                include_persona: !no_persona,
            };
            commands::scry::execute(query.as_deref(), options)?;
        }
        Some(Commands::Eval { dimension }) => {
            commands::eval::execute(dimension.map(|d| d.as_str().to_string()))?;
        }
        Some(Commands::Embeddings { command }) => match command {
            EmbeddingsCommands::Generate { force } => {
                commands::embeddings::generate(force)?;
            }
            EmbeddingsCommands::Status => {
                commands::embeddings::status()?;
            }
        },
        Some(Commands::Query { command }) => match command {
            QueryCommands::Semantic {
                query,
                r#type,
                min_score,
                limit,
            } => {
                commands::query::semantic::execute(&query, r#type.clone(), min_score, limit)?;
            }
        },
        Some(Commands::Belief { command }) => match command {
            BeliefCommands::Validate {
                query,
                min_score,
                limit,
            } => {
                commands::belief::validate::execute(&query, min_score, limit)?;
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
        },
        Some(Commands::Doctor {
            json,
            repos,
            update,
            audit,
        }) => {
            let exit_code = commands::doctor::execute(json, repos, update, audit)?;
            if exit_code != 0 {
                std::process::exit(exit_code);
            }
        }
        Some(Commands::Ask { args }) => {
            commands::ask::run(args)?;
        }
        Some(Commands::Repo {
            command,
            url,
            contrib,
            with_issues,
        }) => commands::repo::execute_cli(command, url, contrib, with_issues)?,
        Some(Commands::Yolo {
            interactive,
            defaults,
            with,
            without,
            json,
        }) => {
            commands::yolo::execute(interactive, defaults, with, without, json)?;
        }
        Some(Commands::Version { json, components }) => {
            commands::version::execute(json, components)?;
        }
        Some(Commands::Serve { host, port }) => {
            let options = commands::serve::ServeOptions { host, port };
            commands::serve::execute(options)?;
        }
        Some(Commands::Launch {
            path,
            frontend,
            no_serve,
        }) => {
            let options = commands::launch::LaunchOptions {
                path,
                frontend,
                auto_start_mothership: !no_serve,
                auto_init: true,
            };
            commands::launch::execute(options)?;
        }
        Some(Commands::Adapter { command }) => commands::adapter::execute(command)?,
    }

    Ok(())
}
