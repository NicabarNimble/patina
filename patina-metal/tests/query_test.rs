use patina_metal::{Analyzer, Metal, QueryType};

#[test]
fn test_rust_symbol_query() {
    let mut analyzer = Analyzer::new().expect("Failed to create analyzer");

    if !analyzer.has_parser(Metal::Rust) {
        println!("Rust parser not available, skipping");
        return;
    }

    let source = r#"
pub struct Config {
    name: String,
}

impl Config {
    pub fn new(name: String) -> Self {
        Config { name }
    }
    
    pub fn with_default() -> Self {
        Config { name: "default".to_string() }
    }
}

pub trait Configurable {
    fn configure(&mut self);
}
"#;

    let parsed = analyzer
        .parse(source, Metal::Rust)
        .expect("Failed to parse");
    let matches = analyzer
        .run_query(&parsed, QueryType::Symbols)
        .expect("Failed to run query");

    // Check we found the expected symbols
    let functions: Vec<_> = matches
        .iter()
        .filter(|m| m.capture_name.contains("function") || m.capture_name.contains("method"))
        .map(|m| m.text.as_str())
        .collect();

    assert!(functions.contains(&"new"));
    assert!(functions.contains(&"with_default"));

    let structs: Vec<_> = matches
        .iter()
        .filter(|m| m.capture_name.contains("struct"))
        .map(|m| m.text.as_str())
        .collect();

    assert!(structs.contains(&"Config"));

    println!("Found {} symbols", matches.len());
    for m in &matches {
        println!(
            "  {} = '{}' (lines {}-{})",
            m.capture_name,
            m.text,
            m.start_line + 1,
            m.end_line + 1
        );
    }
}

#[test]
fn test_go_symbol_query() {
    let mut analyzer = Analyzer::new().expect("Failed to create analyzer");

    if !analyzer.has_parser(Metal::Go) {
        println!("Go parser not available, skipping");
        return;
    }

    let source = r#"
package main

type Server struct {
    port int
}

func NewServer(port int) *Server {
    return &Server{port: port}
}

func (s *Server) Start() error {
    return nil
}

type Handler interface {
    Handle() error
}
"#;

    let parsed = analyzer.parse(source, Metal::Go).expect("Failed to parse");
    let matches = analyzer
        .run_query(&parsed, QueryType::Symbols)
        .expect("Failed to run query");

    let functions: Vec<_> = matches
        .iter()
        .filter(|m| m.capture_name.contains("function") || m.capture_name.contains("method"))
        .map(|m| m.text.as_str())
        .collect();

    assert!(functions.contains(&"NewServer"));
    assert!(functions.contains(&"Start"));

    println!("Found {} symbols", matches.len());
    for m in &matches {
        println!(
            "  {} = '{}' (lines {}-{})",
            m.capture_name,
            m.text,
            m.start_line + 1,
            m.end_line + 1
        );
    }
}

#[test]
fn test_rust_complexity_query() {
    let mut analyzer = Analyzer::new().expect("Failed to create analyzer");

    if !analyzer.has_parser(Metal::Rust) {
        println!("Rust parser not available, skipping");
        return;
    }

    let source = r#"
fn complex_logic(n: i32) -> i32 {
    if n < 0 {
        return -1;
    }
    
    match n {
        0 => 0,
        1 => 1,
        _ => {
            let mut sum = 0;
            for i in 0..n {
                if i % 2 == 0 {
                    sum += i;
                } else {
                    sum -= i;
                }
            }
            sum
        }
    }
}
"#;

    let parsed = analyzer
        .parse(source, Metal::Rust)
        .expect("Failed to parse");
    let matches = analyzer
        .run_query(&parsed, QueryType::Complexity)
        .expect("Failed to run query");

    let branches = matches
        .iter()
        .filter(|m| m.capture_name.contains("branch"))
        .count();

    println!("Found {} complexity points", matches.len());
    println!("Branches: {}", branches);

    assert!(branches > 0, "Should find complexity branches");
}
