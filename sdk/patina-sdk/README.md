# patina-sdk

`patina-sdk` is the SDK for push-pure Patina WASM children.

It is designed to be used alongside `wit_bindgen::generate!` in each child.
Children keep their own WIT world and authoritative trait-signature types. The
SDK provides ergonomic outside toy helpers and convenience type exports.

## Decision Tree

- Building a new push-pure child? Use `patina-sdk`.
- Maintaining a legacy service child (`handle(action, payload)`)? Use
  `patina-sdk-legacy` with feature `child`.
- Maintaining a legacy grammar pipeline child? Use `patina-sdk-legacy` with
  feature `pipeline`.

## What This SDK Provides

- `toys::log::{info,warn,error}`
- `toys::keyvalue::{open, Bucket::{get,set,exists}}`
- `toys::measure::{counter,gauge}`
- `toys::config::get`
- `prelude` and root exports for shared SDK-facing types

## Backend-neutral contract (required)

Children authored with `patina-sdk` must remain orchestration-backend neutral.

- ✅ Author business contracts in WIT (`patina:*` interfaces/worlds).
- ✅ Use toys/WASI capabilities only (`log`, `measure`, `config`, `keyvalue`, etc.).
- ✅ Accept orchestration metadata only as normal contract inputs when needed.
- ❌ Do **not** import Rivet/queue/workflow SDKs in child business code.
- ❌ Do **not** encode orchestrator IDs into WIT package/interface naming.

Mother (or other orchestrators) may adapt external triggers into typed calls, but child logic stays portable.

## Child Pattern

```rust
use patina_sdk::toys;

wit_bindgen::generate!({
    path: "wit",
    world: "my-child",
    generate_all,
});

struct MyChild;

impl exports::patina::records::transform::Guest for MyChild {
    fn transform(
        records: Vec<patina::records::types::RecordEnvelope>,
    ) -> Result<patina::records::types::TransformResult, String> {
        toys::log::info("my-child", "processing batch");
        toys::measure::counter("records_seen", records.len() as f64)?;
        Ok(patina::records::types::TransformResult {
            accepted: records,
            rejected: Vec::new(),
        })
    }
}

export!(MyChild);
```

## Template

Generate a new child scaffold with:

```sh
cargo generate --path sdk/template
```

## License

MIT
