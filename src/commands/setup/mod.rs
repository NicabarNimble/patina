//! Setup command — first-run convenience for installing default components.
//!
//! Public interface:
//! - `execute_grammars()` — install default grammar plugins to ~/.patina/pipeline/
//!
//! Follows dependable-rust: mod.rs (interface) + grammars.rs (implementation)

mod grammars;

use anyhow::Result;

/// Options for grammar setup
pub struct GrammarOptions {
    /// Show what would be installed without doing it
    pub list: bool,
    /// Install only specific grammars (comma-separated names)
    pub only: Option<Vec<String>>,
    /// Force reinstall even if already present
    pub force: bool,
}

/// Install default grammar plugins to ~/.patina/pipeline/
pub fn execute_grammars(options: GrammarOptions) -> Result<()> {
    grammars::install(options)
}
