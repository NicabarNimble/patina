//! Cairo language parser using SimpleParserDatabase
//!
//! This module provides Cairo 2.x parsing capabilities without the complexity
//! of the full Salsa database system. It uses cairo-lang-parser's SimpleParserDatabase
//! for straightforward AST extraction.

use anyhow::Result;
use cairo_lang_parser::utils::SimpleParserDatabase;
use cairo_lang_syntax::node::kind::SyntaxKind;
use cairo_lang_syntax::node::{ast, SyntaxNode, Terminal, TypedSyntaxNode};

/// Symbols extracted from Cairo source code
#[derive(Debug, Default)]
pub struct CairoSymbols {
    pub functions: Vec<FunctionSymbol>,
    pub structs: Vec<StructSymbol>,
    pub traits: Vec<TraitSymbol>,
    pub impls: Vec<ImplSymbol>,
    pub modules: Vec<ModuleSymbol>,
    pub imports: Vec<ImportSymbol>,
}

#[derive(Debug)]
pub struct FunctionSymbol {
    pub name: String,
    pub start_line: usize,
    pub end_line: usize,
    pub is_public: bool,
    pub parameters: Vec<String>,
    pub return_type: Option<String>,
}

#[derive(Debug)]
pub struct StructSymbol {
    pub name: String,
    pub start_line: usize,
    pub end_line: usize,
    pub is_public: bool,
    pub fields: Vec<String>,
}

#[derive(Debug)]
pub struct TraitSymbol {
    pub name: String,
    pub start_line: usize,
    pub end_line: usize,
    pub is_public: bool,
}

#[derive(Debug)]
pub struct ImplSymbol {
    pub trait_name: Option<String>,
    pub type_name: String,
    pub start_line: usize,
    pub end_line: usize,
}

#[derive(Debug)]
pub struct ModuleSymbol {
    pub name: String,
    pub start_line: usize,
    pub end_line: usize,
    pub is_public: bool,
}

#[derive(Debug)]
pub struct ImportSymbol {
    pub path: String,
    pub line: usize,
}

/// Parser for Cairo source code
pub struct CairoParser {
    db: SimpleParserDatabase,
}

impl Default for CairoParser {
    fn default() -> Self {
        Self::new()
    }
}

impl CairoParser {
    /// Create a new Cairo parser
    pub fn new() -> Self {
        Self {
            db: SimpleParserDatabase::default(),
        }
    }

    /// Parse Cairo source code and extract symbols
    pub fn parse(&mut self, content: &str, _filename: &str) -> Result<CairoSymbols> {
        // Parse the content using parse_virtual_with_diagnostics
        let (syntax_node, _diagnostics) = self.db.parse_virtual_with_diagnostics(content);

        // Extract symbols from the AST
        let mut symbols = CairoSymbols::default();
        self.extract_symbols_from_node(&syntax_node, content, &mut symbols)?;

        Ok(symbols)
    }

    /// Recursively extract symbols from a syntax node
    fn extract_symbols_from_node(
        &self,
        node: &SyntaxNode,
        content: &str,
        symbols: &mut CairoSymbols,
    ) -> Result<()> {
        use SyntaxKind::*;

        match node.kind(&self.db) {
            FunctionWithBody => {
                let func = ast::FunctionWithBody::from_syntax_node(&self.db, *node);
                let name = func
                    .declaration(&self.db)
                    .name(&self.db)
                    .text(&self.db)
                    .to_string();
                let (start_line, end_line) = self.get_line_range(node, content);

                // Extract parameters
                let params = func
                    .declaration(&self.db)
                    .signature(&self.db)
                    .parameters(&self.db)
                    .elements(&self.db)
                    .map(|p| p.name(&self.db).text(&self.db).to_string())
                    .collect();

                // Extract return type if present - simplified approach
                let return_type = match func
                    .declaration(&self.db)
                    .signature(&self.db)
                    .ret_ty(&self.db)
                {
                    ast::OptionReturnTypeClause::Empty(_) => None,
                    ast::OptionReturnTypeClause::ReturnTypeClause(clause) => {
                        Some(clause.ty(&self.db).as_syntax_node().get_text(&self.db))
                    }
                };

                symbols.functions.push(FunctionSymbol {
                    name,
                    start_line,
                    end_line,
                    is_public: self.is_public(node),
                    parameters: params,
                    return_type,
                });
            }
            ItemStruct => {
                let struct_item = ast::ItemStruct::from_syntax_node(&self.db, *node);
                let name = struct_item.name(&self.db).text(&self.db).to_string();
                let (start_line, end_line) = self.get_line_range(node, content);

                // Extract field names - simplified approach for now
                let fields = vec![]; // TODO: Extract actual fields when API is clear

                symbols.structs.push(StructSymbol {
                    name,
                    start_line,
                    end_line,
                    is_public: self.is_public(node),
                    fields,
                });
            }
            ItemTrait => {
                let trait_item = ast::ItemTrait::from_syntax_node(&self.db, *node);
                let name = trait_item.name(&self.db).text(&self.db).to_string();
                let (start_line, end_line) = self.get_line_range(node, content);

                symbols.traits.push(TraitSymbol {
                    name,
                    start_line,
                    end_line,
                    is_public: self.is_public(node),
                });
            }
            ItemImpl => {
                let impl_item = ast::ItemImpl::from_syntax_node(&self.db, *node);
                let type_name = impl_item.name(&self.db).as_syntax_node().get_text(&self.db);
                let (start_line, end_line) = self.get_line_range(node, content);

                // Check if this is a trait impl - simplified approach
                let trait_name = Some(
                    impl_item
                        .trait_path(&self.db)
                        .as_syntax_node()
                        .get_text(&self.db),
                );

                symbols.impls.push(ImplSymbol {
                    trait_name,
                    type_name,
                    start_line,
                    end_line,
                });
            }
            ItemModule => {
                let module = ast::ItemModule::from_syntax_node(&self.db, *node);
                let name = module.name(&self.db).text(&self.db).to_string();
                let (start_line, end_line) = self.get_line_range(node, content);

                symbols.modules.push(ModuleSymbol {
                    name,
                    start_line,
                    end_line,
                    is_public: self.is_public(node),
                });
            }
            ItemUse => {
                let use_item = ast::ItemUse::from_syntax_node(&self.db, *node);
                let path = use_item
                    .use_path(&self.db)
                    .as_syntax_node()
                    .get_text(&self.db);
                let (line, _) = self.get_line_range(node, content);

                symbols.imports.push(ImportSymbol { path, line });
            }
            _ => {}
        }

        // Recursively process children
        for child in node.get_children(&self.db).iter() {
            self.extract_symbols_from_node(child, content, symbols)?;
        }

        Ok(())
    }

    /// Get the line range for a syntax node
    fn get_line_range(&self, node: &SyntaxNode, content: &str) -> (usize, usize) {
        // For now, return a simple approximation based on node text
        // This is a simplified approach until we can properly handle the new span API
        let node_text = node.get_text(&self.db);
        let node_lines = node_text.lines().count().max(1);

        // Try to find the node text in the content to estimate position
        if let Some(position) = content.find(&node_text) {
            let prefix = &content[..position];
            let start_line = prefix.lines().count();
            let end_line = start_line + node_lines - 1;
            (start_line, end_line)
        } else {
            // Fallback to 1-based line numbering if we can't find the text
            (1, node_lines)
        }
    }

    /// Check if a node has public visibility
    fn is_public(&self, node: &SyntaxNode) -> bool {
        // Check for 'pub' modifier in the node's text
        // This is a simplified check - could be enhanced
        node.get_text(&self.db).starts_with("pub ")
    }
}

/// Parse Cairo source code and extract symbols
pub fn parse_cairo(content: &str, filename: &str) -> Result<CairoSymbols> {
    let mut parser = CairoParser::new();
    parser.parse(content, filename)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_function() {
        let code = r#"
fn main() {
    println!("Hello, Cairo!");
}
"#;

        let symbols = parse_cairo(code, "test.cairo").unwrap();
        assert_eq!(symbols.functions.len(), 1);
        assert_eq!(symbols.functions[0].name, "main");
    }

    #[test]
    fn test_parse_struct() {
        let code = r#"
#[derive(Drop)]
struct Point {
    x: felt252,
    y: felt252,
}
"#;

        let symbols = parse_cairo(code, "test.cairo").unwrap();
        assert_eq!(symbols.structs.len(), 1);
        assert_eq!(symbols.structs[0].name, "Point");
        assert_eq!(symbols.structs[0].fields.len(), 2);
    }

    #[test]
    fn test_parse_trait() {
        let code = r#"
trait Display {
    fn fmt(self: @Self) -> ByteArray;
}
"#;

        let symbols = parse_cairo(code, "test.cairo").unwrap();
        assert_eq!(symbols.traits.len(), 1);
        assert_eq!(symbols.traits[0].name, "Display");
    }
}
