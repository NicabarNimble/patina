use anyhow::{Context, Result};
use patina::session::SessionManager;

pub fn execute(type_: String, name: String) -> Result<()> {
    // Find project root
    let project_root = SessionManager::find_project_root()
        .context("Not in a Patina project directory. Run 'patina init' first.")?;

    // Get session manager
    let session_manager = SessionManager::new(&project_root);

    // Get or create current session
    let mut session = session_manager.get_or_create_session()?;

    // Validate pattern type
    let valid_types = [
        "core",
        "topic",
        "project",
        "decision",
        "constraint",
        "principle",
    ];
    if !valid_types.contains(&type_.as_str()) {
        anyhow::bail!(
            "Invalid pattern type '{}'. Valid types: {:?}",
            type_,
            valid_types
        );
    }

    // Add pattern to session
    session.add_pattern(type_.clone(), name.clone());

    // Save session
    session_manager.save_session(&session)?;

    println!("✓ Added {type_} '{name}' to current session");
    println!("  Patterns in session: {}", session.patterns.len());
    println!(
        "  Uncommitted patterns: {}",
        session.uncommitted_patterns().len()
    );

    Ok(())
}
