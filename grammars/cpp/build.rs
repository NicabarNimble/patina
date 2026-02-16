use std::path::PathBuf;

fn main() {
    let dir = PathBuf::from("grammars/cpp");
    let src = dir.join("src");

    println!("cargo:rerun-if-changed=grammars/cpp/src/parser.c");
    println!("cargo:rerun-if-changed=grammars/cpp/src/scanner.c");

    cc::Build::new()
        .include(&src)
        .file(src.join("parser.c"))
        .file(src.join("scanner.c"))
        .flag_if_supported("-Wno-unused-parameter")
        .flag_if_supported("-Wno-unused-but-set-variable")
        .flag_if_supported("-Wno-trigraphs")
        .compile("tree-sitter-cpp");
}
