---
type: belief
id: connector-toy-is-indivisible-authority
persona: architect
facets: [architecture, security, pipe]
entrenchment: medium
status: active
endorsed: true
extracted: 2026-03-10
revised: 2026-03-10
---

# connector-toy-is-indivisible-authority

The connector toy is a single indivisible capability bundle (binary, credential, domain allowlist, params, types); HTTP proxy is not a separate toy but a derived enforcement mechanism the child builds from the connector grant, preserving broker security invariants when authority moves from Mother to child.

## Statement

The connector toy is a single indivisible capability bundle (binary, credential, domain allowlist, params, types); HTTP proxy is not a separate toy but a derived enforcement mechanism the child builds from the connector grant, preserving broker security invariants when authority moves from Mother to child.

## Evidence

- [[session-20260310-142000]]: Three-session convergence: started with proxy as separate toy, refined through audit agent discussion to derived enforcement from connector grant (weight: 0.9)
- [[session-20260310-094749]]: Identified 5 gaps between specs and code; converged on child/toy/Mother model through 4 rounds with audit agent (weight: 0.7)
- [[session-20260310-074810]]: Origin session — drafted DuckLake spec with connector as separate concept from proxy (weight: 0.5)

## Supports

- [[children-have-agency-toys-are-capabilities]] — refines the toy taxonomy: connector and storage are the two toy categories, proxy is derived
- [[initialize-is-capability-grant]] — the connector toy IS the primary capability payload in pipe/initialize

## Attacks

- Proxy-as-separate-toy model — treating HTTP proxy as a peer-level toy alongside connector creates a split where a child could conceptually have "connector without policy" or "policy without connector"

## Attacked-By

- "What if a child needs HTTP access without a connector?" — currently no use case; if one arises, that would be a different toy type, not unbundling the connector

## Applied-In

- [[ducklake]] DESIGN.md §1: DuckLake child receives ConnectorToy struct, derives HTTP proxy from it via build_http_proxy
- [[http-proxy-extraction]] SPEC.md: build_http_proxy in patina-pipe is trusted substrate that preserves security invariant when authority moves to child
- [[ducklake]] DESIGN.md §2: grant_lake_capabilities sends connector toy as single JSON object in pipe/initialize toys field

## Revision Log

- 2026-03-10: Created — three-session convergence from separate-toy to derived-enforcement model
