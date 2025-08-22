use anyhow::{Context, Result};
use std::path::Path;
use tree_sitter::{Language as TSLanguage, Parser};

/// Supported programming languages
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Language {
    Rust,
    Go,
    Solidity,
    Unknown,
}

impl Language {
    /// Detect language from file extension
    pub fn from_path(path: &Path) -> Self {
        match path.extension().and_then(|ext| ext.to_str()) {
            Some("rs") => Language::Rust,
            Some("go") => Language::Go,
            Some("sol") => Language::Solidity,
            _ => Language::Unknown,
        }
    }
    
    /// Get the tree-sitter language
    pub fn tree_sitter_language(&self) -> Option<TSLanguage> {
        match self {
            Language::Rust => Some(tree_sitter_rust::LANGUAGE.into()),
            Language::Go => Some(tree_sitter_go::LANGUAGE.into()),
            Language::Solidity => Some(tree_sitter_solidity::LANGUAGE.into()),
            Language::Unknown => None,
        }
    }
    
    /// Get file extension pattern for finding files
    pub fn file_pattern(&self) -> &'static str {
        match self {
            Language::Rust => "*.rs",
            Language::Go => "*.go",
            Language::Solidity => "*.sol",
            Language::Unknown => "*",
        }
    }
    
    /// Map language-specific node types to generic categories
    pub fn normalize_node_kind<'a>(&self, node_kind: &'a str) -> &'a str {
        match self {
            Language::Rust => match node_kind {
                "function_item" => "function",
                "struct_item" => "struct",
                "trait_item" => "trait",
                "impl_item" => "impl",
                "if_expression" => "if",
                "match_expression" => "switch",
                "while_expression" => "while",
                "for_expression" => "for",
                _ => node_kind,
            },
            Language::Go => match node_kind {
                "function_declaration" | "method_declaration" => "function",
                "type_declaration" => "struct",
                "interface_type" => "trait",
                "if_statement" => "if",
                "switch_statement" => "switch",
                "for_statement" => "for",
                _ => node_kind,
            },
            Language::Solidity => match node_kind {
                "function_definition" => "function",
                "contract_declaration" => "struct",  // Contracts are like structs
                "interface_declaration" => "trait",   // Interfaces are like traits
                "library_declaration" => "impl",      // Libraries are like impl blocks
                "modifier_definition" => "function",  // Modifiers are special functions
                "event_definition" => "function",     // Events are like functions
                "if_statement" => "if",
                "for_statement" => "for",
                "while_statement" => "while",
                _ => node_kind,
            },
            Language::Unknown => node_kind,
        }
    }
}

/// Create a parser for the given language
pub fn create_parser(language: Language) -> Result<Parser> {
    let mut parser = Parser::new();
    
    if let Some(ts_lang) = language.tree_sitter_language() {
        parser
            .set_language(&ts_lang)
            .context("Failed to set language")?;
        Ok(parser)
    } else {
        anyhow::bail!("Unsupported language: {:?}", language)
    }
}

/// Detect all languages in a directory
pub fn detect_languages(dir: &Path) -> Result<Vec<Language>> {
    use std::collections::HashSet;
    let mut languages = HashSet::new();
    
    for entry in walkdir::WalkDir::new(dir)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
    {
        let lang = Language::from_path(entry.path());
        if lang != Language::Unknown {
            languages.insert(lang as u8);
        }
    }
    
    Ok(languages.into_iter().map(|l| match l {
        0 => Language::Rust,
        1 => Language::Go,
        2 => Language::Solidity,
        _ => Language::Unknown,
    }).collect())
}