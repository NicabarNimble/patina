use std::path::PathBuf;

fn main() {
    // Build Rust grammar
    let rust_dir = PathBuf::from("grammars/rust");
    let rust_src = rust_dir.join("src");

    println!("cargo:rerun-if-changed=grammars/rust/src/parser.c");
    println!("cargo:rerun-if-changed=grammars/rust/src/scanner.c");

    cc::Build::new()
        .include(&rust_src)
        .file(rust_src.join("parser.c"))
        .file(rust_src.join("scanner.c"))
        .flag_if_supported("-Wno-unused-parameter")
        .flag_if_supported("-Wno-unused-but-set-variable")
        .flag_if_supported("-Wno-trigraphs")
        .compile("tree-sitter-rust");

    // Build Go grammar
    let go_dir = PathBuf::from("grammars/go");
    let go_src = go_dir.join("src");

    println!("cargo:rerun-if-changed=grammars/go/src/parser.c");

    cc::Build::new()
        .include(&go_src)
        .file(go_src.join("parser.c"))
        .flag_if_supported("-Wno-unused-parameter")
        .flag_if_supported("-Wno-unused-but-set-variable")
        .flag_if_supported("-Wno-trigraphs")
        .compile("tree-sitter-go");

    // Build Solidity grammar
    let solidity_dir = PathBuf::from("grammars/solidity");
    let solidity_src = solidity_dir.join("src");

    println!("cargo:rerun-if-changed=grammars/solidity/src/parser.c");

    cc::Build::new()
        .include(&solidity_src)
        .file(solidity_src.join("parser.c"))
        .flag_if_supported("-Wno-unused-parameter")
        .flag_if_supported("-Wno-unused-but-set-variable")
        .flag_if_supported("-Wno-trigraphs")
        .compile("tree-sitter-solidity");

    // Build Python grammar
    let python_dir = PathBuf::from("grammars/python");
    let python_src = python_dir.join("src");

    println!("cargo:rerun-if-changed=grammars/python/src/parser.c");
    println!("cargo:rerun-if-changed=grammars/python/src/scanner.c");

    cc::Build::new()
        .include(&python_src)
        .file(python_src.join("parser.c"))
        .file(python_src.join("scanner.c"))
        .flag_if_supported("-Wno-unused-parameter")
        .flag_if_supported("-Wno-unused-but-set-variable")
        .flag_if_supported("-Wno-trigraphs")
        .compile("tree-sitter-python");

    // Build JavaScript grammar
    let javascript_dir = PathBuf::from("grammars/javascript");
    let javascript_src = javascript_dir.join("src");

    println!("cargo:rerun-if-changed=grammars/javascript/src/parser.c");
    println!("cargo:rerun-if-changed=grammars/javascript/src/scanner.c");

    cc::Build::new()
        .include(&javascript_src)
        .file(javascript_src.join("parser.c"))
        .file(javascript_src.join("scanner.c"))
        .flag_if_supported("-Wno-unused-parameter")
        .flag_if_supported("-Wno-unused-but-set-variable")
        .flag_if_supported("-Wno-trigraphs")
        .compile("tree-sitter-javascript");

    // Build TypeScript grammar
    let typescript_dir = PathBuf::from("grammars/typescript/typescript");
    let typescript_src = typescript_dir.join("src");

    println!("cargo:rerun-if-changed=grammars/typescript/typescript/src/parser.c");
    println!("cargo:rerun-if-changed=grammars/typescript/typescript/src/scanner.c");

    cc::Build::new()
        .include(&typescript_src)
        .file(typescript_src.join("parser.c"))
        .file(typescript_src.join("scanner.c"))
        .flag_if_supported("-Wno-unused-parameter")
        .flag_if_supported("-Wno-unused-but-set-variable")
        .flag_if_supported("-Wno-trigraphs")
        .compile("tree-sitter-typescript");

    // Build TSX grammar
    let tsx_dir = PathBuf::from("grammars/typescript/tsx");
    let tsx_src = tsx_dir.join("src");

    println!("cargo:rerun-if-changed=grammars/typescript/tsx/src/parser.c");
    println!("cargo:rerun-if-changed=grammars/typescript/tsx/src/scanner.c");

    cc::Build::new()
        .include(&tsx_src)
        .file(tsx_src.join("parser.c"))
        .file(tsx_src.join("scanner.c"))
        .flag_if_supported("-Wno-unused-parameter")
        .flag_if_supported("-Wno-unused-but-set-variable")
        .flag_if_supported("-Wno-trigraphs")
        .compile("tree-sitter-tsx");
}
