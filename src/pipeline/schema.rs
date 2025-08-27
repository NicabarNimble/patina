use serde::{Deserialize, Serialize};

/// Root structure for AST data of a single file
#[derive(Debug, Serialize, Deserialize)]
pub struct AstData {
    pub file: String,
    pub language: String,
    pub functions: Vec<Function>,
    pub types: Vec<TypeDef>,
    pub imports: Vec<Import>,
    pub calls: Vec<Call>,
    // Rich analysis data
    pub fingerprints: Vec<CodeFingerprint>,
    pub symbols: Vec<Symbol>,           // All symbols for code search
    pub file_metrics: Option<FileMetrics>,
}

/// Function definition with rich analysis data
#[derive(Debug, Serialize, Deserialize)]
pub struct Function {
    pub name: String,
    pub visibility: String,
    #[serde(rename = "async")]
    pub is_async: bool,
    #[serde(rename = "unsafe")]
    pub is_unsafe: bool,
    pub params: Vec<Parameter>,
    pub returns: Option<String>,
    pub line_start: usize,
    pub line_end: usize,
    pub doc_comment: Option<String>,
    // Rich analysis fields from old system
    #[serde(skip_serializing_if = "Option::is_none")]
    pub signature: Option<String>,      // Full function signature
    #[serde(skip_serializing_if = "Option::is_none")]
    pub complexity: Option<u16>,         // Cyclomatic complexity
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cognitive_complexity: Option<u16>, // Cognitive complexity
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pattern_hash: Option<u32>,      // AST pattern fingerprint
    #[serde(default)]
    pub is_test: bool,                  // Is this a test function?
    #[serde(default)]
    pub is_generated: bool,              // Is this generated code?
}

/// Function parameter
#[derive(Debug, Serialize, Deserialize)]
pub struct Parameter {
    pub name: String,
    pub param_type: Option<String>,
    pub is_mutable: bool,
    pub is_reference: bool,
}

/// Type definition (struct, enum, interface, class, etc.)
#[derive(Debug, Serialize, Deserialize)]
pub struct TypeDef {
    pub name: String,
    pub kind: String,  // "struct", "enum", "interface", "class", "trait", etc.
    pub visibility: String,
    pub fields: Vec<Field>,
    pub methods: Vec<String>,  // Just method names, full definitions are in functions
    pub line_start: usize,
    pub line_end: usize,
    pub doc_comment: Option<String>,
}

/// Field in a type definition
#[derive(Debug, Serialize, Deserialize)]
pub struct Field {
    pub name: String,
    pub field_type: Option<String>,
    pub visibility: String,
    pub is_mutable: bool,
}

/// Import statement
#[derive(Debug, Serialize, Deserialize)]
pub struct Import {
    pub path: String,
    pub items: Vec<String>,
    pub alias: Option<String>,
    pub line: usize,
}

/// Function call
#[derive(Debug, Serialize, Deserialize)]
pub struct Call {
    pub target: String,  // What's being called
    pub caller: String,  // Function making the call
    pub line: usize,
    pub is_method: bool,
    pub is_async: bool,
}

impl AstData {
    /// Create a new empty AstData for a file
    pub fn new(file: String, language: String) -> Self {
        Self {
            file,
            language,
            functions: Vec::new(),
            types: Vec::new(),
            imports: Vec::new(),
            calls: Vec::new(),
            fingerprints: Vec::new(),
            symbols: Vec::new(),
            file_metrics: None,
        }
    }
    
    /// Create from a file path, detecting the language
    pub fn from_path(path: &std::path::Path) -> Self {
        let file = path.display().to_string();
        let language = super::discovery::detect_language(path)
            .unwrap_or("unknown")
            .to_string();
        Self::new(file, language)
    }
}

/// Code fingerprint for pattern matching and similarity analysis
#[derive(Debug, Serialize, Deserialize)]
pub struct CodeFingerprint {
    pub name: String,
    pub kind: String,  // "function", "type", "module", etc.
    pub pattern: u32,  // AST shape hash
    pub imports: u32,  // Dependency hash  
    pub complexity: u16, // Cyclomatic complexity
    pub flags: u16,    // Feature flags (async, unsafe, etc.)
}

/// Symbol for code search
#[derive(Debug, Serialize, Deserialize)]
pub struct Symbol {
    pub name: String,
    pub kind: String,  // "function", "struct", "trait", "variable", etc.
    pub signature: Option<String>,
    pub context: Option<String>,  // Surrounding context for search
    pub line: usize,
}

/// File-level metrics
#[derive(Debug, Serialize, Deserialize)]
pub struct FileMetrics {
    pub total_lines: usize,
    pub code_lines: usize,
    pub comment_lines: usize,
    pub blank_lines: usize,
    pub complexity_sum: u32,
    pub max_complexity: u16,
    pub function_count: usize,
    pub type_count: usize,
}