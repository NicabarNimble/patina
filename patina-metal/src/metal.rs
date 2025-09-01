use std::path::Path;
use tree_sitter::Language as TSLanguage;

// Import our self-built grammars
use crate::grammars;

/// Supported programming languages (metals)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Metal {
    Rust,
    Go,
    Solidity,
    Cairo,
    Python,
    JavaScript, // .js, .jsx
    TypeScript, // .ts, .tsx (internally uses typescript or tsx parser)
    C,
    Cpp,
}

impl Metal {
    /// Get all supported metals
    pub fn all() -> Vec<Metal> {
        vec![
            Metal::Rust,
            Metal::Go,
            Metal::Solidity,
            Metal::Cairo,
            Metal::Python,
            Metal::JavaScript,
            Metal::TypeScript,
            Metal::C,
            Metal::Cpp,
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
            "py" => Some(Metal::Python),
            "js" | "jsx" | "mjs" => Some(Metal::JavaScript),
            "ts" | "tsx" => Some(Metal::TypeScript),
            "c" | "h" => Some(Metal::C),
            "cpp" | "cc" | "cxx" | "hpp" | "hxx" | "h++" => Some(Metal::Cpp),
            _ => None,
        }
    }

    /// Get the tree-sitter language for this metal
    pub fn tree_sitter_language(&self) -> Option<TSLanguage> {
        match self {
            Metal::Rust => Some(grammars::language_rust()),
            Metal::Go => Some(grammars::language_go()),
            Metal::Solidity => Some(grammars::language_solidity()),
            Metal::Cairo => {
                // Cairo not implemented yet
                None
            }
            Metal::Python => Some(grammars::language_python()),
            Metal::JavaScript => Some(grammars::language_javascript()),
            Metal::TypeScript => Some(grammars::language_typescript()),
            Metal::C => Some(grammars::language_c()),
            Metal::Cpp => Some(grammars::language_cpp()),
        }
    }

    /// Get the tree-sitter language for a specific file extension
    /// This handles TypeScript's dual parser situation (typescript vs tsx)
    pub fn tree_sitter_language_for_ext(&self, ext: &str) -> Option<TSLanguage> {
        match self {
            Metal::TypeScript => match ext {
                "tsx" => Some(grammars::language_tsx()),
                _ => Some(grammars::language_typescript()),
            },
            _ => self.tree_sitter_language(),
        }
    }

    /// Get file extension pattern for finding files
    pub fn file_pattern(&self) -> &'static str {
        match self {
            Metal::Rust => "*.rs",
            Metal::Go => "*.go",
            Metal::Solidity => "*.sol",
            Metal::Cairo => "*.cairo",
            Metal::Python => "*.py",
            Metal::JavaScript => "*.js", // Note: also handles .jsx, .mjs
            Metal::TypeScript => "*.ts", // Note: also handles .tsx
            Metal::C => "*.c",           // Note: also handles .h
            Metal::Cpp => "*.cpp",       // Note: also handles .cc, .cxx, .hpp, etc.
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
            Metal::Python => match node_kind {
                "function_definition" => "function",
                "class_definition" => "struct",
                "decorated_definition" => "function", // Decorators often mark special functions
                "if_statement" => "if",
                "for_statement" => "for",
                "while_statement" => "while",
                _ => node_kind,
            },
            Metal::JavaScript | Metal::TypeScript => match node_kind {
                "function_declaration" | "function_expression" | "arrow_function" => "function",
                "method_definition" => "function",
                "class_declaration" => "struct",
                "interface_declaration" => "trait", // TypeScript only
                "type_alias_declaration" => "type_alias", // TypeScript only
                "if_statement" => "if",
                "for_statement" | "for_in_statement" | "for_of_statement" => "for",
                "while_statement" | "do_statement" => "while",
                "switch_statement" => "switch",
                _ => node_kind,
            },
            Metal::C => match node_kind {
                "function_definition" => "function",
                "struct_specifier" => "struct",
                "union_specifier" => "union",
                "enum_specifier" => "enum",
                "if_statement" => "if",
                "for_statement" => "for",
                "while_statement" | "do_statement" => "while",
                "switch_statement" => "switch",
                _ => node_kind,
            },
            Metal::Cpp => match node_kind {
                "function_definition" => "function",
                "class_specifier" => "struct",
                "struct_specifier" => "struct",
                "union_specifier" => "union",
                "enum_specifier" => "enum",
                "namespace_definition" => "namespace",
                "template_declaration" => "template",
                "if_statement" => "if",
                "for_statement" | "for_range_loop" => "for",
                "while_statement" | "do_statement" => "while",
                "switch_statement" => "switch",
                _ => node_kind,
            },
        }
    }
}
