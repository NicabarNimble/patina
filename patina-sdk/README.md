# patina-sdk

SDK for building [Patina](https://github.com/NicabarNimble/patina) WASM plugins.

Patina plugins are WebAssembly Component Model (WASIp2) binaries that extend
the `patina` CLI and Mother daemon. Each plugin selects a **world** via feature
flag — the world determines what host capabilities are available.

## Quick Start

```toml
# Cargo.toml
[dependencies]
patina-sdk = { version = "0.21", features = ["task"] }
```

```rust
use patina_sdk::{register_task, TaskPlugin, Toy};

#[derive(Default)]
struct MyPlugin;

impl TaskPlugin for MyPlugin {
    fn name(&self) -> String { "my-plugin".into() }
    fn description(&self) -> String { "Does things".into() }
    fn run(&mut self, _args: &[String]) -> i32 { 0 }
}

register_task!(MyPlugin);
```

```sh
cargo build --target wasm32-wasip2
```

## Worlds

| Feature | World | Use Case | Host Capabilities |
|---------|-------|----------|-------------------|
| `task` | Task | On-demand actions (PR review, deploy) | log, layer, query, HTTP, toys |
| `command` | Command | CLI subcommands (`patina doctor`) | log, layer, query |
| `pipeline` | Pipeline | Pure compute (parse, chunk, tokenize) | log only |
| `mother-child` | Mother-Child | Daemon-resident services | log, layer, query, HTTP, toys, heartbeat |

Enable **exactly one** feature per plugin crate. The compiler enforces this on
wasm32 targets.

## Scaffold

The fastest way to start:

```sh
patina plugin init my-plugin --world task
```

This generates a complete plugin project with the correct SDK dependency.

## License

MIT
