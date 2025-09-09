// Type-safe wrappers for recode_v2
// Gradual migration from strings to types

use std::fmt;

// ============================================================================
// SYMBOL KINDS
// ============================================================================
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SymbolKind {
    Function,
    Struct,
    Union, // C/C++ union
    Class,
    Trait,
    Interface,
    Module,
    Import,
    Const,
    Static,
    TypeAlias,
    Enum,
    Impl,
    Unknown,
}

impl SymbolKind {
    /// Convert from string (for gradual migration)
    pub fn from_str(s: &str) -> Self {
        match s {
            "function" => Self::Function,
            "struct" => Self::Struct,
            "union" => Self::Union,
            "class" => Self::Class,
            "trait" => Self::Trait,
            "interface" => Self::Interface,
            "module" => Self::Module,
            "import" => Self::Import,
            "const" => Self::Const,
            "static" => Self::Static,
            "type_alias" => Self::TypeAlias,
            "enum" => Self::Enum,
            "impl" => Self::Impl,
            _ => Self::Unknown,
        }
    }

    /// Convert to string (for backward compatibility)
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Function => "function",
            Self::Struct => "struct",
            Self::Union => "union",
            Self::Class => "class",
            Self::Trait => "trait",
            Self::Interface => "interface",
            Self::Module => "module",
            Self::Import => "import",
            Self::Const => "const",
            Self::Static => "static",
            Self::TypeAlias => "type_alias",
            Self::Enum => "enum",
            Self::Impl => "impl",
            Self::Unknown => "unknown",
        }
    }
}

impl fmt::Display for SymbolKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

// ============================================================================
// CALL GRAPH TYPES
// ============================================================================
/// Type-safe representation of call types in the call graph
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CallType {
    Direct,      // Regular function call
    Method,      // Method call (obj.method())
    Async,       // Async/await call
    Goroutine,   // Go goroutine (go func())
    Defer,       // Deferred call (Go defer, Swift defer)
    Macro,       // Macro invocation
    Constructor, // Constructor call (new Class())
    Destructor,  // Destructor call (~Class())
    Decorator,   // Python decorator (@decorator)
    Template,    // C++ template instantiation
    Event,       // Solidity event emission
}

impl CallType {
    /// Convert from string (for gradual migration)
    pub fn from_str(s: &str) -> Self {
        match s {
            "direct" => Self::Direct,
            "method" => Self::Method,
            "async" => Self::Async,
            "goroutine" => Self::Goroutine,
            "defer" => Self::Defer,
            "macro" => Self::Macro,
            "constructor" => Self::Constructor,
            "destructor" => Self::Destructor,
            "decorator" => Self::Decorator,
            "template" => Self::Template,
            "event" => Self::Event,
            _ => Self::Direct, // Default to direct for unknown
        }
    }

    /// Convert to string (for SQL generation)
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Direct => "direct",
            Self::Method => "method",
            Self::Async => "async",
            Self::Goroutine => "goroutine",
            Self::Defer => "defer",
            Self::Macro => "macro",
            Self::Constructor => "constructor",
            Self::Destructor => "destructor",
            Self::Decorator => "decorator",
            Self::Template => "template",
            Self::Event => "event",
        }
    }
}

impl fmt::Display for CallType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Type-safe call graph entry
#[derive(Debug, Clone)]
pub struct CallGraphEntry {
    pub caller: String,
    pub callee: String,
    pub file: String,
    pub call_type: CallType,
    pub line_number: i32,
}

impl CallGraphEntry {
    pub fn new(caller: String, callee: String, call_type: CallType, line_number: i32) -> Self {
        CallGraphEntry {
            caller,
            callee,
            file: String::new(), // File will be filled in later
            call_type,
            line_number,
        }
    }
}

// ============================================================================
// NEWTYPES FOR SAFETY
// ============================================================================
/// Type-safe wrapper for file paths
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FilePath<'a>(pub &'a str);

impl<'a> FilePath<'a> {
    /// Create a new FilePath
    pub fn new(path: &'a str) -> Self {
        FilePath(path)
    }

    /// Get the inner string
    pub fn as_str(&self) -> &'a str {
        self.0
    }

    /// Check if this is a Rust file
    pub fn is_rust_file(&self) -> bool {
        self.0.ends_with(".rs")
    }

    /// Get the file extension
    pub fn extension(&self) -> Option<&'a str> {
        self.0.rsplit('.').next()
    }
}

impl<'a> From<&'a str> for FilePath<'a> {
    fn from(s: &'a str) -> Self {
        FilePath::new(s)
    }
}

impl<'a> fmt::Display for FilePath<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Type-safe wrapper for symbol names
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SymbolName<'a>(pub &'a str);

impl<'a> SymbolName<'a> {
    /// Create a new SymbolName
    pub fn new(name: &'a str) -> Self {
        SymbolName(name)
    }

    /// Get the inner string
    pub fn as_str(&self) -> &'a str {
        self.0
    }

    /// Check if this is a public symbol (for languages that use naming conventions)
    pub fn is_public_by_convention(&self) -> bool {
        // Go convention: uppercase first letter
        self.0.chars().next().is_some_and(|c| c.is_uppercase())
    }

    /// Check if this is private by Python convention
    pub fn is_private_python(&self) -> bool {
        self.0.starts_with('_')
    }
}

impl<'a> From<&'a str> for SymbolName<'a> {
    fn from(s: &'a str) -> Self {
        SymbolName::new(s)
    }
}

impl<'a> fmt::Display for SymbolName<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Type-safe wrapper for documentation strings
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DocString<'a>(pub &'a str);

impl<'a> DocString<'a> {
    /// Create a new DocString
    pub fn new(doc: &'a str) -> Self {
        DocString(doc)
    }

    /// Get the inner string
    pub fn as_str(&self) -> &'a str {
        self.0
    }

    /// Check if this is a doc comment
    pub fn is_doc_comment(&self) -> bool {
        self.0.starts_with("///")
            || self.0.starts_with("//!")
            || self.0.starts_with("/**")
            || self.0.starts_with("\"\"\"")
    }
}

impl<'a> From<&'a str> for DocString<'a> {
    fn from(s: &'a str) -> Self {
        DocString::new(s)
    }
}

impl<'a> fmt::Display for DocString<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

// ============================================================================
// NODE KINDS (Tree-sitter specific)
// ============================================================================
/// Common tree-sitter node types for Rust
pub mod rust_nodes {
    pub const FUNCTION_ITEM: &str = "function_item";
    pub const STRUCT_ITEM: &str = "struct_item";
    pub const TRAIT_ITEM: &str = "trait_item";
    pub const IMPL_ITEM: &str = "impl_item";
    pub const TYPE_ALIAS: &str = "type_alias";
    pub const CONST_ITEM: &str = "const_item";
    pub const STATIC_ITEM: &str = "static_item";
    pub const ENUM_ITEM: &str = "enum_item";
    pub const MODULE: &str = "mod_item";
    pub const USE_DECLARATION: &str = "use_declaration";
    pub const MACRO_INVOCATION: &str = "macro_invocation";
}

/// Common tree-sitter node types for Go
pub mod go_nodes {
    pub const FUNCTION_DECLARATION: &str = "function_declaration";
    pub const METHOD_DECLARATION: &str = "method_declaration";
    pub const TYPE_DECLARATION: &str = "type_declaration";
    pub const TYPE_SPEC: &str = "type_spec";
    pub const STRUCT_TYPE: &str = "struct_type";
    pub const INTERFACE_TYPE: &str = "interface_type";
    pub const CONST_DECLARATION: &str = "const_declaration";
    pub const VAR_DECLARATION: &str = "var_declaration";
    pub const IMPORT_DECLARATION: &str = "import_declaration";
    pub const PACKAGE_CLAUSE: &str = "package_clause";
    pub const CALL_EXPRESSION: &str = "call_expression";
    pub const GO_STATEMENT: &str = "go_statement";
    pub const DEFER_STATEMENT: &str = "defer_statement";
}

/// Common tree-sitter node types for Python
pub mod python_nodes {
    pub const FUNCTION_DEFINITION: &str = "function_definition";
    pub const CLASS_DEFINITION: &str = "class_definition";
    pub const DECORATED_DEFINITION: &str = "decorated_definition";
    pub const IMPORT_STATEMENT: &str = "import_statement";
    pub const IMPORT_FROM_STATEMENT: &str = "import_from_statement";
    pub const ASSIGNMENT: &str = "assignment";
    pub const CALL: &str = "call";
    pub const IDENTIFIER: &str = "identifier";
    pub const MODULE: &str = "module";
}

/// Common tree-sitter node types for JavaScript/TypeScript
pub mod js_nodes {
    pub const FUNCTION_DECLARATION: &str = "function_declaration";
    pub const FUNCTION_EXPRESSION: &str = "function_expression";
    pub const ARROW_FUNCTION: &str = "arrow_function";
    pub const CLASS_DECLARATION: &str = "class_declaration";
    pub const METHOD_DEFINITION: &str = "method_definition";
    pub const VARIABLE_DECLARATION: &str = "variable_declaration";
    pub const LEXICAL_DECLARATION: &str = "lexical_declaration";
    pub const IMPORT_STATEMENT: &str = "import_statement";
    pub const EXPORT_STATEMENT: &str = "export_statement";
    pub const CALL_EXPRESSION: &str = "call_expression";
    pub const INTERFACE_DECLARATION: &str = "interface_declaration";
    pub const TYPE_ALIAS_DECLARATION: &str = "type_alias_declaration";
    pub const ENUM_DECLARATION: &str = "enum_declaration";
}

/// Common tree-sitter node types for C
pub mod c_nodes {
    pub const FUNCTION_DEFINITION: &str = "function_definition";
    pub const DECLARATION: &str = "declaration";
    pub const STRUCT_SPECIFIER: &str = "struct_specifier";
    pub const UNION_SPECIFIER: &str = "union_specifier";
    pub const ENUM_SPECIFIER: &str = "enum_specifier";
    pub const TYPE_DEFINITION: &str = "type_definition";
    pub const PREPROC_INCLUDE: &str = "preproc_include";
    pub const PREPROC_DEF: &str = "preproc_def";
    pub const CALL_EXPRESSION: &str = "call_expression";
}

/// Common tree-sitter node types for C++
pub mod cpp_nodes {
    pub const FUNCTION_DEFINITION: &str = "function_definition";
    pub const CLASS_SPECIFIER: &str = "class_specifier";
    pub const STRUCT_SPECIFIER: &str = "struct_specifier";
    pub const NAMESPACE_DEFINITION: &str = "namespace_definition";
    pub const TEMPLATE_DECLARATION: &str = "template_declaration";
    pub const DECLARATION: &str = "declaration";
    pub const USING_DECLARATION: &str = "using_declaration";
    pub const PREPROC_INCLUDE: &str = "preproc_include";
    pub const CALL_EXPRESSION: &str = "call_expression";
}

/// Common tree-sitter node types for Solidity
pub mod solidity_nodes {
    pub const FUNCTION_DEFINITION: &str = "function_definition";
    pub const CONTRACT_DECLARATION: &str = "contract_declaration";
    pub const INTERFACE_DECLARATION: &str = "interface_declaration";
    pub const LIBRARY_DECLARATION: &str = "library_declaration";
    pub const STRUCT_DECLARATION: &str = "struct_declaration";
    pub const ENUM_DECLARATION: &str = "enum_declaration";
    pub const EVENT_DEFINITION: &str = "event_definition";
    pub const MODIFIER_DEFINITION: &str = "modifier_definition";
    pub const STATE_VARIABLE_DECLARATION: &str = "state_variable_declaration";
    pub const IMPORT_DIRECTIVE: &str = "import_directive";
    pub const CALL_EXPRESSION: &str = "call_expression";
}
