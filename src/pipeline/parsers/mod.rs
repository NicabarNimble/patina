pub mod go;
pub mod javascript;
pub mod python;
pub mod rust;

use anyhow::Result;
use std::path::Path;

use super::schema::AstData;

/// Parse a file based on its language
pub fn parse_file(path: &Path) -> Result<AstData> {
    let language = super::discovery::detect_language(path);
    
    match language {
        Some("rust") => rust::parse_rust_file(path),
        Some("go") => go::parse_go_file(path),
        Some("python") => python::parse_python_file(path),
        Some("javascript") => javascript::parse_javascript_file(path, false),
        Some("typescript") => javascript::parse_javascript_file(path, true),
        _ => {
            // Return empty AST data for unsupported languages for now
            Ok(AstData::from_path(path))
        }
    }
}