//! Pattern copying functionality for initialization

use anyhow::Result;
use std::fs;
use std::path::{Path, PathBuf};

/// Copy core patterns from Patina to a new project, safely
pub fn copy_core_patterns_safe(project_path: &Path, target_layer: &Path) -> Result<bool> {
    let target_core = target_layer.join("core");

    // Don't copy if we're already in the Patina source directory
    // (prevents self-overwriting when running init . in root)
    if is_patina_source(project_path)? {
        return Ok(false);
    }

    // Try to find source patterns
    let source_core = find_patina_core_patterns()?;

    if let Some(source_core) = source_core {
        // Prevent copying to self
        if let (Ok(source_canonical), Ok(target_canonical)) = (
            fs::canonicalize(&source_core),
            fs::canonicalize(&target_core),
        ) {
            if source_canonical == target_canonical {
                return Ok(false);
            }
        }

        fs::create_dir_all(&target_core)?;

        for entry in fs::read_dir(&source_core)? {
            let entry = entry?;
            let source_file = entry.path();
            if source_file.is_file() && source_file.extension().is_some_and(|ext| ext == "md") {
                let file_name = source_file.file_name().unwrap();
                let target_file = target_core.join(file_name);

                // Only copy if target doesn't exist or is empty (from failed copy)
                if !target_file.exists() || fs::metadata(&target_file)?.len() == 0 {
                    fs::copy(&source_file, &target_file)?;
                }
            }
        }
        Ok(true)
    } else {
        Ok(false)
    }
}

/// Check if we're in the Patina source directory
fn is_patina_source(project_path: &Path) -> Result<bool> {
    if project_path.join("Cargo.toml").exists() {
        if let Ok(cargo_content) = fs::read_to_string(project_path.join("Cargo.toml")) {
            return Ok(cargo_content.contains("name = \"patina\""));
        }
    }
    Ok(false)
}

/// Find Patina's core patterns using multiple strategies
fn find_patina_core_patterns() -> Result<Option<PathBuf>> {
    // Strategy 1: Development environment (cargo run)
    if let Ok(manifest_dir) = std::env::var("CARGO_MANIFEST_DIR") {
        let core_path = PathBuf::from(manifest_dir).join("layer").join("core");
        if core_path.exists() {
            return Ok(Some(core_path));
        }
    }

    // Strategy 2: Relative to executable
    if let Ok(exe_path) = std::env::current_exe() {
        if let Some(patina_root) = exe_path
            .parent()
            .and_then(|p| p.parent())
            .and_then(|p| p.parent())
        {
            let core_path = patina_root.join("layer").join("core");
            if core_path.exists() {
                return Ok(Some(core_path));
            }
        }
    }

    // Strategy 3: Future - installed Patina in HOME
    if let Ok(home) = std::env::var("HOME") {
        let core_path = PathBuf::from(home)
            .join(".patina")
            .join("layer")
            .join("core");
        if core_path.exists() {
            return Ok(Some(core_path));
        }
    }

    Ok(None)
}
