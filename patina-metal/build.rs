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
}
