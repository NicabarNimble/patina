# patina-sdk

`patina-sdk` is the single SDK surface for Patina child authoring.

Doctrine lock:

- Mother owns authority, grants, continuity, and orchestration.
- Children own workflow agency.
- Toys are granted capability contracts.

## Quick Start (knowledge-child)

```toml
[dependencies]
patina-sdk = { version = "0.21", features = ["knowledge-child"] }
```

```rust
use patina_sdk::granted::{self, Bundle as GrantedBundle};
use patina_sdk::knowledge_child::{ChildHealth, HealthStatus, KnowledgeChildPlugin};
use patina_sdk::register_knowledge_child;

#[derive(Clone)]
struct Toys {
    log: granted::Log,
}

impl GrantedBundle for Toys {
    fn granted() -> Self {
        Self { log: granted::log() }
    }
}

#[derive(Default)]
struct MyChild {
    toys: Option<Toys>,
}

impl KnowledgeChildPlugin for MyChild {
    fn name(&self) -> String { "my-child".into() }
    fn on_load(&mut self) -> Result<(), String> {
        let toys = Toys::granted();
        toys.log.info("loaded");
        self.toys = Some(toys);
        Ok(())
    }
    fn health(&self) -> ChildHealth {
        ChildHealth { status: HealthStatus::Healthy, reason: None }
    }
    fn handle(&mut self, _action: &str, _payload: &str) -> Result<String, String> {
        Ok("{}".into())
    }
}

register_knowledge_child!(MyChild);
```

Build:

```sh
cargo build --target wasm32-wasip2
```

## Feature Worlds

`patina-sdk` currently supports these feature worlds:

- `knowledge-child` (recommended for Mother/Child/Toy path)
- `task`
- `command`
- `pipeline`
- `mother-child` (legacy migration lane)

Enable exactly one world feature per child crate.

## Naming Policy

- `patina-ai` remains the app/runtime product crate.
- `patina-sdk` is the single SDK crate for child+toy authoring.
- Do not introduce parallel SDK crate surfaces.

## Migration (old SDK imports)

If you previously used split SDK crates, migrate as follows:

- `patina-child-sdk` -> `patina-sdk` with `features = ["knowledge-child"]`
- `patina-toy-sdk::*` -> `patina-sdk::toys::*`
- `patina_child_sdk::granted::*` -> `patina_sdk::granted::*`
- `patina_child_sdk::substrate::*` -> `patina_sdk::substrate::*`
- `patina_child_sdk::{register_knowledge_child, ...}` ->
  `patina_sdk::{register_knowledge_child, ...}` with type imports from
  `patina_sdk::knowledge_child::*`

## License

MIT
