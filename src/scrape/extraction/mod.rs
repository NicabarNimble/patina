use anyhow::Result;
use rayon::prelude::*;
use std::path::Path;

use crate::scrape::discovery::{DiscoveredFile, Language};

// Language-specific extractors
pub mod rust;

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
}

/// Function information
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
}

/// Type information (structs, enums, type aliases)
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
}

/// Import/use statement information
#[derive(Debug, Clone)]
pub struct ImportInfo {
    pub module: String,
    pub items: Vec<String>,
    pub is_wildcard: bool,
    pub line_number: usize,
}

/// Call graph information - who calls whom
#[derive(Debug, Clone)]
pub struct CallGraph {
    pub caller: String,
    pub callee: String,
    pub line_number: usize,
    pub is_external: bool,
}

/// Documentation blocks
#[derive(Debug, Clone)]
pub struct Documentation {
    pub kind: DocKind,
    pub content: String,
    pub line_start: usize,
    pub line_end: usize,
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
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

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
            }],
            types: Vec::new(),
            imports: Vec::new(),
            calls: Vec::new(),
            docs: Vec::new(),
        };
        
        assert_eq!(data.functions.len(), 1);
        assert_eq!(data.functions[0].name, "main");
    }
    
    #[test]
    fn test_placeholder_extractor() {
        let extractor = PlaceholderExtractor {
            language: Language::Python,
        };
        
        let path = Path::new("test.py");
        let source = "def main(): pass";
        
        let result = extractor.extract(path, source).unwrap();
        assert_eq!(result.language, Language::Python);
        assert_eq!(result.file_path, "test.py");
        assert!(result.functions.is_empty());
    }
}