/// Configuration for selecting refactored versions
/// Uses environment variables to switch between original and refactored modules
use std::env;

/// Check if we should use the refactored workspace client
pub fn use_refactored_workspace() -> bool {
    env::var("PATINA_USE_REFACTORED_WORKSPACE")
        .map(|v| v == "1" || v.to_lowercase() == "true")
        .unwrap_or(false)
}


/// Check if we should use the refactored claude adapter
pub fn use_refactored_claude() -> bool {
    env::var("PATINA_USE_REFACTORED_CLAUDE")
        .map(|v| v == "1" || v.to_lowercase() == "true")
        .unwrap_or(false)
}


/// Check if we should use the refactored dagger environment
pub fn use_refactored_dagger() -> bool {
    env::var("PATINA_USE_REFACTORED_DAGGER")
        .map(|v| v == "1" || v.to_lowercase() == "true")
        .unwrap_or(false)
}

/// Print current refactoring configuration (for debugging)
pub fn print_refactor_config() {
    eprintln!("Refactoring configuration:");
    eprintln!("  Workspace: {}", if use_refactored_workspace() { "refactored" } else { "original" });
    eprintln!("  Claude: {}", if use_refactored_claude() { "refactored" } else { "original" });
    eprintln!("  Dagger: {}", if use_refactored_dagger() { "refactored" } else { "original" });
}