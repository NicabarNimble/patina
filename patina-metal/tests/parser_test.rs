use patina_metal::{Analyzer, Metal};

#[test]
fn test_rust_parser() {
    let mut analyzer = Analyzer::new().expect("Failed to create analyzer");

    if !analyzer.has_parser(Metal::Rust) {
        println!("Rust parser not available, skipping");
        return;
    }

    let source = r#"
fn hello() {
    println!("Hello, world!");
}
"#;
    let parsed = analyzer.parse(source, Metal::Rust);
    assert!(parsed.is_ok(), "Rust parser failed: {:?}", parsed.err());
}

#[test]
fn test_go_parser() {
    let mut analyzer = Analyzer::new().expect("Failed to create analyzer");

    if !analyzer.has_parser(Metal::Go) {
        println!("Go parser not available, skipping");
        return;
    }

    let source = r#"
package main

func hello() {
    fmt.Println("Hello, world!")
}
"#;
    let parsed = analyzer.parse(source, Metal::Go);
    assert!(parsed.is_ok(), "Go parser failed: {:?}", parsed.err());
}

#[test]
fn test_solidity_parser() {
    let mut analyzer = Analyzer::new().expect("Failed to create analyzer");

    if !analyzer.has_parser(Metal::Solidity) {
        println!("Solidity parser not available, skipping");
        return;
    }

    let source = r#"
contract HelloWorld {
    function hello() public pure returns (string memory) {
        return "Hello, world!";
    }
}
"#;
    let parsed = analyzer.parse(source, Metal::Solidity);
    assert!(parsed.is_ok(), "Solidity parser failed: {:?}", parsed.err());
}

#[test]
fn test_cairo_parser() {
    let mut analyzer = Analyzer::new().expect("Failed to create analyzer");

    if !analyzer.has_parser(Metal::Cairo) {
        println!("Cairo parser not available, skipping");
        return;
    }

    let source = r#"
fn hello() -> felt252 {
    'Hello, world!'
}
"#;
    let parsed = analyzer.parse(source, Metal::Cairo);
    assert!(parsed.is_ok(), "Cairo parser failed: {:?}", parsed.err());
}

#[test]
fn test_available_languages() {
    let analyzer = Analyzer::new().expect("Failed to create analyzer");

    println!("Available parsers: {:?}", analyzer.available_metals());

    for metal in Metal::all() {
        let available = analyzer.has_parser(metal);
        println!(
            "{:?}: {}",
            metal,
            if available {
                "✓ Available"
            } else {
                "✗ Not available"
            }
        );
    }

    // At least Rust and Go should work
    assert!(
        analyzer.has_parser(Metal::Rust),
        "Rust parser should be available"
    );
    assert!(
        analyzer.has_parser(Metal::Go),
        "Go parser should be available"
    );
}
