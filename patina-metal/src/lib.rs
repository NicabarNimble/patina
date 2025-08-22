use anyhow::Result;
use std::collections::HashMap;
use tree_sitter::{Node, Parser, Query, Tree};

pub mod metal;
pub mod parser;
pub mod queries;

pub use metal::Metal;
pub use parser::MetalParser;

/// Unified interface for parsing different languages
pub struct Analyzer {
    parsers: HashMap<Metal, Parser>,
    #[allow(dead_code)]
    queries: HashMap<(Metal, QueryType), Query>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum QueryType {
    Symbols,
    Complexity,
    Patterns,
}

/// Parsed file with its AST
pub struct ParsedFile {
    pub tree: Tree,
    pub metal: Metal,
    pub source: String,
}

/// A symbol extracted from code
#[derive(Debug, Clone)]
pub struct Symbol {
    pub name: String,
    pub kind: SymbolKind,
    pub start_line: usize,
    pub end_line: usize,
    pub signature: String,
}

#[derive(Debug, Clone, PartialEq)]
pub enum SymbolKind {
    Function,
    Struct,
    Trait,
    Impl,
    Contract,
    Event,
    Modifier,
}

impl Analyzer {
    /// Create a new analyzer with all supported languages
    pub fn new() -> Result<Self> {
        let mut parsers = HashMap::new();
        
        // Initialize parsers for each metal, but skip broken ones
        for metal in Metal::all() {
            // Try to get the language, skip if not available
            let Some(language) = metal.tree_sitter_language() else {
                eprintln!("Warning: No tree-sitter language available for {:?}", metal);
                continue;
            };
            
            let mut parser = Parser::new();
            
            // Try to set the language, skip if incompatible
            if let Err(e) = parser.set_language(&language) {
                eprintln!("Warning: Failed to set language for {:?}: {}", metal, e);
                continue;
            }
            
            parsers.insert(metal, parser);
        }
        
        // Make sure we have at least one working parser
        if parsers.is_empty() {
            return Err(anyhow::anyhow!("No working parsers available"));
        }
        
        // Load queries (for now, empty - we'll add .scm files later)
        let queries = HashMap::new();
        
        Ok(Self { parsers, queries })
    }
    
    /// Check if a parser is available for a given metal
    pub fn has_parser(&self, metal: Metal) -> bool {
        self.parsers.contains_key(&metal)
    }
    
    /// Get all available metals (ones with working parsers)
    pub fn available_metals(&self) -> Vec<Metal> {
        self.parsers.keys().copied().collect()
    }
    
    /// Parse source code into an AST
    pub fn parse(&mut self, source: &str, metal: Metal) -> Result<ParsedFile> {
        // Check availability first to avoid borrow issues
        if !self.parsers.contains_key(&metal) {
            return Err(anyhow::anyhow!("No parser available for {:?}. Available: {:?}", 
                metal, self.available_metals()));
        }
        
        let parser = self.parsers.get_mut(&metal).unwrap();
            
        let tree = parser.parse(source, None)
            .ok_or_else(|| anyhow::anyhow!("Failed to parse source"))?;
            
        Ok(ParsedFile {
            tree,
            metal,
            source: source.to_string(),
        })
    }
    
    /// Extract symbols from parsed file
    pub fn extract_symbols(&self, file: &ParsedFile) -> Vec<Symbol> {
        let mut symbols = Vec::new();
        let mut cursor = file.tree.walk();
        self.visit_node(&mut cursor, &file.source, file.metal, &mut symbols);
        symbols
    }
    
    /// Calculate complexity of parsed file
    pub fn calculate_complexity(&self, file: &ParsedFile) -> usize {
        let mut complexity = 1;
        let mut cursor = file.tree.walk();
        self.count_branches(&mut cursor, file.metal, &mut complexity);
        complexity
    }
    
    /// Generate fingerprint for pattern matching
    pub fn generate_fingerprint(&self, node: Node, _source: &[u8]) -> u64 {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::Hasher;
        
        let mut hasher = DefaultHasher::new();
        self.hash_node_structure(&mut hasher, node);
        hasher.finish()
    }
    
    // Helper methods
    
    fn visit_node(&self, cursor: &mut tree_sitter::TreeCursor, source: &str, metal: Metal, symbols: &mut Vec<Symbol>) {
        let node = cursor.node();
        
        // Check if this node is a symbol we care about
        if let Some(symbol) = self.extract_symbol(node, source, metal) {
            symbols.push(symbol);
        }
        
        // Recurse into children
        if cursor.goto_first_child() {
            loop {
                self.visit_node(cursor, source, metal, symbols);
                if !cursor.goto_next_sibling() {
                    break;
                }
            }
            cursor.goto_parent();
        }
    }
    
    fn extract_symbol(&self, node: Node, source: &str, metal: Metal) -> Option<Symbol> {
        let kind_str = metal.normalize_node_kind(node.kind());
        
        let kind = match kind_str {
            "function" => SymbolKind::Function,
            "struct" => SymbolKind::Struct,
            "trait" => SymbolKind::Trait,
            "impl" => SymbolKind::Impl,
            "contract" => SymbolKind::Contract,
            "event" => SymbolKind::Event,
            "modifier" => SymbolKind::Modifier,
            _ => return None,
        };
        
        // Extract name
        let name_node = node.child_by_field_name("name")
            .or_else(|| node.child_by_field_name("identifier"))?;
            
        let name = name_node.utf8_text(source.as_bytes()).ok()?.to_string();
        
        // Extract signature (first line)
        let signature = node.utf8_text(source.as_bytes())
            .ok()?
            .lines()
            .next()?
            .to_string();
            
        Some(Symbol {
            name,
            kind,
            start_line: node.start_position().row,
            end_line: node.end_position().row,
            signature,
        })
    }
    
    fn count_branches(&self, cursor: &mut tree_sitter::TreeCursor, metal: Metal, complexity: &mut usize) {
        let node = cursor.node();
        let normalized = metal.normalize_node_kind(node.kind());
        
        // Count complexity-adding constructs
        match normalized {
            "if" | "switch" | "for" | "while" | "match_arm" => *complexity += 1,
            _ => {}
        }
        
        // Recurse
        if cursor.goto_first_child() {
            loop {
                self.count_branches(cursor, metal, complexity);
                if !cursor.goto_next_sibling() {
                    break;
                }
            }
            cursor.goto_parent();
        }
    }
    
    fn hash_node_structure(&self, hasher: &mut impl std::hash::Hasher, node: Node) {
        use std::hash::Hash;
        node.kind().hash(hasher);
        
        let mut cursor = node.walk();
        if cursor.goto_first_child() {
            loop {
                self.hash_node_structure(hasher, cursor.node());
                if !cursor.goto_next_sibling() {
                    break;
                }
            }
        }
    }
}