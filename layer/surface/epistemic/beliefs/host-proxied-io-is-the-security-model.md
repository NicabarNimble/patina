---
type: belief
id: host-proxied-io-is-the-security-model
persona: architect
facets: [architecture, security, children, sandbox]
entrenchment: medium
status: active
endorsed: true
extracted: 2026-03-05
revised: 2026-03-06
---

# host-proxied-io-is-the-security-model

Security is host-side for WASM children (host-proxied I/O — all calls go through host functions) and OS-side for native children (macOS sandbox-exec, Linux Landlock). Both enforce the same constraints: domain allowlist, credential injection, leak detection. The three-layer model (protocol enforcement + capability manifest + runtime sandbox) applies to all children regardless of runtime.

## Statement

Security is host-side for WASM children (host-proxied I/O — all calls go through host functions) and OS-side for native children (macOS sandbox-exec, Linux Landlock). Both enforce the same constraints: domain allowlist, credential injection, leak detection. The three-layer model (protocol enforcement + capability manifest + runtime sandbox) applies to all children regardless of runtime.

## Evidence

- [[session-20260305-224446]]: Traced current security through host_support.rs: all enforcement is host-side (validate_http_url, check_secret_grant, resolve_credential, leak_check). WASM just prevents bypass. OS sandbox does the same. Three-layer model: protocol enforcement (always) + capability manifest (always) + runtime sandbox (WASM or OS). (weight: 0.9)
- [[session-20260306-123021]]: Architecture reframe established dual security model: WASM children get host-proxied I/O (existing), native children get OS sandbox (Chrome renderer pattern, ~2ms startup, ~0ns runtime overhead). Same constraints, different enforcement mechanism. (weight: 0.9)

## Supports

- [[safety-boundaries]] — user consent, project-scoped, no surprise side effects — both security models enforce these
- [[pipes-are-processes-not-wasm]] — multi-runtime model requires dual security approach
- [[persona-keypair-is-node-identity]] — UCAN capability tokens extend security with cryptographic scoping (future)

## Attacks

- Attacks "WASM is required for security" — the security was always in the host (host_support.rs), WASM just prevented bypass. OS sandbox does the same for native children.
- Attacks "native children are less secure" — OS sandbox + protocol enforcement + capability manifest provides equivalent isolation

## Attacked-By

- "Native children making direct HTTP is a security hole" — native children have network access within their OS sandbox. Counter: sandbox restricts to declared domains only. Protocol enforcement (Layer 1) validates emitted facts. Credential delivery via stdin (not env/files) prevents exfiltration.

## Applied-In

- `src/plugin/internal/host_support.rs` — host-side security (validate_http_url, check_secret_grant, resolve_credential, leak_check) — reusable patterns for both runtimes
- [[spec-pipe-architecture]] — three-layer security model (protocol + manifest + runtime sandbox) with WASM and OS sandbox variants

## Revision Log

- 2026-03-05: Created — host-proxied I/O as the security model
- 2026-03-06: Revised — dual security model: host-proxied for WASM, OS sandbox for native. Same constraints, different enforcement.
