# Phase 10 Size Audit

Date: 2026-03-21

Build command:

```sh
cargo build -q -p patina-ai-child-session-writer -p patina-ai-child-ducklake --target wasm32-wasip2 --release
```

Artifacts (from `target/wasm32-wasip2/release`):

- `patina_ai_child_session_writer.wasm`: `236607` bytes (~231 KB)
- `patina_ai_child_ducklake.wasm`: `282556` bytes (~276 KB)

Phase target check:

- Session-writer target `<150 KB`: **not met** (current ~231 KB)
- DuckLake target `<2 MB`: **met** (current ~276 KB)

Template child target (`<50 KB`) could not be measured in this environment because `cargo-generate` is not installed.
