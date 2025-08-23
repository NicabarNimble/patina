use anyhow::Result;
use std::fs;
use std::path::Path;
use tree_sitter::{Parser, Node};
use walkdir::WalkDir;

fn main() -> Result<()> {
    println!("🔍 Simple Semantic Analysis of Patina\n");
    
    let mut parser = Parser::new();
    let language = patina_metal::Metal::Rust.tree_sitter_language()
        .ok_or_else(|| anyhow::anyhow!("Rust parser not available"))?;
    parser.set_language(&language)?;
    
    let mut total_functions = 0;
    let mut total_structs = 0;
    let mut total_traits = 0;
    let mut total_impls = 0;
    let mut function_sizes = Vec::new();
    let mut error_functions = Vec::new();
    let mut public_functions = Vec::new();
    let mut test_functions = Vec::new();
    
    // Analyze all Rust files
    for entry in WalkDir::new("src")
        .follow_links(true)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        if entry.file_type().is_file() {
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) == Some("rs") {
                let content = fs::read_to_string(path)?;
                let tree = parser.parse(&content, None)
                    .ok_or_else(|| anyhow::anyhow!("Failed to parse {:?}", path))?;
                
                let file_name = path.to_str().unwrap_or("");
                let mut cursor = tree.root_node().walk();
                analyze_node(&mut cursor, &content, file_name, &mut total_functions, 
                            &mut total_structs, &mut total_traits, &mut total_impls,
                            &mut function_sizes, &mut error_functions, 
                            &mut public_functions, &mut test_functions);
            }
        }
    }
    
    println!("📊 Results:\n");
    println!("  Total functions: {}", total_functions);
    println!("  Total structs: {}", total_structs);
    println!("  Total traits: {}", total_traits);
    println!("  Total impls: {}", total_impls);
    
    println!("\n📏 Function Sizes (REAL, not file size!):");
    if !function_sizes.is_empty() {
        function_sizes.sort_by_key(|&(_, size)| std::cmp::Reverse(size));
        let avg_size: usize = function_sizes.iter().map(|(_, s)| s).sum::<usize>() / function_sizes.len();
        println!("  Average: {} lines", avg_size);
        println!("  Largest functions:");
        for (name, size) in function_sizes.iter().take(5) {
            println!("    - {}: {} lines", name, size);
        }
        
        let small_functions = function_sizes.iter().filter(|(_, s)| *s <= 10).count();
        let medium_functions = function_sizes.iter().filter(|(_, s)| *s > 10 && *s <= 50).count();
        let large_functions = function_sizes.iter().filter(|(_, s)| *s > 50).count();
        
        println!("\n  Distribution:");
        println!("    ≤10 lines: {} functions ({:.1}%)", small_functions, 
                small_functions as f32 / function_sizes.len() as f32 * 100.0);
        println!("    11-50 lines: {} functions ({:.1}%)", medium_functions,
                medium_functions as f32 / function_sizes.len() as f32 * 100.0);
        println!("    >50 lines: {} functions ({:.1}%)", large_functions,
                large_functions as f32 / function_sizes.len() as f32 * 100.0);
    }
    
    println!("\n⚠️  Error Handling:");
    println!("  Functions returning Result: {}", error_functions.len());
    let with_context = error_functions.iter()
        .filter(|(_, content)| content.contains(".context(") || content.contains(".with_context("))
        .count();
    println!("  Functions using .context(): {}", with_context);
    let with_question = error_functions.iter()
        .filter(|(_, content)| content.contains("?"))
        .count();
    println!("  Functions using ?: {}", with_question);
    
    println!("\n🚪 API Surface:");
    println!("  Public functions: {}", public_functions.len());
    let public_with_result = public_functions.iter()
        .filter(|name| error_functions.iter().any(|(n, _)| n == *name))
        .count();
    println!("  Public functions returning Result: {}", public_with_result);
    
    println!("\n🧪 Testing:");
    println!("  Test functions: {}", test_functions.len());
    let test_ratio = test_functions.len() as f32 / total_functions as f32 * 100.0;
    println!("  Test ratio: {:.1}%", test_ratio);
    
    Ok(())
}

fn analyze_node(
    cursor: &mut tree_sitter::TreeCursor,
    source: &str,
    file_name: &str,
    total_functions: &mut usize,
    total_structs: &mut usize,
    total_traits: &mut usize,
    total_impls: &mut usize,
    function_sizes: &mut Vec<(String, usize)>,
    error_functions: &mut Vec<(String, String)>,
    public_functions: &mut Vec<String>,
    test_functions: &mut Vec<String>,
) {
    let node = cursor.node();
    
    match node.kind() {
        "function_item" => {
            *total_functions += 1;
            
            if let Some(name_node) = node.child_by_field_name("name") {
                let name = name_node.utf8_text(source.as_bytes()).unwrap_or("").to_string();
                
                // Get REAL function size
                let start_line = node.start_position().row;
                let end_line = node.end_position().row;
                let size = end_line - start_line + 1;
                function_sizes.push((format!("{}::{}", file_name, name), size));
                
                // Check if it returns Result
                if let Some(return_type) = node.child_by_field_name("return_type") {
                    let return_text = return_type.utf8_text(source.as_bytes()).unwrap_or("");
                    if return_text.contains("Result") {
                        let body = node.utf8_text(source.as_bytes()).unwrap_or("");
                        error_functions.push((name.clone(), body.to_string()));
                    }
                }
                
                // Check if public
                if let Some(vis) = node.child_by_field_name("visibility") {
                    if vis.utf8_text(source.as_bytes()).unwrap_or("") == "pub" {
                        public_functions.push(name.clone());
                    }
                }
                
                // Check if test
                if name.starts_with("test_") || is_test_function(&node, source) {
                    test_functions.push(name);
                }
            }
        }
        "struct_item" => {
            *total_structs += 1;
        }
        "trait_item" => {
            *total_traits += 1;
        }
        "impl_item" => {
            *total_impls += 1;
        }
        _ => {}
    }
    
    // Recurse
    if cursor.goto_first_child() {
        loop {
            analyze_node(cursor, source, file_name, total_functions, total_structs, 
                        total_traits, total_impls, function_sizes, error_functions,
                        public_functions, test_functions);
            if !cursor.goto_next_sibling() {
                break;
            }
        }
        cursor.goto_parent();
    }
}

fn is_test_function(node: &Node, source: &str) -> bool {
    // Check for #[test] attribute
    if let Some(prev) = node.prev_sibling() {
        if prev.kind() == "attribute_item" {
            let attr_text = prev.utf8_text(source.as_bytes()).unwrap_or("");
            return attr_text.contains("#[test]") || attr_text.contains("#[tokio::test]");
        }
    }
    false
}