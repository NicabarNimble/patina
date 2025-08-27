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
}

/// Function definition
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