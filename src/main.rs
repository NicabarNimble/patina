use anyhow::Result;
use clap::{Parser, Subcommand};

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

        /// Design document path
        #[arg(long)]
        design: String,

        /// Development environment (docker, dagger, native)
        #[arg(long)]
        dev: Option<String>,
    },

    /// Check for and install adapter updates or modify project configuration
    Update {
        /// Only check for updates, don't install
        #[arg(short, long)]
        check: bool,

        /// Automatically approve updates (non-interactive)
        #[arg(short, long, conflicts_with = "no")]
        yes: bool,

        /// Automatically decline updates (non-interactive)
        #[arg(short, long, conflicts_with = "yes")]
        no: bool,

        /// Output results as JSON
        #[arg(short, long)]
        json: bool,

        /// Change or add LLM adapter (claude, gemini, local, openai)
        #[arg(long)]
        llm: Option<String>,

        /// Change or add development environment (docker, dagger, native)
        #[arg(long)]
        dev: Option<String>,

        /// Force update even if versions match
        #[arg(short, long)]
        force: bool,
    },

    /// Build project with Docker
    Build,

    /// Run tests in configured environment
    Test,

    /// Check project health and environment
    Doctor {
        /// Only check, don't fix anything
        #[arg(short, long)]
        check: bool,

        /// Automatically fix issues (non-interactive)
        #[arg(short, long, conflicts_with = "check")]
        fix: bool,

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

    /// Manage agent environments
    Agent {
        #[command(subcommand)]
        command: AgentCommands,
    },
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
        Commands::Update {
            check,
            yes,
            no,
            json,
            llm,
            dev,
            force,
        } => {
            let exit_code = commands::update::execute(check, yes, no, json, llm, dev, force)?;
            if exit_code != 0 {
                std::process::exit(exit_code);
            }
        }
        Commands::Build => {
            commands::build::execute()?;
        }
        Commands::Test => {
            commands::test::execute()?;
        }
        Commands::Agent { command } => match command {
            AgentCommands::Start => commands::agent::start()?,
            AgentCommands::Stop => commands::agent::stop()?,
            AgentCommands::Status => commands::agent::status()?,
            AgentCommands::List => commands::agent::list()?,
        },
        Commands::Doctor { check, fix, json } => {
            let exit_code = commands::doctor::execute(check, fix, json)?;
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
