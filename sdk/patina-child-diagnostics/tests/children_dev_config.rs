use std::collections::BTreeSet;
use std::path::PathBuf;

use patina_child_diagnostics::{
    check_children_dev_config, children_dev_config_path, load_children_dev_config,
};

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name)
}

#[test]
fn loads_single_child_dev_config() {
    let root = fixture("single-child-dev-config");
    let config = load_children_dev_config(&root).expect("children-dev config loads");

    assert_eq!(
        children_dev_config_path(&root),
        root.join(".patina/children-dev.toml")
    );
    assert_eq!(config.children.len(), 1);
    assert_eq!(config.children["single-child"].root, PathBuf::from("."));
    assert_eq!(config.children["single-child"].component, None);
}

#[test]
fn single_child_dev_config_runs_local_dev_without_component() {
    let root = fixture("single-child-dev-config");
    let report = check_children_dev_config(&root);

    assert!(report.is_ok(), "{}", report.render_text());
    assert!(report.findings.is_empty(), "{}", report.render_text());
    assert_eq!(report.children.len(), 1);
    assert_eq!(report.children[0].name, "single-child");
    assert_eq!(report.children[0].root, root);
    assert_eq!(report.children[0].component, None);
}

#[test]
fn multi_child_dev_config_resolves_roots_and_components() {
    let root = fixture("multi-child-dev-config");
    let report = check_children_dev_config(&root);

    assert!(report.is_ok(), "{}", report.render_text());
    assert!(report.findings.is_empty(), "{}", report.render_text());
    assert_eq!(report.children.len(), 2);

    let names = report
        .children
        .iter()
        .map(|child| child.name.as_str())
        .collect::<BTreeSet<_>>();
    assert_eq!(names, BTreeSet::from(["alpha-child", "beta-child"]));

    let alpha = report
        .children
        .iter()
        .find(|child| child.name == "alpha-child")
        .expect("alpha child report");
    assert_eq!(
        alpha.component.as_deref(),
        Some(
            root.join(".patina/dev/components/alpha-child.wasm")
                .as_path()
        )
    );

    let beta = report
        .children
        .iter()
        .find(|child| child.name == "beta-child")
        .expect("beta child report");
    assert_eq!(
        beta.component.as_deref(),
        Some(
            root.join(".patina/dev/components/beta-child.wasm")
                .as_path()
        )
    );
}
