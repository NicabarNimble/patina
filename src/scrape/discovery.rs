use anyhow::Result;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

// Re-export Language from semantic module
pub use crate::semantic::languages::Language;

/// File discovery result - path and detected language
#[derive(Debug, Clone)]
pub struct DiscoveredFile {
    pub path: PathBuf,
    pub language: Language,
}

/// Find all files in directory and detect their languages
/// This is a pure function - no I/O beyond reading directory structure
pub fn find_files(work_dir: &Path) -> Result<Vec<DiscoveredFile>> {
    // Common directories to skip
    let skip_dirs = vec![
        ".git",
        "target",
        "node_modules",
        "dist",
        "build",
        ".patina",
        "vendor",
        "__pycache__",
        ".pytest_cache",
        ".mypy_cache",
        ".tox",
        "venv",
        ".venv",
    ];
    
    let mut files = Vec::new();
    
    for entry in WalkDir::new(work_dir)
        .follow_links(false)
        .into_iter()
        .filter_entry(|e| {
            // Skip hidden directories and common build/dependency directories
            if e.file_type().is_dir() {
                if let Some(name) = e.file_name().to_str() {
                    return !name.starts_with('.') && !skip_dirs.contains(&name);
                }
            }
            true
        })
        .filter_map(|e| e.ok())
    {
        if entry.file_type().is_file() {
            let path = entry.path().to_owned();
            let language = Language::from_path(&path);
            
            // Only include files with recognized languages
            if language != Language::Unknown {
                files.push(DiscoveredFile {
                    path,
                    language,
                });
            }
        }
    }
    
    // Sort files by path for consistent processing
    files.sort_by(|a, b| a.path.cmp(&b.path));
    
    Ok(files)
}

/// Filter discovered files by language
pub fn filter_by_language(files: &[DiscoveredFile], language: Language) -> Vec<DiscoveredFile> {
    files
        .iter()
        .filter(|f| f.language == language)
        .cloned()
        .collect()
}

/// Group discovered files by language
pub fn group_by_language(files: Vec<DiscoveredFile>) -> std::collections::HashMap<Language, Vec<PathBuf>> {
    use std::collections::HashMap;
    
    let mut groups: HashMap<Language, Vec<PathBuf>> = HashMap::new();
    
    for file in files {
        groups
            .entry(file.language)
            .or_insert_with(Vec::new)
            .push(file.path);
    }
    
    groups
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_discovery_finds_rust_files() -> Result<()> {
        let temp = TempDir::new()?;
        let temp_path = temp.path();
        
        // Create test files
        fs::write(temp_path.join("main.rs"), "fn main() {}")?;
        fs::write(temp_path.join("lib.rs"), "pub fn foo() {}")?;
        fs::write(temp_path.join("readme.md"), "# Test")?;
        
        // Create subdirectory with file
        let sub_dir = temp_path.join("src");
        fs::create_dir(&sub_dir)?;
        fs::write(sub_dir.join("module.rs"), "mod test;")?;
        
        let files = find_files(temp_path)?;
        
        // Should find 3 Rust files, ignore the markdown
        assert_eq!(files.len(), 3);
        assert!(files.iter().all(|f| f.language == Language::Rust));
        
        Ok(())
    }
    
    #[test]
    fn test_discovery_skips_hidden_and_build_dirs() -> Result<()> {
        let temp = TempDir::new()?;
        let temp_path = temp.path();
        
        // Create visible file
        fs::write(temp_path.join("main.go"), "package main")?;
        
        // Create hidden directory with file (should be skipped)
        let hidden_dir = temp_path.join(".hidden");
        fs::create_dir(&hidden_dir)?;
        fs::write(hidden_dir.join("secret.go"), "package secret")?;
        
        // Create target directory with file (should be skipped)
        let target_dir = temp_path.join("target");
        fs::create_dir(&target_dir)?;
        fs::write(target_dir.join("build.go"), "package build")?;
        
        let files = find_files(temp_path)?;
        
        // Should only find the visible main.go
        assert_eq!(files.len(), 1);
        assert_eq!(files[0].language, Language::Go);
        assert!(files[0].path.ends_with("main.go"));
        
        Ok(())
    }
    
    #[test]
    fn test_group_by_language() {
        let files = vec![
            DiscoveredFile {
                path: PathBuf::from("main.rs"),
                language: Language::Rust,
            },
            DiscoveredFile {
                path: PathBuf::from("lib.rs"),
                language: Language::Rust,
            },
            DiscoveredFile {
                path: PathBuf::from("main.go"),
                language: Language::Go,
            },
            DiscoveredFile {
                path: PathBuf::from("app.py"),
                language: Language::Python,
            },
        ];
        
        let groups = group_by_language(files);
        
        assert_eq!(groups.len(), 3);
        assert_eq!(groups.get(&Language::Rust).unwrap().len(), 2);
        assert_eq!(groups.get(&Language::Go).unwrap().len(), 1);
        assert_eq!(groups.get(&Language::Python).unwrap().len(), 1);
    }
}