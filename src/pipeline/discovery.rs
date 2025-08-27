use anyhow::Result;
use ignore::WalkBuilder;
use std::path::{Path, PathBuf};

/// Supported programming languages and their file extensions
const LANGUAGE_EXTENSIONS: &[(&str, &[&str])] = &[
    ("rust", &["rs"]),
    ("go", &["go"]),
    ("python", &["py", "pyi"]),
    ("javascript", &["js", "jsx", "mjs"]),
    ("typescript", &["ts", "tsx"]),
    ("java", &["java"]),
    ("c", &["c", "h"]),
    ("cpp", &["cpp", "cc", "cxx", "hpp", "hxx"]),
    ("ruby", &["rb"]),
    ("swift", &["swift"]),
    ("kotlin", &["kt", "kts"]),
    ("scala", &["scala"]),
];

/// Discover all source files in a repository
pub fn discover_files(repo: &Path) -> Result<Vec<PathBuf>> {
    let mut files = Vec::new();
    
    // Build walker with gitignore support
    let walker = WalkBuilder::new(repo)
        .hidden(false)  // Don't skip hidden files
        .git_ignore(true)  // Respect .gitignore
        .git_global(true)  // Respect global gitignore
        .git_exclude(true)  // Respect .git/info/exclude
        .build();
    
    for entry in walker {
        let entry = entry?;
        let path = entry.path();
        
        // Skip directories
        if path.is_dir() {
            continue;
        }
        
        // Check if this is a supported source file
        if is_source_file(path) {
            files.push(path.to_path_buf());
        }
    }
    
    // Sort files for consistent processing
    files.sort();
    
    Ok(files)
}

/// Check if a file is a source file we want to index
fn is_source_file(path: &Path) -> bool {
    // Get file extension
    let Some(extension) = path.extension() else {
        return false;
    };
    
    let Some(ext_str) = extension.to_str() else {
        return false;
    };
    
    // Check against supported extensions
    for (_lang, extensions) in LANGUAGE_EXTENSIONS {
        if extensions.contains(&ext_str) {
            return true;
        }
    }
    
    false
}

/// Detect language from file path
pub fn detect_language(file: &Path) -> Option<&'static str> {
    let extension = file.extension()?.to_str()?;
    
    for (lang, extensions) in LANGUAGE_EXTENSIONS {
        if extensions.contains(&extension) {
            return Some(lang);
        }
    }
    
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;
    
    #[test]
    fn test_is_source_file() {
        assert!(is_source_file(Path::new("main.rs")));
        assert!(is_source_file(Path::new("app.go")));
        assert!(is_source_file(Path::new("script.py")));
        assert!(is_source_file(Path::new("index.js")));
        assert!(is_source_file(Path::new("App.tsx")));
        
        assert!(!is_source_file(Path::new("README.md")));
        assert!(!is_source_file(Path::new("Cargo.toml")));
        assert!(!is_source_file(Path::new("image.png")));
        assert!(!is_source_file(Path::new("no_extension")));
    }
    
    #[test]
    fn test_detect_language() {
        assert_eq!(detect_language(Path::new("main.rs")), Some("rust"));
        assert_eq!(detect_language(Path::new("app.go")), Some("go"));
        assert_eq!(detect_language(Path::new("script.py")), Some("python"));
        assert_eq!(detect_language(Path::new("index.js")), Some("javascript"));
        assert_eq!(detect_language(Path::new("App.tsx")), Some("typescript"));
        assert_eq!(detect_language(Path::new("Main.java")), Some("java"));
        
        assert_eq!(detect_language(Path::new("README.md")), None);
        assert_eq!(detect_language(Path::new("no_extension")), None);
    }
}