/// Configuration for selecting refactored versions
/// Uses environment variables to switch between original and refactored modules
use std::env;



/// Check if we should use the refactored claude adapter
pub fn use_refactored_claude() -> bool {
    env::var("PATINA_USE_REFACTORED_CLAUDE")
        .map(|v| v == "1" || v.to_lowercase() == "true")
        .unwrap_or(false)
}



/// Print current refactoring configuration (for debugging)
pub fn print_refactor_config() {
    eprintln!("Refactoring configuration:");
    eprintln!("  Claude: {}", if use_refactored_claude() { "refactored" } else { "original" });
}