use std::path::PathBuf;

fn main() {
    // Record grammar commits in the binary for reproducibility
    println!("cargo:rustc-env=RUST_GRAMMAR_COMMIT=6b7d1fc73ded57f73b1619bcf4371618212208b1");
    println!("cargo:rustc-env=GO_GRAMMAR_COMMIT=81a11f8252998ee6b98d59e6da91fc307491e53d");
    println!("cargo:rustc-env=PYTHON_GRAMMAR_COMMIT=710796b8b877a970297106e5bbc8e2afa47f86ec");
    println!("cargo:rustc-env=JAVASCRIPT_GRAMMAR_COMMIT=6fbef40512dcd9f0a61ce03a4c9ae7597b36ab5c");
    println!("cargo:rustc-env=TYPESCRIPT_GRAMMAR_COMMIT=75b3874edb2dc714fb1fd77a32013d0f8699989f");
    println!("cargo:rustc-env=SOLIDITY_GRAMMAR_COMMIT=c3da7d989747679305ec1c84b68082f01089d49f");
    println!("cargo:rustc-env=GRAMMAR_PACK_VERSION=1.0.0");
    println!("cargo:rustc-env=C_GRAMMAR_COMMIT=212bdfe7e69e7b1a1ee29aeb6d16a7d6128c1209");
    println!("cargo:rustc-env=CPP_GRAMMAR_COMMIT=2369fa8a2b81e16b62f087c59e223fdaa693cf77");

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

    // Build C grammar
    let c_dir = PathBuf::from("grammars/c");
    let c_src = c_dir.join("src");

    println!("cargo:rerun-if-changed=grammars/c/src/parser.c");

    cc::Build::new()
        .include(&c_src)
        .file(c_src.join("parser.c"))
        .flag_if_supported("-Wno-unused-parameter")
        .flag_if_supported("-Wno-unused-but-set-variable")
        .flag_if_supported("-Wno-trigraphs")
        .compile("tree-sitter-c");

    // Build C++ grammar
    let cpp_dir = PathBuf::from("grammars/cpp");
    let cpp_src = cpp_dir.join("src");

    println!("cargo:rerun-if-changed=grammars/cpp/src/parser.c");
    println!("cargo:rerun-if-changed=grammars/cpp/src/scanner.c");

    cc::Build::new()
        .include(&cpp_src)
        .file(cpp_src.join("parser.c"))
        .file(cpp_src.join("scanner.c"))
        .flag_if_supported("-Wno-unused-parameter")
        .flag_if_supported("-Wno-unused-but-set-variable")
        .flag_if_supported("-Wno-trigraphs")
        .compile("tree-sitter-cpp");
}
