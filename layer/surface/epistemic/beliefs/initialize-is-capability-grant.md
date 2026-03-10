---
type: belief
id: initialize-is-capability-grant
persona: architect
facets: [architecture, security, pipe-protocol]
entrenchment: medium
status: active
endorsed: true
extracted: 2026-03-10
revised: 2026-03-10
---

# initialize-is-capability-grant

pipe/initialize is a capability grant, not just startup config. The init payload carries typed capability grants (e.g., `DuckLakeGrant` with `ConnectorToy` + `StorageToy`). It is the security boundary between Mother's authority and the child's autonomy. Grants are typed per child type — no untyped blobs at trust boundaries.

## Statement

pipe/initialize is a capability grant, not just startup config. The init payload carries typed capability grants (e.g., `DuckLakeGrant` with `ConnectorToy` + `StorageToy`). It is the security boundary between Mother's authority and the child's autonomy. Grants are typed per child type — no untyped blobs at trust boundaries.

## Evidence

- [[session-20260310-094749]]: Audit agent: initialize carries binary path, credential, domain allowlist, storage path — effectively the capability token set for the whole child. Should be treated as a serious security boundary. (weight: 0.9)
- [[session-20260310-142000]]: Hardened from "Option C: child reads extra fields from raw params" to typed `DuckLakeGrant` on `InitializeParams`. Per [[no-untyped-blobs-at-trust-boundaries]]: concrete before generic at security boundaries. (weight: 0.95)
- [[session-20260310-094749]]: "Child autonomy is over workflow, not over expanding its own authority." Init grants fixed capabilities; child cannot acquire new ones at runtime. (weight: 0.9)

## Supports

- [[children-have-agency-toys-are-capabilities]] — the init payload is WHERE toy approvals are granted
- [[safety-boundaries]] — project-scoped, user consent; init is the consent boundary for child capabilities

## Attacks

<!-- none yet -->

## Attacked-By

- Risk: init payload becomes a grab-bag of untyped config. Mitigated by: [[no-untyped-blobs-at-trust-boundaries]] — typed grants per child type, fail closed on malformed.
- Risk: credential material in init means init channel must be trusted. Currently stdio (local process). Future WIT host-guest (WASM sandbox). Both are trusted channels.

## Applied-In

- [[ducklake]] spec — `InitializeParams.ducklake: Option<DuckLakeGrant>` carries typed `ConnectorToy` + `StorageToy`
- Current `InitializeParams` in `patina-pipe-types/config.rs` — will gain typed grant fields per child type

## Revision Log

- 2026-03-10: Created in [[session-20260310-094749]] — audit agent identified that init payload is effectively the capability token set and should be treated as a security boundary.
- 2026-03-10: Revised in [[session-20260310-142000]] — hardened from untyped "Option C" to typed `DuckLakeGrant` per [[no-untyped-blobs-at-trust-boundaries]].
