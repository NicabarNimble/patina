use anyhow::Result;
use clap::{Parser, Subcommand};
use patina::indexer;

mod commands;
mod config;

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

        /// Design document path
        #[arg(long, default_value = "PROJECT_DESIGN.toml")]
        design: String,

        /// Development environment (docker, dagger, native)
        #[arg(long)]
        dev: Option<String>,
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

    /// Navigate patterns using semantic search
    Navigate {
        /// Search query
        query: String,

        /// Search across all branches (not just current)
        #[arg(short, long)]
        all_branches: bool,

        /// Filter by layer (core, surface, dust)
        #[arg(short, long)]
        layer: Option<String>,

        /// Output as JSON
        #[arg(short, long)]
        json: bool,
    },

    /// Organize and clean up patterns
    Organize(commands::organize::OrganizeArgs),
    
    /// Organize patterns using Git history (v2)
    OrganizeV2(commands::organize_v2::OrganizeArgs),
    
    /// Analyze session activity and patterns
    SessionAnalyze(commands::session_analyze::SessionAnalyzeArgs),

    /// Manage agent environments
    Agent {
        #[command(subcommand)]
        command: AgentCommands,
    },

    /// Process hooks from LLMs (Claude, Gemini, etc)
    Hook {
        /// Hook event name (on-stop, on-modified, on-before-edit, on-session-start)
        event: String,
    },

    /// Trace ideas through their implementation lifecycle
    Trace {
        /// Pattern/idea name to trace
        pattern: String,
    },

    /// Recognize patterns in surviving code
    Recognize,

    /// Connect ideas to their implementations
    Connect,
    
    /// Analyze Git metrics and code evolution
    Metrics(commands::metrics::MetricsArgs),
}

#[derive(Subcommand)]
enum AgentCommands {
    /// Start the agent environment service
    Start,

    /// Stop the agent environment service
    Stop,

    /// Show agent service status
    Status,

    /// List active agent environments
    List,
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
            design,
            dev,
        } => {
            commands::init::execute(name, llm, design, dev)?;
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
        Commands::Navigate {
            query,
            all_branches,
            layer,
            json,
        } => {
            commands::navigate::execute(&query, all_branches, layer, json)?;
        }
        Commands::Organize(args) => {
            let config = config::Config::load()?;
            commands::organize::execute(&config, args)?;
        }
        Commands::OrganizeV2(args) => {
            commands::organize_v2::execute(args)?;
        }
        Commands::SessionAnalyze(args) => {
            commands::session_analyze::execute(args)?;
        }
        Commands::Agent { command } => match command {
            AgentCommands::Start => commands::agent::start()?,
            AgentCommands::Stop => commands::agent::stop()?,
            AgentCommands::Status => commands::agent::status()?,
            AgentCommands::List => commands::agent::list()?,
        },
        Commands::Hook { event } => {
            commands::hook::process_hook(&event)?;
        }
        Commands::Trace { pattern } => {
            commands::trace::execute(&pattern)?;
        }
        Commands::Recognize => {
            commands::recognize::execute()?;
        }
        Commands::Connect => {
            commands::connect::execute()?;
        }
        Commands::Metrics(args) => {
            commands::metrics::execute(args)?;
        }
        Commands::Doctor { json } => {
            let exit_code = commands::doctor::execute(json)?;
            if exit_code != 0 {
                std::process::exit(exit_code);
            }
        }
        Commands::Version { json, components } => {
            commands::version::execute(json, components)?;
        }
    }

    Ok(())
}
