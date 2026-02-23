---
type: belief
id: granted-capabilities-is-capability-bus
persona: architect
facets: [wasm, architecture, plugins]
entrenchment: medium
status: active
endorsed: true
extracted: 2026-02-23
revised: 2026-02-23
---

# granted-capabilities-is-capability-bus

GrantedCapabilities is the capability bus — new capabilities extend the struct with zero plumbing changes at call sites, because http_get/http_post/query all receive grants by reference

## Statement

GrantedCapabilities is the capability bus — new capabilities extend the struct with zero plumbing changes at call sites, because http_get/http_post/query all receive grants by reference

## Evidence

- [[session-20260223-061011]]: [[session-20260223-061011]] - Adding credential_mappings to GrantedCapabilities required zero changes to mother_child.rs, task.rs, or command.rs call sites — all HTTP and query functions already receive &grants (weight: 0.9)

## Supports

- [[dependable-rust]] — Small stable interface (`&GrantedCapabilities`) hides internal complexity changes
- [[two-layer-capability-grants]] — Grants struct is the enforcement point for both layers

## Attacks

<!-- Add beliefs this defeats -->

## Attacked-By

- Struct bloat: as capabilities grow, GrantedCapabilities could become a god-object — mitigated by each field being a self-contained collection (HashSet, HashMap)

## Applied-In

- [[spec-wasm-credential-injection]] — `credential_mappings: HashMap<String, CredentialMapping>` added to GrantedCapabilities; `http_get`/`http_post` in `src/plugin/internal/host_support.rs` access it via existing `grants` param
- `src/plugin/internal/host_support.rs` — query(), http_get(), http_post() all take `&GrantedCapabilities`
- Prior art: `http_domains` and `query_kinds` were added the same way without touching callers

## Revision Log

- 2026-02-23: Created — metrics computed by `patina scrape`
