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
    /// Convert to string for DuckDB storage
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
    /// Convert to string for DuckDB storage
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
    pub fn new(caller: String, callee: String, file: String, call_type: CallType, line_number: i32) -> Self {
        CallGraphEntry {
            caller,
            callee,
            file,
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
}

/// Common tree-sitter node types for Go
pub mod go_nodes {
}

/// Common tree-sitter node types for Python
pub mod python_nodes {
}

/// Common tree-sitter node types for JavaScript/TypeScript
pub mod js_nodes {
}

/// Common tree-sitter node types for C
pub mod c_nodes {
}

/// Common tree-sitter node types for C++
pub mod cpp_nodes {
}

/// Common tree-sitter node types for Solidity
pub mod solidity_nodes {
}
