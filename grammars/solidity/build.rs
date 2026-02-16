use std::path::PathBuf;

fn main() {
    let dir = PathBuf::from("grammars/solidity");
    let src = dir.join("src");

    println!("cargo:rerun-if-changed=grammars/solidity/src/parser.c");

    cc::Build::new()
        .include(&src)
        .file(src.join("parser.c"))
        .flag_if_supported("-Wno-unused-parameter")
        .flag_if_supported("-Wno-unused-but-set-variable")
        .flag_if_supported("-Wno-trigraphs")
        .compile("tree-sitter-solidity");
}
