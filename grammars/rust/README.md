# grammar-rust

Rust grammar extraction plugin for Patina pipeline. Uses tree-sitter-rust
compiled to WASM via wasi-sdk.

## Prerequisites

### wasi-sdk

Tree-sitter grammars contain C code that must be compiled to WASM.
[wasi-sdk](https://github.com/WebAssembly/wasi-sdk/releases) provides the
C compiler targeting wasm32-wasip2.

```bash
# Download wasi-sdk (macOS ARM64 example)
curl -LO https://github.com/WebAssembly/wasi-sdk/releases/download/wasi-sdk-30/wasi-sdk-30.0-arm64-macos.tar.gz
tar xzf wasi-sdk-30.0-arm64-macos.tar.gz
mv wasi-sdk-30.0-arm64-macos /path/to/wasi-sdk

# Set environment variables (add to .bashrc/.zshrc)
export WASI_SDK_PATH=/path/to/wasi-sdk
export CC_wasm32_wasip2=$WASI_SDK_PATH/bin/clang
export AR_wasm32_wasip2=$WASI_SDK_PATH/bin/llvm-ar
export CFLAGS_wasm32_wasip2="--sysroot=$WASI_SDK_PATH/share/wasi-sysroot"
```

### Rust WASM target

```bash
rustup target add wasm32-wasip2
```

## Configuration

Update `.cargo/config.toml` with your wasi-sdk paths, or set the env vars
above and simplify config to just:

```toml
[build]
target = "wasm32-wasip2"
```

## Build

```bash
cargo build --release
```

Produces `target/wasm32-wasip2/release/grammar_rust.wasm` (~1.4MB).

## Install

```bash
mkdir -p ~/.patina/pipeline/grammar-rust
cp target/wasm32-wasip2/release/grammar_rust.wasm ~/.patina/pipeline/grammar-rust/plugin.wasm
cp child.toml ~/.patina/pipeline/grammar-rust/
```

## Test

```bash
patina scrape  # .rs files dispatch to plugin automatically
```

## Performance

Benchmarked on 238-file Rust codebase (patina itself):

| Metric | Plugin | Built-in | Overhead |
|--------|--------|----------|----------|
| Time | 6.2s | 3.4s | 1.82x |
| Functions | 2292 | 2238 | +2.4% |
| Types | 948 | 588 | +61% (captures modules) |
| Call edges | 26333 | 26260 | +0.3% |

1.82x overhead is well within the 10x decision gate. The plugin extracts
slightly more data because it captures `mod` items as types (the built-in
processor doesn't).

## Architecture

- `build.rs` — compiles tree-sitter-rust parser.c + scanner.c via cc crate
- `src/lib.rs` — PipelinePlugin impl with full extraction logic ported from
  `src/commands/scrape/code/languages/rust.rs`
- `grammars/rust/src/` — vendored tree-sitter-rust C source (parser.c, scanner.c)
- `child.toml` — claims language "rs" for pipeline dispatch

The plugin defines its own serialization types matching the host's JSON
contract (per [[json-contract-over-shared-types]]), not shared Rust types.
