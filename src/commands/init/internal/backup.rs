//! Backup functionality for re-initialization

use anyhow::{Context, Result};
use std::fs;
use std::path::Path;

/// Backup gitignored directories before re-initialization
pub fn backup_gitignored_dirs() -> Result<()> {
    let timestamp = chrono::Utc::now().format("%Y%m%d_%H%M%S");
    
    // Backup .claude directory if it exists
    if Path::new(".claude").exists() {
        // Create .backup directory if it doesn't exist
        let backup_root = Path::new(".backup");
        fs::create_dir_all(backup_root)
            .context("Failed to create .backup directory")?;
        
        // Create the backup destination
        let backup_name = format!("claude_{}", timestamp);
        let backup_path = backup_root.join(&backup_name);
        
        println!("📦 Backing up .claude to .backup/{}", backup_name);
        
        // Move the entire .claude directory to the backup location
        fs::rename(".claude", &backup_path)
            .context("Failed to move .claude to backup")?;
    }
    
    // Could add more directories here in the future
    // e.g., backup .patina/sessions if needed
    
    Ok(())
}