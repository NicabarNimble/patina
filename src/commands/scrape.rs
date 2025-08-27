use anyhow::Result;
use std::env;

pub fn run(
    init: bool,
    query: Option<String>,
    repo: Option<String>,
    force: bool,
) -> Result<()> {
    // Check environment variable to determine which implementation to use
    let use_new_scrape = env::var("PATINA_NEW_SCRAPE")
        .unwrap_or_else(|_| "false".to_string())
        .parse::<bool>()
        .unwrap_or(false);

    if use_new_scrape {
        // Use new modular implementation
        eprintln!("Using new modular scrape implementation");
        patina::scrape::execute(init, query, repo, force)
    } else {
        // Use old implementation (default for now)
        eprintln!("Using legacy scrape implementation (set PATINA_NEW_SCRAPE=true for new version)");
        super::scrape_old::execute(init, query, repo, force)
    }
}