# Design: CAR Dead Code (A8, A10-A12, A14-A15, A22-A24)

## Principle Alignment

- [[unix-philosophy]]: remove tools/modules that no longer do a job.
- [[spec-driven-design]]: each deletion must be explicitly authorized and verified.
- [[children-have-agency-toys-are-capabilities]]: toy host functions are part of the child/toy boundary. "Dead to Rust" != "dead to the toybox."
- [[canonical-module-bypass-compounds]]: when an abstraction is bypassed by 50+ sites, delete the abstraction.

## Gate Dependencies

A8 must complete before A10. A8 deletes `embeddings/database.rs` (one of A10's 2 consumers of `db/sqlite.rs`). After A8, A10 only needs to migrate `scrape/code/database.rs`.

All other gates are independent.

## Gate Details

### A8: Delete embeddings/database.rs

Delete `src/embeddings/database.rs`. Remove re-export from `embeddings/mod.rs`. Zero callers. The oxidize pipeline replaced this with direct corpus queries → USearch index building. Children emit facts via the `emit` toy, not by writing to an embeddings database — this module is from before children existed.

### A10: Delete db/sqlite.rs

Delete `src/db/sqlite.rs` and `src/db/mod.rs`. Migrate `scrape/code/database.rs` to direct `rusqlite::Connection::open()`. The wrapper was an aspirational multi-backend abstraction (`9ab88ed9` "add DatabaseBackend enum for multi-backend support") that never achieved adoption. SQLite is the database. The abstraction adds indirection with no benefit.

### A11: Delete dead dev command paths

Delete code in `dev/bump_version.rs`, `dev/sync_adapters.rs`, `dev/validate.rs` that references `src/adapters/claude.rs` (doesn't exist — adapters became interface runtimes) and `.patina/version_manifest.json` (doesn't exist — version management moved to `version.rs` constants). These are remnants of a pre-interface-system adapter architecture.

### A12: Delete SDK tier sub-crates

Delete `sdk/patina-sdk-core/`, `sdk/patina-sdk-data/`, `sdk/patina-sdk-agent/` from disk. [[child-construction-canon]] chose manifest-declared toys with Mother runtime enforcement as the capability model. The tier crates attempted compile-time toy scoping from the SDK side — a different approach that was never shipped. The umbrella `patina-sdk` with feature flags and inline `toys.rs` is the canonical SDK surface.

Update AGENTS.md line 10: remove tier references, state that `sdk/patina-sdk` is the SDK surface.

### A14: Delete LAST_QUERY_ID

Remove static from `scry/logging.rs` and all write sites. Was intended for a "use last query" shortcut in `scry open`/`scry copy` that was never implemented. Write-only global.

### A15: Delete dead persona check

Remove `result.sources.contains(&"persona")` at `scry/semantic.rs:61`. The persona oracle was removed from the retrieval engine pipeline and bolted on separately in the scry CLI layer. The check in the semantic path is unreachable — semantic oracle always emits `"semantic"` as source name.

### A22: Narrow blanket dead_code allows

Replace `#![allow(dead_code)]` at `toy_host/v2.rs:1` and `host_support.rs:11` with per-item annotations.

**Important distinction:** These files implement the toy host interface — the boundary between Mother and children. Three categories:
1. **Called by current children** — live code, no annotation needed.
2. **Part of the toybox but no current child uses it** — alive in the architecture, `#[allow(dead_code)]` is correct. The function exists because the toy exists in the WIT contract. A future child may use it.
3. **Not part of any toy, not called by anything** — genuinely dead. Delete.

Audit each function against the WIT interface definitions in `wit/knowledge-child/` to determine which category it falls in. `host_support.rs:579` (`emit_fact`) is explicitly marked `FROZEN LEGACY PATH` — this is category 3 (dead, documented as non-extensible).

### A23: Delete graph tag stub

Delete `"tag" => {}` from `graph_host.rs:145`. This is a match arm that silently succeeds and does nothing. If a child sends a "tag" action, it should get an error ("unknown action"), not silent success. Silent success on an unimplemented action violates [[observation-at-the-boundary]] — the child can't tell that nothing happened.

### A24: Delete NoneWriter

Delete `NoneWriter` from `git/writer.rs:181`. Zero callers. Keep the `ForgeWriter` trait — the trait abstraction serves a real purpose (additional forge backends like Gitea are a plausible future direction per the connect provider pattern). But `NoneWriter` (a null implementation that rejects everything) has no callers and no test use.

## Strategy

- A8 → A10 (dependency order), then remaining gates in any order.
- Use compile failures as the map for hidden callers.
- Each deletion in a narrow commit.
- A22: audit against WIT before deleting toy host functions.

## Verification

- `cargo check --workspace -q` and `cargo test -q --lib` after each deletion cluster.
- No blanket `allow(dead_code)` added as escape hatch.

## Out of Scope

- Behavior fixes (A1-A6) and inversion fixes (A7).
