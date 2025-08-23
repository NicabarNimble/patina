use anyhow::Result;
use std::collections::HashMap;
use tree_sitter::{Parser, Node};

use super::queries::{SemanticQueries, SemanticPattern, extract_semantic_meaning};

/// Deep semantic analysis that understands code behavior
pub struct DeepAnalyzer {
    parser: Parser,
    queries: SemanticQueries,
    call_graph: HashMap<String, Vec<String>>,
    error_flows: HashMap<String, ErrorFlow>,
}

#[derive(Debug, Clone)]
pub struct ErrorFlow {
    pub function: String,
    pub propagates_errors: bool,
    pub adds_context: Vec<String>,
    pub error_sources: Vec<String>,
    pub error_sinks: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct FunctionSemantics {
    pub name: String,
    pub file: String,
    pub patterns: Vec<SemanticPattern>,
    pub calls: Vec<String>,
    pub called_by: Vec<String>,
    pub error_flow: Option<ErrorFlow>,
    pub actual_line_count: usize,  // Real function size, not file size!
    pub cognitive_complexity: usize,
    pub test_coverage: bool,
}

impl DeepAnalyzer {
    pub fn new() -> Result<Self> {
        let mut parser = Parser::new();
        let language = patina_metal::Metal::Rust.tree_sitter_language()
            .ok_or_else(|| anyhow::anyhow!("Rust parser not available"))?;
        parser.set_language(&language)?;
        
        Ok(Self {
            queries: SemanticQueries::new(language)?,
            parser,
            call_graph: HashMap::new(),
            error_flows: HashMap::new(),
        })
    }
    
    /// Analyze a function's actual behavior
    pub fn analyze_function(&mut self, source: &str, file: &str) -> Result<Vec<FunctionSemantics>> {
        let tree = self.parser.parse(source, None)
            .ok_or_else(|| anyhow::anyhow!("Failed to parse"))?;
        
        let mut functions = Vec::new();
        let mut cursor = tree.root_node().walk();
        
        self.extract_functions(&mut cursor, source, file, &mut functions);
        self.build_call_graph(&functions);
        self.trace_error_flows(&functions);
        
        Ok(functions)
    }
    
    fn extract_functions(
        &self,
        cursor: &mut tree_sitter::TreeCursor,
        source: &str,
        file: &str,
        functions: &mut Vec<FunctionSemantics>,
    ) {
        let node = cursor.node();
        
        if node.kind() == "function_item" {
            if let Some(name_node) = node.child_by_field_name("name") {
                let name = name_node.utf8_text(source.as_bytes()).unwrap_or("").to_string();
                
                // Get ACTUAL function line count
                let start_line = node.start_position().row;
                let end_line = node.end_position().row;
                let actual_line_count = end_line - start_line + 1;
                
                // Extract semantic patterns
                let patterns = extract_semantic_meaning(node, source);
                
                // Extract function calls
                let calls = self.extract_function_calls(&node, source);
                
                // Calculate cognitive complexity (not just cyclomatic)
                let cognitive_complexity = self.calculate_cognitive_complexity(&node, source);
                
                // Check if this is a test
                let is_test = self.is_test_function(&node, source);
                
                functions.push(FunctionSemantics {
                    name: name.clone(),
                    file: file.to_string(),
                    patterns,
                    calls,
                    called_by: Vec::new(), // Will be filled by build_call_graph
                    error_flow: None,      // Will be filled by trace_error_flows
                    actual_line_count,
                    cognitive_complexity,
                    test_coverage: is_test,
                });
            }
        }
        
        // Recurse to children
        if cursor.goto_first_child() {
            loop {
                self.extract_functions(cursor, source, file, functions);
                if !cursor.goto_next_sibling() {
                    break;
                }
            }
            cursor.goto_parent();
        }
    }
    
    fn extract_function_calls(&self, node: &Node, source: &str) -> Vec<String> {
        let mut calls = Vec::new();
        let mut cursor = node.walk();
        
        self.find_calls_recursive(&mut cursor, source, &mut calls);
        calls
    }
    
    fn find_calls_recursive(
        &self,
        cursor: &mut tree_sitter::TreeCursor,
        source: &str,
        calls: &mut Vec<String>,
    ) {
        let node = cursor.node();
        
        match node.kind() {
            "call_expression" => {
                if let Some(function) = node.child_by_field_name("function") {
                    let call_text = function.utf8_text(source.as_bytes()).unwrap_or("");
                    calls.push(call_text.to_string());
                }
            }
            "method_call_expression" => {
                if let Some(method) = node.child_by_field_name("method") {
                    let method_text = method.utf8_text(source.as_bytes()).unwrap_or("");
                    calls.push(method_text.to_string());
                }
            }
            _ => {}
        }
        
        if cursor.goto_first_child() {
            loop {
                self.find_calls_recursive(cursor, source, calls);
                if !cursor.goto_next_sibling() {
                    break;
                }
            }
            cursor.goto_parent();
        }
    }
    
    fn calculate_cognitive_complexity(&self, node: &Node, source: &str) -> usize {
        let mut complexity = 0;
        let mut nesting_level = 0;
        let mut cursor = node.walk();
        
        self.calculate_complexity_recursive(&mut cursor, &mut complexity, &mut nesting_level);
        complexity
    }
    
    fn calculate_complexity_recursive(
        &self,
        cursor: &mut tree_sitter::TreeCursor,
        complexity: &mut usize,
        nesting_level: &mut usize,
    ) {
        let node = cursor.node();
        
        // Cognitive complexity considers nesting depth
        match node.kind() {
            "if_expression" | "match_expression" => {
                *complexity += 1 + *nesting_level;
                *nesting_level += 1;
            }
            "while_expression" | "for_expression" | "loop_expression" => {
                *complexity += 1 + *nesting_level;
                *nesting_level += 1;
            }
            "binary_expression" => {
                // && and || add complexity
                if let Some(op) = node.child_by_field_name("operator") {
                    let op_text = op.kind();
                    if op_text == "&&" || op_text == "||" {
                        *complexity += 1;
                    }
                }
            }
            "return_expression" | "break_expression" | "continue_expression" => {
                // Early returns add complexity
                *complexity += 1;
            }
            _ => {}
        }
        
        if cursor.goto_first_child() {
            loop {
                self.calculate_complexity_recursive(cursor, complexity, nesting_level);
                if !cursor.goto_next_sibling() {
                    break;
                }
            }
            cursor.goto_parent();
            
            // Decrease nesting when leaving a block
            match node.kind() {
                "if_expression" | "match_expression" | 
                "while_expression" | "for_expression" | "loop_expression" => {
                    *nesting_level = nesting_level.saturating_sub(1);
                }
                _ => {}
            }
        }
    }
    
    fn is_test_function(&self, node: &Node, source: &str) -> bool {
        // Check for #[test] attribute
        if let Some(prev) = node.prev_sibling() {
            if prev.kind() == "attribute_item" {
                let attr_text = prev.utf8_text(source.as_bytes()).unwrap_or("");
                return attr_text.contains("#[test]") || attr_text.contains("#[tokio::test]");
            }
        }
        
        // Check if function name starts with test_
        if let Some(name) = node.child_by_field_name("name") {
            let name_text = name.utf8_text(source.as_bytes()).unwrap_or("");
            return name_text.starts_with("test_");
        }
        
        false
    }
    
    fn build_call_graph(&mut self, functions: &[FunctionSemantics]) {
        // Build reverse mapping: who calls whom
        for func in functions {
            for called in &func.calls {
                self.call_graph
                    .entry(called.clone())
                    .or_insert_with(Vec::new)
                    .push(func.name.clone());
            }
        }
    }
    
    fn trace_error_flows(&mut self, functions: &[FunctionSemantics]) {
        for func in functions {
            let mut flow = ErrorFlow {
                function: func.name.clone(),
                propagates_errors: false,
                adds_context: Vec::new(),
                error_sources: Vec::new(),
                error_sinks: Vec::new(),
            };
            
            // Check patterns for error handling
            for pattern in &func.patterns {
                if let SemanticPattern::ErrorPropagation { propagates, adds_context, .. } = pattern {
                    flow.propagates_errors = *propagates;
                    if *adds_context {
                        flow.adds_context.push("context".to_string());
                    }
                }
            }
            
            // Track error sources (functions that might return errors)
            for call in &func.calls {
                if call.ends_with("?") || call.contains("unwrap") || call.contains("expect") {
                    flow.error_sources.push(call.clone());
                }
            }
            
            self.error_flows.insert(func.name.clone(), flow);
        }
    }
    
    /// Generate insights about the code
    pub fn generate_insights(&self, functions: &[FunctionSemantics]) -> SemanticInsights {
        let total_functions = functions.len();
        
        let complex_functions: Vec<_> = functions.iter()
            .filter(|f| f.cognitive_complexity > 10)
            .map(|f| (f.name.clone(), f.cognitive_complexity))
            .collect();
        
        let large_functions: Vec<_> = functions.iter()
            .filter(|f| f.actual_line_count > 50)
            .map(|f| (f.name.clone(), f.actual_line_count))
            .collect();
        
        let api_boundaries: Vec<_> = functions.iter()
            .filter(|f| f.patterns.iter().any(|p| {
                matches!(p, SemanticPattern::ApiBoundary { is_public: true, .. })
            }))
            .map(|f| f.name.clone())
            .collect();
        
        let error_handlers: Vec<_> = functions.iter()
            .filter(|f| f.patterns.iter().any(|p| {
                matches!(p, SemanticPattern::ErrorPropagation { adds_context: true, .. })
            }))
            .map(|f| f.name.clone())
            .collect();
        
        let test_functions = functions.iter()
            .filter(|f| f.test_coverage)
            .count();
        
        SemanticInsights {
            total_functions,
            complex_functions,
            large_functions,
            api_boundaries,
            error_handlers,
            test_coverage_ratio: test_functions as f32 / total_functions as f32,
        }
    }
}

#[derive(Debug)]
pub struct SemanticInsights {
    pub total_functions: usize,
    pub complex_functions: Vec<(String, usize)>,
    pub large_functions: Vec<(String, usize)>,
    pub api_boundaries: Vec<String>,
    pub error_handlers: Vec<String>,
    pub test_coverage_ratio: f32,
}