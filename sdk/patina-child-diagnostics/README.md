# patina-child-diagnostics

SDK-adjacent diagnostics for Patina child packages.

This crate is for child developer tooling and tests. It is intentionally not
linked into the main `patina` binary.

## Current scope

The first diagnostic slice covers the local development stage:

- `child.toml` identity and needs shape
- legacy manifest schema rejection
- WIT package/world resolution
- WIT imports/exports
- WIT toy imports compared to `[needs].toys`

The component-built stage also inspects an explicit WebAssembly component artifact and compares its top-level imports/exports/toy imports to WIT plus `child.toml` declarations.

The release-candidate stage checks local release bundle evidence: `.wasm`, `child.toml`, `child.toml.sha256`, `checksums.txt`, checksum coverage, checksum matches, and optional tag/version alignment.

## Single-child package example

```rust
#[test]
fn child_package_conforms_locally() {
    patina_child_diagnostics::check_current_package()
        .assert_ok();
}
```

## Repo-local children dev config

Multi-language and multi-child repos can commit a repo-local dev/check harness config:

```text
.patina/children-dev.toml
```

Example:

```toml
[children.folder-watch-actor]
root = "children/folder-watch-actor"
component = ".patina/dev/components/folder-watch-actor.wasm"
release = ".patina/dev/releases/folder-watch-actor"
tag = "folder-watch-actor-v0.1.0"

[children.watch-null-sink]
root = "children/watch-null-sink"
component = ".patina/dev/components/watch-null-sink.wasm"
release = ".patina/dev/releases/watch-null-sink"
tag = "watch-null-sink-v0.1.0"
```

The `component`, `release`, and `tag` fields are optional for local-dev checks. `component` is required for component-built checks. `component` and `release` are required for release-candidate checks; `tag` enables tag/version alignment diagnostics. Generated SDK dev artifacts should live under `.patina/dev/` so they can be cleaned without knowing whether the child was built by Rust, C, Go, TypeScript, or another toolchain.

```rust
#[test]
fn children_dev_config_conforms_locally() {
    patina_child_diagnostics::check_children_dev_config(".")
        .assert_ok();
}

#[test]
fn built_components_match_declared_contracts() {
    patina_child_diagnostics::check_children_dev_components(".")
        .assert_ok();
}

#[test]
fn release_candidates_have_installable_assets() {
    patina_child_diagnostics::check_children_dev_release_candidates(".")
        .assert_ok();
}
```

For lower-level tests or CI jobs that already know the artifact paths:

```rust
#[test]
fn built_component_matches_declared_contract() {
    patina_child_diagnostics::check_component_built(
        ".",
        ".patina/dev/components/my-child.wasm",
    )
    .assert_ok();
}

#[test]
fn release_candidate_matches_mother_intake() {
    patina_child_diagnostics::check_release_candidate_with_tag(
        ".",
        ".patina/dev/components/my-child.wasm",
        ".patina/dev/releases/my-child",
        "my-child-v0.1.0",
    )
    .assert_ok();
}
```
