use tree_sitter::{Parser, Language};

fn main() {
    let code = r#"
impl MyStruct {
    fn new() -> Self {
        Self { value: 0 }
    }
    
    fn method_one(&self) {
        self.method_two();
    }
}"#;

    extern "C" { fn tree_sitter_rust() -> Language; }
    let language = unsafe { tree_sitter_rust() };
    
    let mut parser = Parser::new();
    parser.set_language(&language).unwrap();
    
    let tree = parser.parse(code, None).unwrap();
    let root = tree.root_node();
    
    print_tree(root, code.as_bytes(), 0);
}

fn print_tree(node: tree_sitter::Node, source: &[u8], indent: usize) {
    let kind = node.kind();
    let text = if node.child_count() == 0 {
        node.utf8_text(source).unwrap_or("").to_string()
    } else {
        String::new()
    };
    
    println!("{}{} {}", "  ".repeat(indent), kind, 
             if !text.is_empty() { format!("'{}'", text) } else { String::new() });
    
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        print_tree(child, source, indent + 1);
    }
}