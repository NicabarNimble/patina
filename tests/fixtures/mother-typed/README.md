# mother-typed fixtures

Deterministic wasm component fixtures for `src/commands/mother/daemon.rs` typed-wiring audit tests.

## Provenance

Generated from workspace crates (debug profile):

```bash
cargo build -p patina-ai-child-schema-enforcer --target wasm32-wasip2
cargo build -p patina-ai-adapter-se-pando --target wasm32-wasip2
cp target/wasm32-wasip2/debug/patina_ai_child_schema_enforcer.wasm tests/fixtures/mother-typed/schema-enforcer-child.wasm
cp target/wasm32-wasip2/debug/patina_ai_adapter_se_pando.wasm tests/fixtures/mother-typed/se-pando-adapter.wasm
```

These are checked in so tests do not depend on local `target/` build artifacts at runtime.
