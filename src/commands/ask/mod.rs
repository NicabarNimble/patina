use anyhow::Result;
use clap::Parser;
use std::path::PathBuf;

mod patterns;

#[derive(Parser, Debug)]
pub struct AskCommand {
    /// Query to ask about the codebase
    #[arg(help = "What to ask (e.g., 'naming patterns', 'error handling', 'architecture')")]
    query: String,
    
    /// Path to the database (defaults to .patina/knowledge.db)
    #[arg(short, long)]
    db: Option<PathBuf>,
    
    /// Repository to query (if using repo-specific database)
    #[arg(short, long)]
    repo: Option<String>,
}

pub fn run(cmd: AskCommand) -> Result<()> {
    // Determine database path
    let db_path = if let Some(db) = cmd.db {
        db
    } else if let Some(repo) = &cmd.repo {
        PathBuf::from(format!("layer/dust/repos/{}.db", repo))
    } else {
        PathBuf::from(".patina/knowledge.db")
    };
    
    if !db_path.exists() {
        anyhow::bail!("Database not found at {:?}. Run 'patina scrape code' first.", db_path);
    }
    
    println!("📊 Analyzing codebase from {:?}...\n", db_path);
    
    // Route queries to appropriate handlers
    match cmd.query.to_lowercase().as_str() {
        q if q.contains("pattern") || q.contains("naming") => {
            patterns::analyze_naming_patterns(&db_path)?;
        }
        q if q.contains("convention") || q.contains("style") => {
            patterns::analyze_conventions(&db_path)?;
        }
        q if q.contains("architect") || q.contains("structure") => {
            patterns::analyze_architecture(&db_path)?;
        }
        q if q.contains("error") => {
            patterns::analyze_error_handling(&db_path)?;
        }
        _ => {
            println!("🤔 I can help with:");
            println!("  • naming patterns");
            println!("  • code conventions");
            println!("  • architecture/structure");
            println!("  • error handling");
            println!("\nTry: patina ask 'naming patterns'");
        }
    }
    
    Ok(())
}