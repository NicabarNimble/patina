use anyhow::Result;
use clap::{Args, Parser, Subcommand};

mod commands;

#[derive(Parser)]
#[command(author, version = env!("CARGO_PKG_VERSION"), about = "Context management for AI-assisted development", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Initialize a new project
    Init {
        /// Project name
        name: String,

        /// LLM to use (claude, gemini, local)
        #[arg(long)]
        llm: String,

        /// Development environment (docker, dagger, native)
        #[arg(long)]
        dev: Option<String>,

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

    /// Ask questions about the codebase
    Ask {
        #[command(flatten)]
        args: commands::ask::AskCommand,
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
}

/// Common arguments for all scrape subcommands
#[derive(Args)]
struct ScrapeArgs {
    /// Initialize the knowledge database
    #[arg(long)]
    init: bool,

    /// Run a custom SQL query against the database
    #[arg(long)]
    query: Option<String>,

    /// Scrape a reference repo from layer/dust/repos/<name>
    #[arg(long)]
    repo: Option<String>,

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

    /// Extract knowledge from markdown/text files (coming soon)
    Docs {
        #[command(flatten)]
        args: ScrapeArgs,
    },

    /// Extract content from PDF documents (coming soon)
    Pdf {
        #[command(flatten)]
        args: ScrapeArgs,
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
        Commands::Init {
            name,
            llm,
            dev,
            force,
            local,
        } => {
            commands::init::execute(name, llm, dev, force, local)?;
        }
        Commands::Upgrade { check, json } => {
            commands::upgrade::execute(check, json)?;
        }
        #[cfg(feature = "dev")]
        Commands::Dev { command } => match command {
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
        Commands::Build => {
            commands::build::execute()?;
        }
        Commands::Test => {
            commands::test::execute()?;
        }
        Commands::Scrape { command } => {
            // Default to Code subcommand with default args for backward compatibility
            let subcommand = command.unwrap_or(ScrapeCommands::Code {
                args: ScrapeArgs {
                    init: false,
                    query: None,
                    repo: None,
                    force: false,
                },
            });
            match subcommand {
                ScrapeCommands::Code { args } => {
                    commands::scrape::execute_code(args.init, args.query, args.repo, args.force)?;
                }
                ScrapeCommands::Docs { args } => {
                    commands::scrape::execute_docs(args.init, args.query, args.repo, args.force)?;
                }
                ScrapeCommands::Pdf { args } => {
                    commands::scrape::execute_pdf(args.init, args.query, args.repo, args.force)?;
                }
            }
        }
        Commands::Embeddings { command } => match command {
            EmbeddingsCommands::Generate { force } => {
                commands::embeddings::generate(force)?;
            }
            EmbeddingsCommands::Status => {
                commands::embeddings::status()?;
            }
        },
        Commands::Query { command } => match command {
            QueryCommands::Semantic {
                query,
                r#type,
                min_score,
                limit,
            } => {
                commands::query::semantic::execute(&query, r#type.clone(), min_score, limit)?;
            }
        },
        Commands::Belief { command } => match command {
            BeliefCommands::Validate {
                query,
                min_score,
                limit,
            } => {
                commands::belief::validate::execute(&query, min_score, limit)?;
            }
        },
        Commands::Doctor {
            json,
            repos,
            update,
            audit,
        } => {
            let exit_code = commands::doctor::execute(json, repos, update, audit)?;
            if exit_code != 0 {
                std::process::exit(exit_code);
            }
        }
        Commands::Ask { args } => {
            commands::ask::run(args)?;
        }
        Commands::Yolo {
            interactive,
            defaults,
            with,
            without,
            json,
        } => {
            commands::yolo::execute(interactive, defaults, with, without, json)?;
        }
        Commands::Version { json, components } => {
            commands::version::execute(json, components)?;
        }
    }

    Ok(())
}
