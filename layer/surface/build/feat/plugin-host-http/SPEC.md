---
type: feat
id: plugin-host-http
status: ready
created: 2026-02-13
sessions:
  origin: 20260213-120746
blocked_by:
- plugin-system
related:
- layer/surface/build/feat/plugin-ecosystem/SPEC.md
beliefs:
- lib-owns-policy-binary-owns-wiring
- sanitize-at-data-level-not-just-control-flow
- two-layer-capability-grants
- separate-worlds-for-isolation
---

# feat: Host HTTP Interface (`patina:host/http`)

> Domain-allowlisted HTTP for plugins. The plugin never sees `curl`.
> The host handles TLS, domain enforcement, and credential injection.

## Problem

Action plugins (webhook notifiers, PR reviewers, deploy triggers) need
HTTP access. Raw `curl` via the toy system is dangerous — toy allowlists
gate commands but not arguments (`curl https://evil.com` passes if `curl`
is allowed). A host-provided HTTP interface with domain allowlisting is
safer and gives the host control over credentials.

## Parent Design

This is build order item #2 from [[plugin-ecosystem]] SPEC.md (lines
461-506). All design decisions are locked there. This spec owns the
implementation — scope, commits, acceptance criteria.

## Scope

Add `patina:host/http@0.1.0` to the host interfaces. Land it in the
**mother-child world first** (exists, has `PluginEngine` + `WasmChild`).
Task world comes later (build order #3, separate spec).

### WIT Interface

From ecosystem spec, locked:

```wit
interface http {
    record http-response {
        status: u16,
        body: string,
    }

    /// POST to an allowed domain. Host enforces domain allowlist.
    http-post: func(url: string, body: string, content-type: string) -> result<http-response, string>;

    /// GET from an allowed domain.
    http-get: func(url: string) -> result<http-response, string>;
}
```

### Manifest Gating

```toml
[capabilities]
host_http = ["hooks.slack.com", "api.github.com"]
```

`host_http` is a list of allowed domains. Empty list or absent key = no
HTTP access. The host extracts the domain from the URL and checks against
this list before making any request.

### Security Properties (locked)

1. **HTTPS only** — reject non-HTTPS URLs (no plaintext HTTP)
2. **Domain allowlist** — host extracts domain from URL, rejects if not in
   `GrantedCapabilities.http_domains`
3. **No cross-domain redirects** — if a response redirects to a different
   domain, reject it (prevents allowlist bypass)
4. **Auth injection** — host MAY inject `Authorization` headers from
   `~/.patina/secrets/` based on domain. Plugin never handles credentials.
   This is optional for v1 — can be a follow-up.
5. **Status code transparency** — return `http-response { status, body }`
   so plugins can branch on 4xx/5xx

### What NOT to Build

- Auth injection from secrets store — design it but defer implementation
  to follow-up (keeps this spec small, secrets integration is its own scope)
- Request headers from guest — future-compatible (`headers` field on
  `http-response`), not built now
- Connection pooling — `reqwest` handles this. No custom pool management.
- Command world HTTP — commands are read-only by design
- Task world HTTP — task world doesn't exist yet (build order #3)

## Architecture

### Pattern: lib-owns-policy, binary-owns-wiring

Same pattern as query (belief: [[lib-owns-policy-binary-owns-wiring]]):

- **lib crate** (`src/plugin/`): defines `HttpDispatchFn` type, implements
  `patina::host::http::Host` trait with domain validation + call-time
  gating, strips/validates before dispatch
- **binary crate** (`src/main.rs`): provides `make_http_dispatch()` closure
  that captures a `reqwest::blocking::Client`

But — **check if this boundary is needed**. Unlike query, `reqwest` is a
library crate dependency (not binary-only). If `reqwest` is accessible from
lib, the host impl can call it directly without a callback. Read `Cargo.toml`
to verify. If reqwest is in lib's deps, skip the callback — simpler is better.

### GrantedCapabilities Extension

```rust
// Already in spec, add to existing struct:
pub struct GrantedCapabilities {
    pub query_kinds: HashSet<String>,
    pub query_scope: QueryScope,
    pub http_domains: HashSet<String>,  // NEW
}
```

### Defense in Depth (per [[two-layer-capability-grants]])

1. **Load-time**: `check_capabilities()` validates `host_http` domains are
   non-empty strings. Rejects malformed manifests early.
2. **Call-time**: Host impl extracts domain from URL, checks against
   `grants.http_domains`. Deny-by-default.
3. **Data-level** (per [[sanitize-at-data-level-not-just-control-flow]]):
   URL validation — reject non-HTTPS, reject IPs (no `https://192.168.1.1`),
   reject localhost. This is the trust boundary sanitization.

### Mother-Child Integration

Mother-child `HostState` currently has: `plugin_name`, `wasi`, `wasi_table`.
It needs to grow to carry `GrantedCapabilities` and the HTTP dispatch/client,
matching the pattern `CommandHostState` already uses.

Files to touch:
- `wit/deps/patina-host/host.wit` — add `interface http { ... }`
- `wit/mother-child/mother-child.wit` — add `import patina:host/http@0.1.0;`
- `wit/mother-child/deps/patina-host/host.wit` — sync copy
- `src/plugin/internal/mod.rs` — extend `GrantedCapabilities`, parse
  `host_http` from manifest, extend `check_capabilities()`
- `src/plugin/internal/mother_child.rs` — expand `HostState`, implement
  `patina::host::http::Host` trait, wire through `instantiate_child()`
- `src/plugin/internal/tests.rs` — domain validation tests

### Implementation Plan (3 commits)

**Commit 1: WIT + host-side HTTP dispatch**
- Add `interface http` to `wit/deps/patina-host/host.wit`
- Add `import patina:host/http@0.1.0` to mother-child world WIT
- Sync WIT copies across plugin crates
- Extend `GrantedCapabilities` with `http_domains: HashSet<String>`
- Parse `host_http` from manifest TOML in `PluginManifest::from_path()`
- Extend `check_capabilities()` for HTTP domain validation
- Expand mother-child `HostState` with grants + reqwest client
- Implement `patina::host::http::Host` for `HostState`:
  - Extract domain from URL
  - HTTPS-only check
  - Domain allowlist check
  - No cross-domain redirect (reqwest redirect policy)
  - Execute request, return `http-response`
- Wire through `instantiate_child()`

**Commit 2: Guest API**
- Add `pub mod http` to `patina-plugin-api/src/lib.rs` (mother-child guest API)
- Follow existing `pub mod layer` wrapper pattern
- Sync WIT in `patina-plugin-api/` directory

**Commit 3: Conformance test**
- Update existing test child or create `http-test-child`:
  - Manifest with `host_http = ["httpbin.org"]` (or mock)
  - Call `http-get("https://httpbin.org/get")`
  - Verify status 200 returned
- Test: plugin WITHOUT `host_http` is denied at call time
- Test: plugin requesting non-allowed domain is denied
- Test: non-HTTPS URL is rejected

## Exit Criteria

- [ ] `interface http` in host WIT, imported by mother-child world
- [ ] `GrantedCapabilities.http_domains` parsed from manifest
- [ ] `check_capabilities()` validates HTTP domains at load time
- [ ] Host impl enforces: HTTPS-only, domain allowlist, no cross-domain redirect
- [ ] Mother-child `HostState` carries grants (pattern matches `CommandHostState`)
- [ ] Guest API module in mother-child guest crate
- [ ] Conformance test: allowed domain succeeds, denied domain fails
- [ ] `./resources/git/pre-push-checks.sh` passes

## Status Log

| Date | Status | Note |
|------|--------|------|
| 2026-02-13 | ready | Extracted from [[plugin-ecosystem]] build order item #2. Design locked in parent spec. Implementation-ready. |
