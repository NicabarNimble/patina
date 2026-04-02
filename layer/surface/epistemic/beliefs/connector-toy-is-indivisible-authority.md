---
type: belief
id: connector-toy-is-indivisible-authority
persona: architect
facets: [architecture, security, pipe]
entrenchment: medium
status: active
endorsed: true
extracted: 2026-03-10
revised: 2026-03-26
---

# connector-toy-is-indivisible-authority

The `patina:connect` toy is a single indivisible capability bundle: named connections carry credential, domain allowlist, and injection config as one grant. `connect::request(...)` owns the credential path. HTTP access, credential injection, and domain policy are not separate toys but facets of the connect grant, preserving security invariants when authority moves from Mother to child via WIT host imports.

## Statement

The `patina:connect` toy is a single indivisible capability bundle: named connections carry credential, domain allowlist, and injection config as one grant. `connect::request(...)` owns the credential path. HTTP access, credential injection, and domain policy are not separate toys but facets of the connect grant, preserving security invariants when authority moves from Mother to child via WIT host imports.

## Evidence

- [[session-20260310-142000]]: Three-session convergence: started with proxy as separate toy, refined through audit agent discussion to derived enforcement from connector grant (weight: 0.9)
- [[session-20260310-094749]]: Identified 5 gaps between specs and code; converged on child/toy/Mother model through 4 rounds with audit agent (weight: 0.7)
- [[session-20260310-074810]]: Origin session — drafted DuckLake spec with connector as separate concept from proxy (weight: 0.5)

## Supports

- [[children-have-agency-toys-are-capabilities]] — refines the toy taxonomy: `connect` bundles credential + domain + HTTP as one indivisible grant
- [[initialize-is-capability-grant]] — connect grants are part of the `GrantedCapabilities` resolved at init from `[needs].toys`

## Attacks

- Proxy-as-separate-toy model — defeated by toy-collapse-wasi-alignment; HTTP proxy merged into `patina:connect` as a facet, not a peer toy
- Split-capability model — any design where credential, domain policy, and HTTP access are separable toys; the indivisible bundle prevents "HTTP without policy" or "credential without domain scope"

## Attacked-By

- "What if a child needs raw HTTP without named connections?" — addressed by WASI `wasi:http` adoption: children can import `wasi:http` for raw HTTP, but `patina:connect` is the credential-safe path. Raw HTTP has no credential injection.

## Applied-In

- `wit/toys/deps/patina-connect.wit` — WIT interface defining `connect::request(...)` as the credential-safe named connection surface
- `src/child/internal/mod.rs` — `GrantedCapabilities` resolves connect grants (http_domains, credential_mappings) from `[needs].toys` at init
- [[toy-collapse-wasi-alignment]] — connector + http-proxy collapsed into `patina:connect`; `connect::request(...)` owns the credential path as decided in Phase 0

## Revision Log

- 2026-03-10: Created — three-session convergence from separate-toy to derived-enforcement model
- 2026-03-26: Revised — reframed around `patina:connect` WIT interface post-toy-collapse. Retired ConnectorToy/build_http_proxy vocabulary. Updated Applied-In to current code paths. Attacks/Attacked-By updated for WASI http coexistence.
