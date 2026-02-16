use std::path::PathBuf;

fn main() {
    // TypeScript grammar has two separate parsers: typescript and tsx
    let ts_dir = PathBuf::from("grammars/typescript");
    let ts_src = ts_dir.join("src");
    let tsx_dir = PathBuf::from("grammars/tsx");
    let tsx_src = tsx_dir.join("src");

    println!("cargo:rerun-if-changed=grammars/typescript/src/parser.c");
    println!("cargo:rerun-if-changed=grammars/typescript/src/scanner.c");
    println!("cargo:rerun-if-changed=grammars/tsx/src/parser.c");
    println!("cargo:rerun-if-changed=grammars/tsx/src/scanner.c");
    println!("cargo:rerun-if-changed=grammars/common/scanner.h");

    // Build TypeScript parser
    cc::Build::new()
        .include(&ts_src)
        .file(ts_src.join("parser.c"))
        .file(ts_src.join("scanner.c"))
        .flag_if_supported("-Wno-unused-parameter")
        .flag_if_supported("-Wno-unused-but-set-variable")
        .flag_if_supported("-Wno-trigraphs")
        .compile("tree-sitter-typescript");

    // Build TSX parser
    cc::Build::new()
        .include(&tsx_src)
        .file(tsx_src.join("parser.c"))
        .file(tsx_src.join("scanner.c"))
        .flag_if_supported("-Wno-unused-parameter")
        .flag_if_supported("-Wno-unused-but-set-variable")
        .flag_if_supported("-Wno-trigraphs")
        .compile("tree-sitter-tsx");
}
