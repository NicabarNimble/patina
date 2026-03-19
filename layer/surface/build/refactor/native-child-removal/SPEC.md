---
type: refactor
id: native-child-removal
status: active
created: 2026-03-19
sessions:
  origin: 20260319-081650-356341000
related:
- single-patina-sdk-consolidation
- child-plugin-sdk-alignment
- ducklake-native-removal-and-verification
exit_criteria: []
---
# refactor: Remove Dead Native Child Infrastructure

> patina-pipe native child spawn path is dead code — github-connector is never invoked, Mother handles connector sync directly via WASM knowledge child. Remove the parallel system so patina-sdk is the single child surface.

## Problem

Two parallel child systems exist: `patina-sdk` (WASM, `plugin.toml`, 5 feature-gated worlds) and `patina-pipe` native child path (`child.toml`, `ChildType`, `Child` trait, JSON-RPC stdio). The native spawn path (`spawn_native_with_plan`) has zero callsites — Mother handles connector sync directly via WASM knowledge child (`knowledge_child.rs:528-578`). `children/github-connector` is the sole outlier not using `patina-sdk`.

This contradicts the locked doctrine ([[belief:children-have-agency-toys-are-capabilities]]): github-connector has no agency and is doctrinally a toy, but uses `Child` vocabulary from a parallel SDK surface. The dual system creates confusion about what the SDK is and prevents it from being the single 3rd-party extension surface.

## Goal

Make `patina-sdk` the only public child/extension surface. Remove dead native child infrastructure. DuckLake production path must be completely unaffected.

## Status

Draft — validated by two independent audit agents and manual code tracing in [[session-20260319-071818-503477000]].

## Non-Goals

- Removing `patina-pipe` crate entirely — its `http_proxy` module (`build_http_client`, `validate_http_url`) and `measure` module (`VALID_VERBS`) are used by live production paths in `host_support.rs` and `src/measure.rs`
- Removing `patina-pipe-types` entirely — SDK re-exports it (`pub use patina_pipe_types as pipe_types`)
- Changing how DuckLake connector sync works (Mother's direct HTTP in `knowledge_child.rs` stays)
- Designing a new connector SDK surface (out of scope, separate spec)

## Current State

**Dead code (native child spawn path):**
- `src/mother/broker/spawn.rs` — `spawn_native_with_plan()` has zero callsites outside its own file
- `src/mother/broker/lifecycle.rs` — `NativeChild`, `BrokerChild` trait, only used by spawn.rs
- `children/github-connector/` — native binary never invoked by any production path
- `crates/patina-pipe-types/src/manifest.rs` — `ChildManifest`, `ChildType` enum, only consumed by dead spawn path + schema command

**Coupling points (must fix before deletion):**
- `src/connect/internal/resolve.rs:23` — calls `resolve_child_binary(&record.auth.child)`, checking if a native binary exists. This is the `auth.child` field in the connection model assuming a binary name
- `src/mother/broker/routing.rs` — partially stranded; `WriteResult` type is still used by `broker::run_source` return
- `src/commands/schema/internal.rs:317` — schema consistency check parsing `child.toml`

**Live code in patina-pipe (stays):**
- `patina_pipe::http_proxy::build_http_client()` — used by `host_support.rs:201`
- `patina_pipe::http_proxy::validate_http_url()` — used by `host_support.rs:208`
- `patina_pipe::measure::VALID_VERBS` — used by `src/measure.rs`, `host_support.rs`

## Target State

- `patina-sdk` is the only public extension surface for children
- No `child.toml` manifests exist in the workspace
- No `ChildType` enum, no native `Child` trait in public-facing paths
- `patina-pipe` retained as internal utility crate (HTTP proxy, measure vocab) — not a child SDK
- Connection model uses connector binding semantics, not binary-existence checks
- CI guard prevents re-introduction of native child paths

## Solution

Six-step ordered refactoring. Each step is independently committable and testable.

## Implementation Order

### Step 1: Refactor connection model to remove binary-existence dependency

**Files:** `src/connect/internal/resolve.rs`, `src/connect/internal/model.rs`

Remove the `resolve_child_binary(&record.auth.child)` check at `resolve.rs:23`. The `auth.child` field is persisted in stored connection records (`model.rs:93`), so this requires a migration strategy:

**Migration:** Read `auth.child` on load, ignore it for validation (no binary check), preserve it on save for backwards compat. Add a deprecation warning if the field is present. Future spec can remove the field entirely once all stored connections have been rewritten.

**Replacement validation:** The binary-existence check was an early guard against bad configs. Replace with provider-level validation: verify the connection's `provider` field maps to a known connector capability (e.g., "github" → knowledge child can handle GitHub sources). The broker must still fail closed on invalid configs — removing a check without replacement violates fail-closed principle.

This unblocks deletion of `spawn.rs`.

### Step 2: Decouple WriteResult from routing.rs

**Files:** `src/mother/broker/routing.rs`, `src/mother/broker/mod.rs`

Move `WriteResult` to `mod.rs` or a shared types location. This allows `routing.rs` to be removed without breaking `run_source()` return type.

**Caution on routing.rs:** This file contains fact validation and write logic that may have hardening value. Before deletion, prove zero runtime callsites with `cargo test` + grep. If any validation logic is still reachable, extract it rather than delete. Only remove after confirmed fully stranded.

### Step 3: Delete dead native spawn path

**Files to remove:**
- `src/mother/broker/spawn.rs`
- `src/mother/broker/lifecycle.rs`

**Files to update:**
- `src/mother/broker/mod.rs` — remove `pub mod spawn; pub mod lifecycle;` declarations and `pub use self::spawn::resolve_child_binary`
- `src/mother/broker/routing.rs` — remove if fully dead after Step 2, or strip to only live code

### Step 4: Remove native connector package

**Files to remove:**
- `children/github-connector/` (entire directory)

**Files to update:**
- `Cargo.toml` — remove `children/github-connector` from workspace members
- `src/commands/schema/internal.rs:317` — replace `child.toml` parsing block with `plugin.toml` consistency check. Don't just delete the drift guard — the schema command should still validate that plugin manifests and schema declarations are consistent. Replace the `ChildManifest`-based check with a `PluginManifest`-based equivalent.

### Step 5: Trim patina-pipe-types manifest surface

**Files:** `crates/patina-pipe-types/src/manifest.rs`, `crates/patina-pipe-types/src/lib.rs`

Remove `manifest.rs` and its `pub mod manifest` export once no callers remain. Verify SDK re-export (`pub use patina_pipe_types as pipe_types`) is unaffected — it should be, since SDK consumers use protocol/measure types, not manifest types.

### Step 6: Add CI anti-regression guard

**Files:** `resources/scripts/check-single-sdk-surface.sh`

Extend existing CI guard to fail on:
- Any `children/**/child.toml` files
- Direct `patina_pipe::Child` trait usage in `children/`
- `ChildType` references in code directories (`src/`, `children/`, `sdk/`, `crates/`) — exclude `layer/`, docs, and test fixtures to avoid false positives from session artifacts and spec history
- `resolve_child_binary` callsites

## Resolved Decisions

| Decision | Rationale |
|----------|-----------|
| Keep `patina-pipe` crate | HTTP proxy and measure vocab are live; crate becomes internal utility, not child SDK |
| Keep `patina-pipe-types` crate | SDK re-exports it; protocol types shared across worlds |
| Remove `ChildType` enum | Decorative — only selected sandbox profile, no method gating or lifecycle differentiation |
| Remove `github-connector` binary | Never invoked; Mother handles GitHub HTTP directly in knowledge child host |
| Refactor `auth.child` before deletion | `resolve.rs:23` is the sole coupling point blocking spawn.rs removal |
| Backwards-compat migration for `auth.child` | Field is persisted in stored connections; read-ignore-preserve, don't break existing configs |
| Replace validation, don't just delete | Binary-existence check was a fail-closed guard; replace with provider-capability validation |
| Replace schema drift check, don't just delete | `child.toml` check becomes `plugin.toml` check; drift guard stays, target changes |

## Verification

**Compile & unit tests (after each step):**
- `cargo check --workspace`
- `cargo test --workspace`

**Behavioral parity (after Steps 1-4, and after completion):**
- `patina mother run <source>` — single source sync completes successfully
- `patina scrape` — full scrape with DuckLake ingestion produces expected fact counts
- End-to-end DuckLake sync assertion: run against a known repo, verify rows written matches pre-refactor baseline

**Structural checks:**
- No `child.toml` files remain in workspace
- CI guard script passes (`resources/scripts/check-single-sdk-surface.sh`)
- SDK re-export (`pub use patina_pipe_types as pipe_types`) still compiles
- Connection model loads existing stored connections without error (migration compat)

## Exit Criteria

- [ ] `spawn_native_with_plan` deleted, no callsites
- [ ] `NativeChild` / `BrokerChild` trait deleted
- [ ] `children/github-connector/` removed from workspace
- [ ] `ChildType` enum removed or quarantined to test-only
- [ ] `resolve_child_binary` removed from connection model
- [ ] `child.toml` manifest parsing removed from schema command
- [ ] `manifest.rs` removed from `patina-pipe-types` public surface
- [ ] CI guard blocks re-introduction of native child paths
- [ ] `patina scrape` passes (DuckLake unaffected)
- [ ] `cargo test --workspace` green

## Build Readiness

Ready after review. Each step is independently committable. No blockers from other specs. Estimated scope: ~200 lines deleted, ~50 lines modified.
