//! Migration module - handles data migration from old paths to new paths.
//!
//! This module is separate from paths.rs following unix philosophy:
//! - paths.rs: defines WHERE data lives (pure, no I/O)
//! - migration.rs: moves data from old to new locations (impure, one-time)
//!
//! Called early in startup to ensure data is in the right place.

use std::fs;
use std::path::{Path, PathBuf};

use crate::paths;

/// Check for old paths and migrate to new cache structure if needed.
///
/// Migrations:
/// - ~/.patina/personas/default/materialized/ -> ~/.patina/cache/personas/default/
/// - ~/.patina/repos/ -> ~/.patina/cache/repos/
///
/// This function is idempotent - safe to call multiple times.
pub fn migrate_if_needed() {
    // Only run if patina home exists (not first run)
    if !paths::patina_home().exists() {
        return;
    }

    let mut migrated = false;

    // Migrate persona materialized data
    if migrate_persona_cache() {
        migrated = true;
    }

    // Migrate repos
    if migrate_repos_cache() {
        migrated = true;
    }

    if migrated {
        println!();
    }
}

/// Migrate persona materialized data to cache
fn migrate_persona_cache() -> bool {
    let old_path = paths::patina_home()
        .join("personas")
        .join("default")
        .join("materialized");

    let new_path = paths::persona::cache_dir();

    if !old_path.exists() {
        return false;
    }

    // If new path already exists with data, skip
    if new_path.exists() && new_path.join("persona.db").exists() {
        // Clean up old path
        if let Err(e) = fs::remove_dir_all(&old_path) {
            eprintln!(
                "Warning: Could not remove old materialized dir: {} ({})",
                old_path.display(),
                e
            );
        }
        return false;
    }

    println!("📦 Migrating persona data to new cache location...");

    // Create parent directories
    if let Some(parent) = new_path.parent() {
        if let Err(e) = fs::create_dir_all(parent) {
            eprintln!("Warning: Could not create cache directory: {}", e);
            return false;
        }
    }

    // Move the directory
    match fs::rename(&old_path, &new_path) {
        Ok(_) => {
            println!(
                "   ✓ Moved {} -> {}",
                old_path.display(),
                new_path.display()
            );
            true
        }
        Err(e) => {
            // Rename might fail across filesystems, try copy + delete
            if let Err(copy_err) = copy_dir_recursive(&old_path, &new_path) {
                eprintln!(
                    "Warning: Could not migrate persona cache: {} (copy: {})",
                    e, copy_err
                );
                return false;
            }
            if let Err(rm_err) = fs::remove_dir_all(&old_path) {
                eprintln!(
                    "Warning: Migrated but could not remove old path: {}",
                    rm_err
                );
            }
            println!(
                "   ✓ Moved {} -> {}",
                old_path.display(),
                new_path.display()
            );
            true
        }
    }
}

/// Migrate repos to cache
fn migrate_repos_cache() -> bool {
    let old_path = paths::patina_home().join("repos");
    let new_path = paths::repos::cache_dir();

    if !old_path.exists() {
        return false;
    }

    // Check if old path has any repos
    let has_repos = fs::read_dir(&old_path)
        .map(|entries| entries.count() > 0)
        .unwrap_or(false);

    if !has_repos {
        // Empty directory, just remove it
        let _ = fs::remove_dir(&old_path);
        return false;
    }

    // If new path already has repos, merge by moving individual repos
    if new_path.exists() {
        return migrate_repos_merge(&old_path, &new_path);
    }

    println!("📦 Migrating repos to new cache location...");

    // Create parent directories
    if let Some(parent) = new_path.parent() {
        if let Err(e) = fs::create_dir_all(parent) {
            eprintln!("Warning: Could not create cache directory: {}", e);
            return false;
        }
    }

    // Move the directory
    match fs::rename(&old_path, &new_path) {
        Ok(_) => {
            println!(
                "   ✓ Moved {} -> {}",
                old_path.display(),
                new_path.display()
            );
            true
        }
        Err(e) => {
            eprintln!("Warning: Could not migrate repos cache: {}", e);
            false
        }
    }
}

/// Merge repos when both old and new paths exist
fn migrate_repos_merge(old_path: &Path, new_path: &Path) -> bool {
    let mut migrated_any = false;

    if let Ok(entries) = fs::read_dir(old_path) {
        for entry in entries.flatten() {
            let entry_name = entry.file_name();
            let old_repo = old_path.join(&entry_name);
            let new_repo = new_path.join(&entry_name);

            if new_repo.exists() {
                // Already exists in new location, skip
                continue;
            }

            if !migrated_any {
                println!("📦 Migrating remaining repos to cache...");
                migrated_any = true;
            }

            if let Err(e) = fs::rename(&old_repo, &new_repo) {
                eprintln!(
                    "Warning: Could not move repo {}: {}",
                    entry_name.to_string_lossy(),
                    e
                );
            } else {
                println!("   ✓ Moved {}", entry_name.to_string_lossy());
            }
        }
    }

    // Clean up old repos dir if empty
    if let Ok(entries) = fs::read_dir(old_path) {
        if entries.count() == 0 {
            let _ = fs::remove_dir(old_path);
        }
    }

    migrated_any
}

/// Recursively copy a directory
fn copy_dir_recursive(src: &PathBuf, dest: &PathBuf) -> std::io::Result<()> {
    fs::create_dir_all(dest)?;

    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let file_type = entry.file_type()?;
        let src_path = entry.path();
        let dest_path = dest.join(entry.file_name());

        if file_type.is_dir() {
            copy_dir_recursive(&src_path, &dest_path)?;
        } else {
            fs::copy(&src_path, &dest_path)?;
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_migrate_if_needed_no_op_on_missing_home() {
        // Just verify it doesn't panic when home doesn't exist
        // (can't easily test with real paths)
        migrate_if_needed();
    }

    #[test]
    fn test_copy_dir_recursive() {
        let temp = TempDir::new().unwrap();
        let src = temp.path().join("src");
        let dest = temp.path().join("dest");

        // Create source structure
        fs::create_dir_all(src.join("subdir")).unwrap();
        fs::write(src.join("file.txt"), "content").unwrap();
        fs::write(src.join("subdir/nested.txt"), "nested").unwrap();

        // Copy
        copy_dir_recursive(&src, &dest).unwrap();

        // Verify
        assert!(dest.join("file.txt").exists());
        assert!(dest.join("subdir/nested.txt").exists());
        assert_eq!(
            fs::read_to_string(dest.join("file.txt")).unwrap(),
            "content"
        );
    }
}
