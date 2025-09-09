// ============================================================================
// LANGUAGE SUPPORT MODULE
// ============================================================================
//! Central module for all language implementations.
//! Each language gets its own file with a complete, self-contained implementation.

use std::path::Path;

// ============================================================================
// LANGUAGE MODULES
// ============================================================================
pub mod c;
pub mod cairo;
pub mod cpp;
pub mod go;
pub mod javascript;
pub mod python;
pub mod rust;
pub mod solidity;
pub mod typescript; // Special non-tree-sitter parser

// ============================================================================
// LANGUAGE ENUM
// ============================================================================
/// Supported programming languages
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Language {
    Rust,
    Go,
    Python,
    JavaScript,
    JavaScriptJSX, // .jsx files
    TypeScript,
    TypeScriptTSX, // .tsx files
    Solidity,
    Cairo, // Future
    C,     // Future
    Cpp,   // Future
    Unknown,
}

impl Language {
    /// Detect language from file extension
    pub fn from_path(path: &Path) -> Self {
        path.extension()
            .and_then(|ext| ext.to_str())
            .and_then(Self::from_extension)
            .unwrap_or(Language::Unknown)
    }

    /// Detect language from file extension string
    pub fn from_extension(ext: &str) -> Option<Self> {
        let lang = match ext {
            "rs" => Language::Rust,
            "go" => Language::Go,
            "py" => Language::Python,
            "js" | "mjs" => Language::JavaScript,
            "jsx" => Language::JavaScriptJSX,
            "ts" => Language::TypeScript,
            "tsx" => Language::TypeScriptTSX,
            "sol" => Language::Solidity,
            "cairo" => Language::Cairo,
            "c" | "h" => Language::C,
            "cpp" | "cc" | "cxx" | "hpp" | "hxx" => Language::Cpp,
            _ => return None,
        };
        Some(lang)
    }

}

