// New scrape command router - handles subcommands
// This will replace scrape.rs once we're done

use anyhow::{bail, Result};

mod scrape;
use scrape::ScrapeConfig;

/// Execute scrape command with subcommands
pub fn execute(
    subcommand: Option<&str>,
    init: bool, 
    query: Option<String>, 
    repo: Option<String>, 
    force: bool
) -> Result<()> {
    // Default to "code" for backward compatibility
    let subcommand = subcommand.unwrap_or("code");
    
    match subcommand {
        "code" => {
            // Delegate to code scraper (the refactored 2456-line beast)
            let mut config = ScrapeConfig::new(force);
            if let Some(r) = repo.as_ref() {
                config.for_repo(r);
            }
            
            if init {
                scrape::code::initialize(&config)?;
            } else if let Some(q) = query {
                // Query should move to Ask command eventually
                scrape::code::query(&config, &q)?;
            } else {
                scrape::code::extract(&config)?;
            }
            Ok(())
        }
        
        "docs" => {
            bail!("Document extraction not yet implemented. Use 'scrape code' for now.");
        }
        
        "pdf" => {
            bail!("PDF extraction not yet implemented. Use 'scrape code' for now.");
        }
        
        _ => {
            bail!("Unknown scrape subcommand: {}. Available: code, docs, pdf", subcommand);
        }
    }
}

/// List available scrape subcommands (for help text)
pub fn list_subcommands() -> Vec<(&'static str, &'static str)> {
    vec![
        ("code", "Extract semantic information from source code"),
        ("docs", "Extract knowledge from markdown/text files (coming soon)"),
        ("pdf", "Extract content from PDF documents (coming soon)"),
    ]
}