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
    
    /// Update CLAUDE.md with latest context
    Update,
    
    /// Build project with Docker
    Build,
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
        Commands::Update => {
            commands::update::execute()?;
        }
        Commands::Build => {
            commands::build::execute()?;
        }
    }
    
    Ok(())
}