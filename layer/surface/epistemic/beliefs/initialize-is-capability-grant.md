---
type: belief
id: initialize-is-capability-grant
persona: architect
facets: [architecture, security, pipe-protocol]
entrenchment: medium
status: active
endorsed: true
extracted: 2026-03-10
revised: 2026-03-26
---

# initialize-is-capability-grant

Child initialization is a capability grant, not just startup config. Mother resolves `GrantedCapabilities` from the child's `[needs].toys` manifest at load time, assembling the toybox (http_domains, credential_mappings, host_emit, state_enabled, etc.) as the sealed capability payload. This is the security boundary between Mother's authority and the child's autonomy. Grants are typed and fail-closed on mismatch.

## Statement

Child initialization is a capability grant, not just startup config. Mother resolves `GrantedCapabilities` from the child's `[needs].toys` manifest at load time, assembling the toybox (http_domains, credential_mappings, host_emit, state_enabled, etc.) as the sealed capability payload. This is the security boundary between Mother's authority and the child's autonomy. Grants are typed and fail-closed on mismatch.

## Evidence

- [[session-20260310-094749]]: Audit agent: initialize carries binary path, credential, domain allowlist, storage path — effectively the capability token set for the whole child. Should be treated as a serious security boundary. (weight: 0.9)
- [[session-20260310-142000]]: Hardened from "Option C: child reads extra fields from raw params" to typed grants on init. Per [[no-untyped-blobs-at-trust-boundaries]]: concrete before generic at security boundaries. (weight: 0.95)
- [[session-20260310-094749]]: "Child autonomy is over workflow, not over expanding its own authority." Init grants fixed capabilities; child cannot acquire new ones at runtime. (weight: 0.9)

## Supports

- [[children-have-agency-toys-are-capabilities]] — the init payload is WHERE toy approvals are granted
- [[safety-boundaries]] — project-scoped, user consent; init is the consent boundary for child capabilities

## Attacks

<!-- none yet -->

## Attacked-By

- Risk: init payload becomes a grab-bag of untyped config. Mitigated by: [[no-untyped-blobs-at-trust-boundaries]] — typed grants per child type, fail closed on malformed.
- Risk: credential material in init means init channel must be trusted. Currently WIT host-guest (WASM sandbox) — a trusted channel. Credential injection flows through `patina:connect` host imports, not raw child memory.

## Applied-In

- `src/child/internal/mod.rs` — `GrantedCapabilities` struct resolved from `[needs].toys` in `child.toml` at load time; fields include http_domains, credential_mappings, host_emit, state_enabled, schema_facts, lake_names
- `src/child/internal/knowledge_child.rs` — `KnowledgeChildEngine` receives `GrantedCapabilities` as the sealed grant set; Host impl checks grants at call-time
- [[toy-collapse-wasi-alignment]] — retired per-child typed grants (DuckLakeGrant/ConnectorToy/StorageToy) in favor of uniform `GrantedCapabilities` resolved from manifest `[needs].toys`

## Revision Log

- 2026-03-10: Created in [[session-20260310-094749]] — audit agent identified that init payload is effectively the capability token set and should be treated as a security boundary.
- 2026-03-10: Revised in [[session-20260310-142000]] — hardened from untyped "Option C" to typed grants per [[no-untyped-blobs-at-trust-boundaries]].
- 2026-03-26: Revised — updated for post-toy-collapse reality. Retired DuckLakeGrant/ConnectorToy/StorageToy vocabulary (no longer in codebase). Current model: `GrantedCapabilities` resolved uniformly from `[needs].toys` in child.toml. Updated Applied-In to current code paths. Updated Attacked-By for WASM host-import credential channel.
