use std::path::PathBuf;

fn main() {
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
}
