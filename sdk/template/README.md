# {{ child_name }}

Patina push-pure child generated from the Rust child template.

## Local diagnostics

Run the low-friction local-dev checks before building a component:

```sh
pai-dev children check local-dev .
```

If `pai-dev` is not installed yet, use the standalone diagnostics package fallback:

```sh
cargo test --manifest-path checks/diagnostics/Cargo.toml
```

These checks validate `child.toml`, `wit/`, and `.patina/children-dev.toml` without linking the WebAssembly component crate. They do not require a built `.wasm` component.

The generated child depends on the published `patina-sdk` crate. The standalone diagnostics package uses the Patina git repository until `patina-child-diagnostics` is published as its own crate; contributors working in a local Patina checkout may replace that dependency with a local `path` dependency.

## Build component

Build with your Rust component toolchain, then copy the final WebAssembly
component into the Patina dev artifact location:

```sh
mkdir -p .patina/dev/components
cp <built-component>.wasm .patina/dev/components/{{ child_name }}.wasm
```

Then run component-built diagnostics:

```sh
pai-dev children check component-built .
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

```sh
pai-dev children check release-candidate .
```

`.patina/dev/` is generated output and can be deleted safely.
