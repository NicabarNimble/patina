---
type: refactor
id: pipe-contract-safety
status: ready
created: 2026-03-07
sessions:
  origin: 20260307-165002
related:
- mother-broker
- pipe-architecture
beliefs:
- cross-crate-json-contracts-need-shared-types
exit_criteria:
- id: auth-init-shared-type
  text: Auth payload in pipe/initialize uses a shared struct (e.g., pipe_types::AuthInit) serialized by both broker and child — no ad-hoc serde_json::json!() maps for auth.
  checked: false
  verify: build_init_params() takes AuthInit struct. Child deserializes InitializeParams.auth as Option<AuthInit>. Changing a field in AuthInit causes compile errors on both sides.
- id: fetch-params-single-source
  text: Broker FetchParams::to_json() is replaced by serializing pipe_types::FetchParams directly — one struct definition, zero manual field mapping.
  checked: false
  verify: broker::lifecycle::FetchParams removed or wraps pipe_types::FetchParams. Adding a required field to pipe FetchParams causes compile error in broker.
- id: wire-format-test
  text: A compile-time or unit test in patina-pipe-types validates that broker-produced JSON round-trips through child-side deserialization for all protocol messages.
  checked: false
  verify: cargo test -p patina-pipe-types includes round-trip serde tests for InitializeParams (with auth), FetchParams, and FetchResult.
---
# refactor: Pipe Contract Safety — Shared Types for Cross-Crate Wire Formats

> Move ad-hoc JSON shapes (auth payload, fetch params) into shared structs
> in patina-pipe-types so broker and child agree at compile time, not at
> runtime.

## Context

Session 20260307-165002 found two bugs where the broker and child
disagreed on JSON wire format: FetchParams missing required `types`/`limit`
fields, and auth payload missing required `provider` field. Both passed
unit tests in their own crate but failed at integration time.

Root cause: the broker builds JSON via ad-hoc `serde_json::json!()` maps
instead of serializing the same struct the child deserializes. When the
child-side struct adds a required field, the broker silently produces
invalid JSON.

## Current State

- `broker::lifecycle::FetchParams` is a separate struct from
  `pipe_types::FetchParams` — manual `to_json()` mapping
- `broker::spawn::build_init_params()` builds auth via
  `serde_json::json!({ "token": ..., "provider": ... })` instead of
  using `pipe_types::AuthConfig`
- Wire format agreement is verified only by the pre-push integration
  test (`patina mother run test`), not at compile time

## Target State

- Broker serializes `pipe_types::FetchParams` directly (or a wrapper
  that `Deref`s to it)
- `build_init_params()` constructs `pipe_types::InitializeParams`
  with `auth: Option<pipe_types::AuthConfig>` and serializes it
- Adding/removing a field in pipe-types causes compile errors in the
  broker — impossible to drift silently

## Steps

1. Replace `broker::lifecycle::FetchParams::to_json()` with direct
   serialization of `pipe_types::FetchParams`
2. Replace ad-hoc auth JSON in `build_init_params()` with
   `pipe_types::AuthConfig` construction
3. Add round-trip serde tests in `patina-pipe-types`
4. Remove duplicate type definitions from broker

## Non-Goals

- Changing the wire protocol itself (JSON-RPC structure stays the same)
- Refactoring the harness/ChildConnection (that's a separate concern)
