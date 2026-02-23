---
type: belief
id: wasm-host-boundary-hides-credentials
persona: architect
facets: [secrets, wasm, architecture, security]
entrenchment: medium
status: active
endorsed: true
extracted: 2026-02-22
revised: 2026-02-22
---

# wasm-host-boundary-hides-credentials

WASM host-boundary injection achieves full credential isolation — credentials never exist in the LLM process, plugin memory, or tool results, only inside the host for the duration of one HTTP call.

## Statement

WASM host-boundary injection achieves full credential isolation — credentials never exist in the LLM process, plugin memory, or tool results, only inside the host for the duration of one HTTP call.

## Evidence

- [[session-20260222-200024]]: Architecture synthesis — Skills (workflows) + MCP (tool routing) + WASM plugins (capability agents) + host-boundary credential injection = zero credentials in LLM process. LLM calls MCP tools, plugins call host_http, host injects from vault, leak-detects response. (weight: 0.95)
- [[session-20260222-165738]]: IronClaw reference repo analysis — their `credential_injector.rs` injects at WASM host boundary, tools never see plaintext. Validated pattern at production scale. Their HTTPS CONNECT limitation (line 246-248) confirms proxy alone isn't enough, but host-boundary injection works. (weight: 0.9)
- [[wit/deps/patina-host/host.wit]]: Lines 100-104 — "The host controls TLS, domain enforcement, and credential injection. Plugins never make raw network calls." Architecture designed for this from the beginning. (weight: 0.85)
- [[src/plugin/internal/host_support.rs]]: Domain allowlisting already validates at load-time AND call-time. Host-controlled `reqwest::blocking::Client` means credential injection is architecturally possible today — just not wired to vault. (weight: 0.8)
- IronClaw `SharedCredentialRegistry` pattern: Thread-safe, append-only credential mapping built at tool registration. Maps (secret_name, host_pattern, injection_location). Supports bearer, basic, header, query_param, url_path. (weight: 0.8)

## Supports

- [[defense-in-depth-over-perfect-isolation]]: Host-boundary injection IS the perfect isolation for WASM plugins — defense in depth is only needed for native processes
- [[storage-encryption-vs-runtime-isolation]]: This belief solves the runtime isolation gap — credentials encrypted at rest AND hidden at runtime through host boundary
- [[bearer-token-forces-plaintext-exposure]]: Bearer tokens still require plaintext, but the plaintext exists ONLY inside the host boundary, never in plugin or LLM memory

## Attacks

- [[bearer-token-forces-plaintext-exposure]]: Partially defeats — the protocol constraint is real, but the exposure is contained to the host boundary (microseconds, single function scope), not the LLM process lifetime

## Attacked-By

- Native processes (Claude Code, cargo) cannot use WASM boundary — they still get env var injection. Scoped injection mitigates but doesn't eliminate. Status: acknowledged limitation, not a defeat.

## Applied-In

- `wit/deps/patina-host/host.wit` — HTTP interface designed for credential injection (not yet wired)
- `src/plugin/internal/host_support.rs` — Domain allowlisting infrastructure (working, needs credential wiring)
- `plugin.toml` capability declarations — Plugin manifests already declare domain needs (extend for secrets)
- IronClaw `tools-src/github/capabilities.json` — Reference implementation of per-tool credential declarations

## Revision Log

- 2026-02-22: Created — metrics computed by `patina scrape`
