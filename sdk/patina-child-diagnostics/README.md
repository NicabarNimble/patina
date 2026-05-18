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

It does not yet inspect built WASM components or release assets.

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

[children.watch-null-sink]
root = "children/watch-null-sink"
component = ".patina/dev/components/watch-null-sink.wasm"
```

The `component` field is optional for local-dev checks. Generated SDK dev artifacts should live under `.patina/dev/` so they can be cleaned without knowing whether the child was built by Rust, C, Go, TypeScript, or another toolchain.

```rust
#[test]
fn children_dev_config_conforms_locally() {
    patina_child_diagnostics::check_children_dev_config(".")
        .assert_ok();
}
```
