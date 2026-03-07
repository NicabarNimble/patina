---
type: belief
id: cross-crate-json-contracts-need-shared-types
persona: architect
facets: [rust, architecture, testing]
entrenchment: medium
status: active
endorsed: true
extracted: 2026-03-07
revised: 2026-03-07
---

# cross-crate-json-contracts-need-shared-types

When two crates exchange JSON over a wire protocol, serialization and deserialization must use the same struct definition from a shared types crate — not ad-hoc serde_json::json!() maps that can silently drift when fields are added or removed.

## Statement

When two crates exchange JSON over a wire protocol, serialization and deserialization must use the same struct definition from a shared types crate — not ad-hoc serde_json::json!() maps that can silently drift when fields are added or removed.

## Evidence

- [[session-20260307-165002]]: [[session-20260307-165002]] - Found two bugs where broker and child disagreed on required JSON fields: FetchParams missing types/limit, auth payload missing provider. Both passed unit tests in isolation but failed at integration. Fixed by always including required fields, but root cause is ad-hoc JSON construction. (weight: 0.95)

## Supports

- [[host-proxied-io-is-the-security-model]] — shared types enforce that the broker and child agree on auth delivery format
- [[pipes-are-processes-not-wasm]] — native children communicate via JSON-RPC over stdio; shared types prevent wire drift

## Attacks

<!-- Add beliefs this defeats -->

## Attacked-By

<!-- Add beliefs that challenge this -->

## Applied-In

- [[spec-pipe-contract-safety]] — spec to replace ad-hoc JSON with shared pipe_types structs
- `src/broker/spawn.rs:build_init_params()` — currently builds auth JSON manually, will use `pipe_types::AuthConfig`
- `src/broker/lifecycle.rs:FetchParams::to_json()` — currently manual field mapping, will serialize `pipe_types::FetchParams` directly
- `resources/git/pre-push-checks.sh` — integration test catches wire format drift at push time

## Revision Log

- 2026-03-07: Created — metrics computed by `patina scrape`
