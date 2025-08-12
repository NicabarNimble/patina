//! Backup functionality for re-initialization

use anyhow::{Context, Result};
use std::fs;
use std::path::{Path, PathBuf};

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
        
        // First, preserve active session files if they exist
        let session_files_to_preserve = vec![
            ".claude/context/active-session.md",
            ".claude/context/last-session.md",
        ];
        
        let mut preserved_files = Vec::new();
        for file in &session_files_to_preserve {
            if Path::new(file).exists() {
                let temp_path = format!("{}.preserve", file);
                fs::copy(file, &temp_path)?;
                preserved_files.push((file.to_string(), temp_path));
            }
        }
        
        // Move the entire .claude directory to the backup location
        fs::rename(".claude", &backup_path)
            .context("Failed to move .claude to backup")?;
        
        // After init creates new .claude, we'll restore these files
        // Store paths for restoration
        if !preserved_files.is_empty() {
            // Create marker file for init to know there are files to restore
            fs::write(".claude_session_restore", "pending")?;
        }
    }
    
    Ok(())
}

/// Restore preserved session files after init
pub fn restore_session_files() -> Result<()> {
    if Path::new(".claude_session_restore").exists() {
        // Ensure context directory exists
        fs::create_dir_all(".claude/context")?;
        
        // Restore preserved files
        let files_to_restore = vec![
            (".claude/context/active-session.md.preserve", ".claude/context/active-session.md"),
            (".claude/context/last-session.md.preserve", ".claude/context/last-session.md"),
        ];
        
        for (preserved, target) in files_to_restore {
            if Path::new(preserved).exists() {
                fs::rename(preserved, target)?;
                println!("  ✓ Restored session file: {}", target);
            }
        }
        
        // Remove marker file
        fs::remove_file(".claude_session_restore")?;
    }
    
    Ok(())
}