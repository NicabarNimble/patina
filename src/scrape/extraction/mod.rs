use anyhow::Result;
use rayon::prelude::*;
use std::path::Path;

use crate::scrape::discovery::{DiscoveredFile, Language};

// Language-specific extractors
pub mod rust;
pub mod go;

/// Core semantic data structure - what we extract from source code
#[derive(Debug, Clone)]
pub struct SemanticData {
    pub file_path: String,
    pub language: Language,
    pub functions: Vec<FunctionInfo>,
    pub types: Vec<TypeInfo>,
    pub imports: Vec<ImportInfo>,
    pub calls: Vec<CallGraph>,
    pub docs: Vec<Documentation>,
    pub fingerprints: Vec<(String, FunctionFingerprint)>,  // name -> fingerprint
    pub behavioral_hints: Vec<BehavioralHints>,
}

/// Function information with full behavioral analysis
#[derive(Debug, Clone)]
pub struct FunctionInfo {
    pub name: String,
    pub visibility: Visibility,
    pub parameters: Vec<Parameter>,
    pub return_type: Option<String>,
    pub is_async: bool,
    pub is_unsafe: bool,
    pub line_start: usize,
    pub line_end: usize,
    pub doc_comment: Option<String>,
    
    // Function facts for behavioral analysis
    pub signature: String,              // Full function signature
    pub takes_mut_self: bool,          // &mut self parameter
    pub takes_mut_params: bool,        // Any &mut parameters
    pub returns_result: bool,          // Returns Result<...>
    pub returns_option: bool,          // Returns Option<...>
    pub parameter_count: usize,        // Total parameter count
    pub has_self: bool,               // Is a method (has self)
    pub context_snippet: String,       // Surrounding code for search
}

/// Type information (structs, enums, type aliases) with full definitions
#[derive(Debug, Clone)]
pub struct TypeInfo {
    pub name: String,
    pub kind: TypeKind,
    pub visibility: Visibility,
    pub fields: Vec<Field>,
    pub generics: Vec<String>,
    pub line_start: usize,
    pub line_end: usize,
    pub doc_comment: Option<String>,
    
    // Type vocabulary
    pub full_definition: String,       // Complete type definition
    pub signature: String,              // Type signature
    pub context_snippet: String,       // Surrounding code
}

/// Import/use statement information
#[derive(Debug, Clone)]
pub struct ImportInfo {
    pub module: String,
    pub items: Vec<String>,
    pub is_wildcard: bool,
    pub is_external: bool,              // External crate?
    pub line_number: usize,
}

/// Call graph information - who calls whom
#[derive(Debug, Clone)]
pub struct CallGraph {
    pub caller: String,
    pub callee: String,
    pub line_number: usize,
    pub call_type: String,             // "direct", "method", "async", "callback"
    pub is_external: bool,
}

/// Documentation blocks with analysis
#[derive(Debug, Clone)]
pub struct Documentation {
    pub kind: DocKind,
    pub symbol_name: String,           // Associated symbol
    pub content: String,
    pub line_start: usize,
    pub line_end: usize,
    
    // Documentation analysis
    pub raw_content: String,           // Original with markers
    pub summary: String,                // First sentence
    pub keywords: Vec<String>,          // Extracted keywords
    pub has_examples: bool,             // Contains code blocks
}

/// AST-based function fingerprint
#[derive(Debug, Clone)]
pub struct FunctionFingerprint {
    pub pattern: u32,                  // AST shape hash
    pub imports: u32,                  // Dependency hash
    pub complexity: u16,               // Cyclomatic complexity
    pub flags: u16,                    // Feature flags
}

/// Behavioral hints for code quality analysis
#[derive(Debug, Clone)]
pub struct BehavioralHints {
    pub function_name: String,
    pub calls_unwrap: usize,          // Count of .unwrap()
    pub calls_expect: usize,          // Count of .expect()
    pub has_panic_macro: bool,        // Contains panic!()
    pub has_todo_macro: bool,         // Contains todo!()
    pub has_unsafe_block: bool,       // Contains unsafe {}
    pub has_mutex: bool,              // Thread synchronization
    pub has_arc: bool,                // Shared ownership
}

/// Visibility modifier
#[derive(Debug, Clone, PartialEq)]
pub enum Visibility {
    Public,
    Private,
    Protected,
    Internal,
}

/// Type kinds
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum TypeKind {
    Struct,
    Enum,
    Interface,
    Class,
    TypeAlias,
    Trait,
}

impl TypeKind {
    pub fn to_string(&self) -> String {
        match self {
            TypeKind::Struct => "struct".to_string(),
            TypeKind::Enum => "enum".to_string(),
            TypeKind::Interface => "interface".to_string(),
            TypeKind::Class => "class".to_string(),
            TypeKind::TypeAlias => "type_alias".to_string(),
            TypeKind::Trait => "trait".to_string(),
        }
    }
}

/// Documentation kinds
#[derive(Debug, Clone, PartialEq)]
pub enum DocKind {
    Module,
    Function,
    Type,
    Field,
    Comment,
}

/// Function parameter
#[derive(Debug, Clone)]
pub struct Parameter {
    pub name: String,
    pub type_annotation: Option<String>,
    pub default_value: Option<String>,
}

/// Type field (struct field, enum variant, etc)
#[derive(Debug, Clone)]
pub struct Field {
    pub name: String,
    pub type_annotation: Option<String>,
    pub visibility: Visibility,
    pub doc_comment: Option<String>,
}

/// Trait for language-specific extractors
pub trait LanguageExtractor: Send + Sync {
    fn extract(&self, path: &Path, source: &str) -> Result<SemanticData>;
}

/// Extract semantic data from all discovered files
pub fn extract_all(files: Vec<DiscoveredFile>) -> Result<Vec<SemanticData>> {
    files
        .par_iter()
        .map(|file| {
            let source = std::fs::read_to_string(&file.path)?;
            let extractor = create_extractor(file.language);
            extractor.extract(&file.path, &source)
        })
        .collect()
}

/// Create the appropriate extractor for a language
fn create_extractor(language: Language) -> Box<dyn LanguageExtractor> {
    match language {
        Language::Rust => Box::new(rust::RustExtractor),
        Language::Go => Box::new(go::GoExtractor),
        // Placeholder for other languages - will implement next
        _ => Box::new(PlaceholderExtractor { language }),
    }
}

/// Placeholder extractor for unimplemented languages
struct PlaceholderExtractor {
    language: Language,
}

impl LanguageExtractor for PlaceholderExtractor {
    fn extract(&self, path: &Path, _source: &str) -> Result<SemanticData> {
        // Return empty semantic data for now
        Ok(SemanticData {
            file_path: path.to_string_lossy().to_string(),
            language: self.language,
            functions: Vec::new(),
            types: Vec::new(),
            imports: Vec::new(),
            calls: Vec::new(),
            docs: Vec::new(),
            fingerprints: Vec::new(),
            behavioral_hints: Vec::new(),
        })
    }
}

/// Helper function to extract keywords from text
pub fn extract_keywords(text: &str) -> Vec<String> {
    const STOP_WORDS: &[&str] = &[
        "the", "and", "for", "with", "this", "that", "from", "into",
        "will", "can", "may", "must", "should", "would", "could",
        "has", "have", "had", "does", "did", "are", "was", "were",
        "been", "being", "get", "set", "new", "all", "some", "any",
        "each", "every", "but", "not", "only", "also", "just", "more",
    ];
    
    text.split_whitespace()
        .flat_map(|word| word.split(|c: char| !c.is_alphanumeric()))
        .filter(|w| w.len() > 3)
        .map(|w| w.to_lowercase())
        .filter(|w| !STOP_WORDS.contains(&w.as_str()))
        .collect::<std::collections::HashSet<_>>()
        .into_iter()
        .collect()
}

/// Helper function to extract first sentence as summary
pub fn extract_summary(text: &str) -> String {
    text.split('.')
        .next()
        .unwrap_or(text)
        .trim()
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_semantic_data_creation() {
        let data = SemanticData {
            file_path: "test.rs".to_string(),
            language: Language::Rust,
            functions: vec![FunctionInfo {
                name: "main".to_string(),
                visibility: Visibility::Public,
                parameters: Vec::new(),
                return_type: None,
                is_async: false,
                is_unsafe: false,
                line_start: 1,
                line_end: 3,
                doc_comment: None,
                signature: "pub fn main()".to_string(),
                takes_mut_self: false,
                takes_mut_params: false,
                returns_result: false,
                returns_option: false,
                parameter_count: 0,
                has_self: false,
                context_snippet: String::new(),
            }],
            types: Vec::new(),
            imports: Vec::new(),
            calls: Vec::new(),
            docs: Vec::new(),
            fingerprints: Vec::new(),
            behavioral_hints: Vec::new(),
        };
        
        assert_eq!(data.functions.len(), 1);
        assert_eq!(data.functions[0].name, "main");
    }
    
    #[test]
    fn test_keyword_extraction() {
        let text = "This function processes the input data and returns the result";
        let keywords = extract_keywords(text);
        
        assert!(keywords.contains(&"function".to_string()));
        assert!(keywords.contains(&"processes".to_string()));
        assert!(keywords.contains(&"input".to_string()));
        assert!(keywords.contains(&"data".to_string()));
        assert!(keywords.contains(&"returns".to_string()));
        assert!(keywords.contains(&"result".to_string()));
        assert!(!keywords.contains(&"the".to_string())); // Stop word
        assert!(!keywords.contains(&"and".to_string())); // Stop word
    }
    
    #[test]
    fn test_summary_extraction() {
        let text = "This is the first sentence. This is the second sentence.";
        let summary = extract_summary(text);
        assert_eq!(summary, "This is the first sentence");
    }
}