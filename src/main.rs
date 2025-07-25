use clap::{Parser, Subcommand};
use anyhow::Result;

mod commands;

#[derive(Parser)]
#[command(author, version, about = "Context management for AI-assisted development", long_about = None)]
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
        
        /// Development environment (docker, native)
        #[arg(long)]
        dev: String,
    },
    
    /// Add a pattern to the current session
    Add {
        /// Pattern type (pattern, decision, etc)
        #[arg(value_enum)]
        type_: String,
        
        /// Pattern name
        name: String,
    },
    
    /// Commit session patterns to brain
    Commit {
        /// Commit message
        #[arg(short, long)]
        message: String,
    },
    
    /// Generate context for LLM
    Push,
    
    /// Check for and install adapter updates
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
    },
    
    /// Build project with Docker
    Build,
    
    /// Run agent workflows with Dagger
    Agent {
        /// Subcommand (workspace, test, shell)
        #[arg(value_name = "COMMAND")]
        command: Option<String>,
    },
    
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
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    
    match cli.command {
        Commands::Init { name, llm, design, dev } => {
            commands::init::execute(name, llm, design, dev)?;
        }
        Commands::Add { type_, name } => {
            commands::add::execute(type_, name)?;
        }
        Commands::Commit { message } => {
            commands::commit::execute(message)?;
        }
        Commands::Push => {
            commands::push::execute()?;
        }
        Commands::Update { check, yes, no, json } => {
            let exit_code = commands::update::execute(check, yes, no, json)?;
            if exit_code != 0 {
                std::process::exit(exit_code);
            }
        }
        Commands::Build => {
            commands::build::execute()?;
        }
        Commands::Agent { command } => {
            commands::agent::execute(command)?;
        }
        Commands::Doctor { check, fix, json } => {
            let exit_code = commands::doctor::execute(check, fix, json)?;
            if exit_code != 0 {
                std::process::exit(exit_code);
            }
        }
    }
    
    Ok(())
}