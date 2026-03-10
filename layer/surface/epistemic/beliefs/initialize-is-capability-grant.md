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

pipe/initialize is a capability grant, not just startup config. The init payload carries the full set of approved toys (connector binary, credentials, storage path, domain allowlist). It is the security boundary between Mother's authority and the child's autonomy.

## Statement

pipe/initialize is a capability grant, not just startup config. The init payload carries the full set of approved toys (connector binary, credentials, storage path, domain allowlist). It is the security boundary between Mother's authority and the child's autonomy.

## Evidence

- [[session-20260310-094749]]: Audit agent: initialize carries binary path, credential, domain allowlist, storage path — effectively the capability token set for the whole child. Should be treated as a serious security boundary. (weight: 0.9)
- [[session-20260310-094749]]: Current InitializeParams is minimal (protocol_version + auth). DuckLake needs lake_path, connector binary, params, types, allowed_domains. Option C chosen: init carries everything, child reads what it needs. (weight: 0.85)
- [[session-20260310-094749]]: "Child autonomy is over workflow, not over expanding its own authority." Init grants fixed capabilities; child cannot acquire new ones at runtime. (weight: 0.9)

## Supports

- [[children-have-agency-toys-are-capabilities]] — the init payload is WHERE toy approvals are granted
- [[safety-boundaries]] — project-scoped, user consent; init is the consent boundary for child capabilities

## Attacks

<!-- none yet -->

## Attacked-By

- Risk: init payload becomes a grab-bag of untyped config. Mitigated by: keep toy approvals explicit and coarse, typed per child type.
- Risk: credential material in init means init channel must be trusted. Currently stdio (local process). Future WIT host-guest (WASM sandbox). Both are trusted channels.

## Applied-In

- [[ducklake]] spec — init carries connector binary, credential, lake path, domain allowlist, source params
- Current `InitializeParams` in `patina-pipe-types/config.rs` — minimal version (protocol_version + auth) that will grow

## Revision Log

- 2026-03-10: Created in [[session-20260310-094749]] — audit agent identified that init payload is effectively the capability token set and should be treated as a security boundary.
