# mother-service fixtures

Deterministic fixtures for Mother loader tests that prove backward-safe behavior for handle-based service children.

## Provenance

Generated from workspace child + manifest:

```bash
cargo build -p patina-ai-child-belief-verifier --target wasm32-wasip2
cp target/wasm32-wasip2/release/patina_ai_child_belief_verifier.wasm tests/fixtures/mother-service/belief-verifier-child.wasm
cp children/belief-verifier/child.toml tests/fixtures/mother-service/belief-verifier-child.toml
```

These are checked in so tests do not depend on local `target/` artifacts at runtime.
