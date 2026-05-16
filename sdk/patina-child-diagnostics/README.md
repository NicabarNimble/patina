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

## Example

```rust
#[test]
fn child_package_conforms_locally() {
    patina_child_diagnostics::check_current_package()
        .assert_ok();
}
```
