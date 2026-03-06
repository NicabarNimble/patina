---
type: belief
id: host-proxied-io-is-the-security-model
persona: architect
facets: [architecture, security, pipes, sandbox]
entrenchment: medium
status: active
endorsed: true
extracted: 2026-03-05
revised: 2026-03-05
---

# host-proxied-io-is-the-security-model

The security model for pipes is host-proxied I/O — pipe asks Mother to make HTTP calls, Mother validates domain allowlist, injects credentials, scans for leaks — not WASM sandboxing. OS sandboxing (macOS sandbox-exec, Linux Landlock) prevents community pipes from bypassing the host, same pattern as Chrome renderer processes

## Statement

The security model for pipes is host-proxied I/O — pipe asks Mother to make HTTP calls, Mother validates domain allowlist, injects credentials, scans for leaks — not WASM sandboxing. OS sandboxing (macOS sandbox-exec, Linux Landlock) prevents community pipes from bypassing the host, same pattern as Chrome renderer processes

## Evidence

- [[session-20260305-224446]]: [[session-20260305-224446]] - Traced current security through host_support.rs: all enforcement is host-side (validate_http_url, check_secret_grant, resolve_credential, leak_check). WASM just prevents bypass. OS sandbox does the same. Three-layer model: protocol enforcement (always) + capability manifest (always) + OS sandbox (community) (weight: 0.9)

## Supports

- [[safety-boundaries]] — user consent, project-scoped, no surprise side effects — host proxy enforces all of these
- [[pipes-are-processes-not-wasm]] — the security model that makes process-based pipes safe without WASM
- [[persona-keypair-is-node-identity]] — UCAN capability tokens extend host-proxied security with cryptographic scoping

## Attacks

- Attacks "WASM is required for security" — the security was always in the host (host_support.rs), WASM just prevented bypass. OS sandbox does the same

## Attacked-By

- "First-party pipes making direct HTTP is a security hole" — if a first-party pipe is compromised, it has network access. Counter: first-party code is trusted (you wrote it), and protocol enforcement (Layer 1) limits what credentials are available. OS sandbox is the belt-and-suspenders for community code

## Applied-In

- `src/plugin/internal/host_support.rs` — existing host-side security (validate_http_url, check_secret_grant, resolve_credential, leak_check) — this code is the security model, reusable for pipes
- [[spec-pipe-architecture]] — three-layer security model (protocol + manifest + OS sandbox)

## Revision Log

- 2026-03-05: Created — metrics computed by `patina scrape`
