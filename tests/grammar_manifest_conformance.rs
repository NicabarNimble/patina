use std::path::PathBuf;

#[test]
fn javascript_grammar_manifest_claims_js_jsx_and_mjs() {
    let manifest_path =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("grammars/javascript/child.toml");
    assert!(
        manifest_path.exists(),
        "missing {}",
        manifest_path.display()
    );

    let content = std::fs::read_to_string(&manifest_path)
        .unwrap_or_else(|e| panic!("read {}: {e}", manifest_path.display()));
    let table: toml::Table = content
        .parse()
        .unwrap_or_else(|e| panic!("parse {}: {e}", manifest_path.display()));

    let languages = table
        .get("provides")
        .and_then(|v| v.as_table())
        .and_then(|t| t.get("languages"))
        .and_then(|v| v.as_array())
        .expect("[provides].languages must be present");

    let as_strings = languages
        .iter()
        .filter_map(|v| v.as_str())
        .collect::<Vec<_>>();

    for expected in ["js", "jsx", "mjs"] {
        assert!(
            as_strings.iter().any(|value| value == &expected),
            "missing '{}' in javascript grammar languages: {:?}",
            expected,
            as_strings
        );
    }
}
