use std::path::Path;
use tree_sitter::Language as TSLanguage;

/// Supported programming languages (metals)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Metal {
    Rust,
    Go,
    Solidity,
    Cairo,
}

impl Metal {
    /// Get all supported metals
    pub fn all() -> Vec<Metal> {
        vec![
            Metal::Rust,
            Metal::Go,
            Metal::Solidity,
            Metal::Cairo,
        ]
    }
    
    /// Detect metal from file extension
    pub fn from_path(path: &Path) -> Option<Self> {
        let ext = path.extension()?.to_str()?;
        Self::from_extension(ext)
    }
    
    /// Detect metal from extension string
    pub fn from_extension(ext: &str) -> Option<Self> {
        match ext {
            "rs" => Some(Metal::Rust),
            "go" => Some(Metal::Go),
            "sol" => Some(Metal::Solidity),
            "cairo" => Some(Metal::Cairo),
            _ => None,
        }
    }
    
    /// Get the tree-sitter language for this metal
    pub fn tree_sitter_language(&self) -> Option<TSLanguage> {
        match self {
            #[cfg(feature = "rust")]
            Metal::Rust => Some(tree_sitter_rust::LANGUAGE.into()),
            #[cfg(feature = "go")]
            Metal::Go => Some(tree_sitter_go::LANGUAGE.into()),
            #[cfg(feature = "solidity")]
            Metal::Solidity => Some(tree_sitter_solidity::LANGUAGE.into()),
            #[cfg(feature = "cairo")]
            Metal::Cairo => {
                // Cairo might not be available yet
                None
            },
            _ => None,
        }
    }
    
    /// Get file extension pattern for finding files
    pub fn file_pattern(&self) -> &'static str {
        match self {
            Metal::Rust => "*.rs",
            Metal::Go => "*.go",
            Metal::Solidity => "*.sol",
            Metal::Cairo => "*.cairo",
        }
    }
    
    /// Map language-specific node types to generic categories
    pub fn normalize_node_kind<'a>(&self, node_kind: &'a str) -> &'a str {
        match self {
            Metal::Rust => match node_kind {
                "function_item" => "function",
                "struct_item" => "struct",
                "trait_item" => "trait",
                "impl_item" => "impl",
                "if_expression" => "if",
                "match_expression" => "switch",
                "while_expression" => "while",
                "for_expression" => "for",
                "match_arm" => "match_arm",
                _ => node_kind,
            },
            Metal::Go => match node_kind {
                "function_declaration" | "method_declaration" => "function",
                "type_spec" => "struct", // Simplified
                "interface_type" => "trait",
                "if_statement" => "if",
                "switch_statement" => "switch",
                "for_statement" => "for",
                _ => node_kind,
            },
            Metal::Solidity => match node_kind {
                "function_definition" => "function",
                "contract_declaration" => "contract",
                "interface_declaration" => "trait",
                "library_declaration" => "impl",
                "modifier_definition" => "modifier",
                "event_definition" => "event",
                "if_statement" => "if",
                "for_statement" => "for",
                "while_statement" => "while",
                _ => node_kind,
            },
            Metal::Cairo => match node_kind {
                "function_definition" => "function",
                "trait_definition" => "trait",
                "impl_block" => "impl",
                "struct_definition" => "struct",
                "if_expression" => "if",
                "loop_expression" => "while",
                _ => node_kind,
            },
        }
    }
}