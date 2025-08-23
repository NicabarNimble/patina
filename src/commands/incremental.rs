use anyhow::{Context, Result};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::process::Command;

/// Tracks file changes for incremental updates
#[derive(Debug)]
pub struct FileChanges {
    pub new_files: Vec<PathBuf>,
    pub modified_files: Vec<PathBuf>,
    pub deleted_files: Vec<String>,
    pub unchanged_files: Vec<PathBuf>,
}

impl FileChanges {
    pub fn is_empty(&self) -> bool {
        self.new_files.is_empty() && self.modified_files.is_empty() && self.deleted_files.is_empty()
    }
    
    pub fn total_changes(&self) -> usize {
        self.new_files.len() + self.modified_files.len() + self.deleted_files.len()
    }
}

/// Detect which files have changed since last index
pub fn detect_changes(db_path: &str, current_files: &HashMap<PathBuf, i64>) -> Result<FileChanges> {
    // Query existing index state using stdin to get clean output
    let query = "SELECT path || '|' || CAST(mtime AS VARCHAR) FROM index_state;";
    let mut child = Command::new("duckdb")
        .arg(db_path)
        .arg("-noheader")
        .arg("-list")
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .context("Failed to start DuckDB")?;
    
    if let Some(mut stdin) = child.stdin.take() {
        use std::io::Write;
        stdin.write_all(query.as_bytes()).context("Failed to write query")?;
    }
    
    let output = child.wait_with_output().context("Failed to query index_state")?;
    
    if !output.status.success() {
        // Table might not exist on first run
        return Ok(FileChanges {
            new_files: current_files.keys().cloned().collect(),
            modified_files: vec![],
            deleted_files: vec![],
            unchanged_files: vec![],
        });
    }
    
    // Parse existing index state
    let mut indexed_files = HashMap::new();
    let stdout = String::from_utf8_lossy(&output.stdout);
    for line in stdout.lines() {
        if let Some((path, mtime_str)) = line.split_once('|') {
            if let Ok(mtime) = mtime_str.parse::<i64>() {
                indexed_files.insert(path.to_string(), mtime);
            }
        }
    }
    
    let mut changes = FileChanges {
        new_files: vec![],
        modified_files: vec![],
        deleted_files: vec![],
        unchanged_files: vec![],
    };
    
    // Check for new and modified files
    for (path, &mtime) in current_files {
        let path_str = path.to_string_lossy().to_string();
        match indexed_files.get(&path_str) {
            None => changes.new_files.push(path.clone()),
            Some(&old_mtime) if old_mtime != mtime => changes.modified_files.push(path.clone()),
            Some(_) => changes.unchanged_files.push(path.clone()),
        }
    }
    
    // Check for deleted files
    let current_paths: HashSet<String> = current_files
        .keys()
        .map(|p| p.to_string_lossy().to_string())
        .collect();
    
    for indexed_path in indexed_files.keys() {
        if !current_paths.contains(indexed_path) {
            changes.deleted_files.push(indexed_path.clone());
        }
    }
    
    Ok(changes)
}

/// Clean up data for modified and deleted files
pub fn cleanup_changed_files(db_path: &str, changes: &FileChanges) -> Result<()> {
    if changes.modified_files.is_empty() && changes.deleted_files.is_empty() {
        return Ok(());
    }
    
    let mut sql = String::new();
    
    // Build list of paths to clean
    let mut paths_to_clean = Vec::new();
    for path in &changes.modified_files {
        paths_to_clean.push(format!("'{}'", path.to_string_lossy()));
    }
    for path in &changes.deleted_files {
        paths_to_clean.push(format!("'{}'", path));
    }
    
    if !paths_to_clean.is_empty() {
        let path_list = paths_to_clean.join(", ");
        
        // Delete from all relevant tables
        sql.push_str(&format!(
            "DELETE FROM code_fingerprints WHERE path IN ({});\n",
            path_list
        ));
        sql.push_str(&format!(
            "DELETE FROM code_search WHERE path IN ({});\n",
            path_list
        ));
        sql.push_str(&format!(
            "DELETE FROM index_state WHERE path IN ({});\n",
            path_list
        ));
    }
    
    // Execute cleanup
    if !sql.is_empty() {
        let mut child = Command::new("duckdb")
            .arg(db_path)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .context("Failed to start DuckDB")?;
        
        if let Some(mut stdin) = child.stdin.take() {
            use std::io::Write;
            stdin.write_all(sql.as_bytes()).context("Failed to write cleanup SQL")?;
        }
        
        let output = child.wait_with_output().context("Failed to execute cleanup")?;
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("Cleanup failed: {}", stderr);
        }
    }
    
    Ok(())
}

/// Print change summary
pub fn print_change_summary(changes: &FileChanges) {
    if changes.is_empty() {
        println!("  ✓ No changes detected - index is up to date");
    } else {
        if !changes.new_files.is_empty() {
            println!("  📄 {} new files", changes.new_files.len());
        }
        if !changes.modified_files.is_empty() {
            println!("  ✏️  {} modified files", changes.modified_files.len());
        }
        if !changes.deleted_files.is_empty() {
            println!("  🗑️  {} deleted files", changes.deleted_files.len());
        }
        println!("  📊 {} unchanged files", changes.unchanged_files.len());
    }
}