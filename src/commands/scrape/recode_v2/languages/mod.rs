// ============================================================================
// LANGUAGE SUPPORT MODULE
// ============================================================================
//! Central module for all language implementations.
//! Each language gets its own file with a complete, self-contained implementation.

use anyhow::{Context, Result};
use std::path::Path;
use tree_sitter::Parser;

// ============================================================================
// LANGUAGE MODULES
// ============================================================================
pub mod rust;
pub mod go;
pub mod python;
pub mod javascript;
pub mod typescript;
pub mod solidity;

// Future additions:
// pub mod c;
// pub mod cpp;
// pub mod cairo;

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
    JavaScriptJSX,  // .jsx files
    TypeScript,
    TypeScriptTSX,  // .tsx files
    Solidity,
    Cairo,          // Future
    C,              // Future
    Cpp,            // Future
    Unknown,
}

impl Language {
    /// Detect language from file extension
    pub fn from_path(path: &Path) -> Self {
        match path.extension().and_then(|ext| ext.to_str()) {
            Some("rs") => Language::Rust,
            Some("go") => Language::Go,
            Some("py") => Language::Python,
            Some("js") | Some("mjs") => Language::JavaScript,
            Some("jsx") => Language::JavaScriptJSX,
            Some("ts") => Language::TypeScript,
            Some("tsx") => Language::TypeScriptTSX,
            Some("sol") => Language::Solidity,
            Some("cairo") => Language::Cairo,
            Some("c") | Some("h") => Language::C,
            Some("cpp") | Some("cc") | Some("cxx") | Some("hpp") | Some("hxx") => Language::Cpp,
            _ => Language::Unknown,
        }
    }

    /// Convert to patina_metal::Metal enum
    pub fn to_metal(self) -> Option<patina_metal::Metal> {
        match self {
            Language::Rust => Some(patina_metal::Metal::Rust),
            Language::Go => Some(patina_metal::Metal::Go),
            Language::Python => Some(patina_metal::Metal::Python),
            Language::JavaScript | Language::JavaScriptJSX => {
                Some(patina_metal::Metal::JavaScript)
            }
            Language::TypeScript | Language::TypeScriptTSX => {
                Some(patina_metal::Metal::TypeScript)
            }
            Language::Solidity => Some(patina_metal::Metal::Solidity),
            Language::Cairo => Some(patina_metal::Metal::Cairo),
            Language::C => Some(patina_metal::Metal::C),
            Language::Cpp => Some(patina_metal::Metal::Cpp),
            Language::Unknown => None,
        }
    }

    /// Get human-readable name
    pub fn name(&self) -> &'static str {
        match self {
            Language::Rust => "Rust",
            Language::Go => "Go",
            Language::Python => "Python",
            Language::JavaScript => "JavaScript",
            Language::JavaScriptJSX => "JavaScript (JSX)",
            Language::TypeScript => "TypeScript",
            Language::TypeScriptTSX => "TypeScript (TSX)",
            Language::Solidity => "Solidity",
            Language::Cairo => "Cairo",
            Language::C => "C",
            Language::Cpp => "C++",
            Language::Unknown => "Unknown",
        }
    }
}

/// Create a parser for a specific file path, handling TypeScript's tsx vs ts distinction
pub fn create_parser_for_path(path: &Path) -> Result<Parser> {
    let language = Language::from_path(path);
    let metal = language
        .to_metal()
        .ok_or_else(|| anyhow::anyhow!("Unsupported language: {:?}", language))?;

    // Use the extension-aware method for TypeScript to get the right parser
    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
    let ts_lang = metal
        .tree_sitter_language_for_ext(ext)
        .ok_or_else(|| anyhow::anyhow!("No parser available for {:?}", language))?;

    let mut parser = Parser::new();
    parser
        .set_language(&ts_lang)
        .context("Failed to set language")?;

    Ok(parser)
}