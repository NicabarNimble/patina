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
    pub fn new(
        caller: String,
        callee: String,
        file: String,
        call_type: CallType,
        line_number: i32,
    ) -> Self {
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
