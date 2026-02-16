use std::path::PathBuf;

fn main() {
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
}
