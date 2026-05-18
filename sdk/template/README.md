# {{ child_name }}

Patina push-pure child generated from the Rust child template.

## Local diagnostics

Run the low-friction local-dev checks before building a component:

```sh
cargo test --manifest-path checks/diagnostics/Cargo.toml
```

These checks validate `child.toml`, `wit/`, and `.patina/children-dev.toml` from a standalone diagnostics package, so host-side diagnostics do not try to link the WebAssembly component crate. They do not require a built `.wasm` component.

## Build component

Build with your Rust component toolchain, then copy the final WebAssembly
component into the Patina dev artifact location:

```sh
mkdir -p .patina/dev/components
cp <built-component>.wasm .patina/dev/components/{{ child_name }}.wasm
```

Then run component-built diagnostics from a test or CI job:

```rust
patina_child_diagnostics::check_children_dev_components(".").assert_ok();
```

## Prepare release candidate

Create a local release-candidate bundle under `.patina/dev/releases/`:

```sh
mkdir -p .patina/dev/releases/{{ child_name }}
cp .patina/dev/components/{{ child_name }}.wasm .patina/dev/releases/{{ child_name }}/{{ child_name }}.wasm
cp child.toml .patina/dev/releases/{{ child_name }}/child.toml
shasum -a 256 .patina/dev/releases/{{ child_name }}/child.toml > .patina/dev/releases/{{ child_name }}/child.toml.sha256
(
  cd .patina/dev/releases/{{ child_name }}
  shasum -a 256 {{ child_name }}.wasm child.toml > checksums.txt
)
```

Then run release-candidate diagnostics:

```rust
patina_child_diagnostics::check_children_dev_release_candidates(".").assert_ok();
```

`.patina/dev/` is generated output and can be deleted safely.
