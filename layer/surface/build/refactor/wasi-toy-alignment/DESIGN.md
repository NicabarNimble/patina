# Design: wasi-toy-alignment

## Why This Design

Patina had overlapping custom toy interfaces where WASI already provides durable standards. Keeping parallel interfaces increases long-term maintenance and pushes protocol debt into every child. This design makes WASI the default foundation and keeps Patina-only interfaces only where there is real product-specific delta.

## Build Target

1. Migrate logging to `wasi:logging`.
2. Migrate state to `wasi:keyvalue`.
3. Migrate store access to `wasi:sql`.
4. Narrow connect to HTTP authority and align types with `wasi:http/types`.
5. Split events into standard publish plus Patina cursor delta (`wasi:messaging` + `patina:events-stream`).
6. Introduce `patina:measure` as explicit delta with manifest policy enforcement.

## Resolved Decisions

- Standards-first: adopt WASI where overlap exists.
- Keep `patina:connect` for named-service HTTP authority only.
- Move DB connection ownership to `wasi:sql`.
- Publish via `wasi:messaging`; keep subscribe/ack cursor semantics in `patina:events-stream`.
- Add `patina:measure` as Patina delta with deterministic rejection of undeclared metrics.

## Commits

1. `refactor: migrate log imports to wasi logging`
2. `refactor: align state toy with wasi keyvalue`
3. `refactor: align store toy with wasi sql`
4. `refactor: align connect headers with wasi http types`
5. `refactor: split events into wasi messaging and stream delta`
6. `feat: add measure toy contract with manifest policy checks`

## Direct Code Targets

- `wit/toys/toybox.wit`
- `wit/knowledge-child/knowledge-child.wit`
- `sdk/patina-sdk/wit/knowledge-child/knowledge-child.wit`
- `wit/toys/deps/*.wit`
- `wit/knowledge-child/deps/*.wit`
- `sdk/patina-sdk/wit/knowledge-child/deps/*.wit`
- `src/child/internal/knowledge_child.rs`
- `src/child/internal/mod.rs`
- `src/child/internal/host_support.rs`
- `sdk/patina-sdk/src/knowledge_child.rs`

## Verification Plan

```bash
cargo fmt --all
cargo check --workspace -q
cargo test -q --workspace
wasm-tools component wit wit/toys
wasm-tools component wit wit/knowledge-child
find sdk/patina-sdk-*/src/compat/ -name '*.rs' -not -name 'mod.rs' | grep -c .
```

## Build Readiness

Implemented and verified. Ready for archive once lifecycle transition succeeds.
