---
type: feat
id: plugin-host-http
status: complete
created: 2026-02-13
blocked_by:
- plugin-system
sessions:
  origin: 20260213-120746
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

## Spec Divergences from Parent

1. **No HttpDispatchFn callback.** The ecosystem spec (lines 453-461)
   shows the callback pattern used for query. RESOLVED: `reqwest` is a
   **lib crate dependency** (Cargo.toml line 47), used by 4 lib files
   (`src/secrets/session.rs`, `src/mother/internal.rs`,
   `src/commands/upgrade.rs`, `src/models/download.rs`). The lib can call
   `reqwest::blocking::Client` directly. Per [[lib-owns-policy-binary-owns-wiring]]:
   callbacks are for when engines live in the binary crate. `reqwest` doesn't.
   **Simpler is better.**

2. **URL parsing uses `reqwest::Url`** (re-exports `url` v2.5.8, already
   in the dependency tree via reqwest + wasmtime-wasi). No new dependency.

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

### What NOT to Touch

- `src/plugin/internal/command.rs` — command world is read-only, no HTTP
- `src/main.rs` — no binary-side dispatch needed (reqwest is lib-accessible)
- `wit/command/` — command world does not import HTTP
- `src/mcp/` — MCP server is unrelated
- Auth injection from secrets store — defer to follow-up
- Request headers from guest — future-compatible, not built now
- Connection pooling — `reqwest` handles this. No custom pool management.
- Task world HTTP — task world doesn't exist yet (build order #3)

## Architecture

### Direct reqwest (no callback)

Unlike query (which needed `QueryDispatchFn` because `QueryEngine` lives
in the binary crate), HTTP uses `reqwest::blocking::Client` directly from
the lib crate. The host impl creates a client with a custom redirect
policy and calls it inline.

```rust
// In mother_child.rs HostState — no callback needed
pub http_client: reqwest::blocking::Client,
pub grants: GrantedCapabilities,
```

The client is built once at instantiation with redirect policy set to
reject cross-domain redirects:

```rust
reqwest::blocking::Client::builder()
    .redirect(reqwest::redirect::Policy::custom(|attempt| {
        // Only follow redirects within the same domain
        if attempt.url().host_str() != attempt.previous().last()
            .and_then(|u| u.host_str()) {
            attempt.stop()
        } else {
            attempt.follow()
        }
    }))
    .build()
```

### GrantedCapabilities Extension

```rust
// src/plugin/internal/mod.rs — extend existing struct:
pub struct GrantedCapabilities {
    pub query_kinds: HashSet<String>,
    pub query_scope: QueryScope,
    pub http_domains: HashSet<String>,  // NEW
}
```

### PluginManifest Extension

```rust
// src/plugin/internal/mod.rs — add field to PluginManifest:
pub host_http_domains: Vec<String>,  // NEW — from [capabilities].host_http
```

Parse pattern: identical to `host_query_kinds` parsing (lines 152-161
of current `mod.rs`). `host_http` is an array of domain strings.

### Defense in Depth (per [[two-layer-capability-grants]])

1. **Load-time**: `check_capabilities()` validates `host_http` domains are
   non-empty strings, ASCII-only, no path components. Rejects malformed
   manifests early.
2. **Call-time**: Host impl extracts domain from URL via `reqwest::Url::parse()`,
   checks against `grants.http_domains`. Deny-by-default.
3. **Data-level** (per [[sanitize-at-data-level-not-just-control-flow]]):
   URL validation function (`validate_http_url`) — reject non-HTTPS, reject
   IPs (no `https://192.168.1.1`), reject localhost/127.0.0.1/[::1].
   This is the trust boundary sanitization. Testable independently of wasmtime.

### Mother-Child HostState Expansion

Current `HostState` (mother_child.rs:24-28):
```rust
pub struct HostState {
    pub plugin_name: String,
    pub wasi: wasmtime_wasi::WasiCtx,
    pub wasi_table: wasmtime::component::ResourceTable,
}
```

Expanded to match `CommandHostState` pattern (command.rs:58-69):
```rust
pub struct HostState {
    pub plugin_name: String,
    pub wasi: wasmtime_wasi::WasiCtx,
    pub wasi_table: wasmtime::component::ResourceTable,
    pub grants: GrantedCapabilities,        // NEW
    pub http_client: reqwest::blocking::Client,  // NEW
}
```

Note: mother-child does NOT need `project_root` (doesn't import `layer`)
or `query_fn` (doesn't import `query` yet — that's a separate future item).

### instantiate_child() Changes

`PluginEngine::instantiate_child()` (mother_child.rs:154-186) needs:
1. Call `manifest.granted_capabilities()` to build grants
2. Build `reqwest::blocking::Client` with redirect policy
3. Pass both into `HostState`
4. `check_capabilities()` already called at line 160 — add HTTP validation

## Exact Files to Change

| File | What changes | Lines affected |
|------|-------------|----------------|
| `wit/deps/patina-host/host.wit` | Add `interface http { ... }` after `interface query` (line 98) | +15 lines at end |
| `wit/mother-child/deps/patina-host/host.wit` | Add `interface http { ... }` (sync copy — this file currently lacks `query` too, only add `http`) | +15 lines at end |
| `wit/mother-child/mother-child.wit` | Add `import patina:host/http@0.1.0;` after `types` import (line 10) | +1 line |
| `src/plugin/internal/mod.rs` | (a) Add `http_domains` to `GrantedCapabilities` (line 77), (b) add `host_http_domains` to `PluginManifest` (line 52), (c) parse `host_http` in `from_path()` (after line 161), (d) extend `granted_capabilities()` (line 207), (e) extend `check_capabilities()` (after line 147) | ~25 new lines |
| `src/plugin/internal/mother_child.rs` | (a) Add `grants` + `http_client` to `HostState` (line 24), (b) implement `patina::host::http::Host` for `HostState` (new ~50-line impl block), (c) expand `instantiate_child()` to build client + grants (line 164) | ~70 new lines |
| `src/plugin/internal/tests.rs` | Add `validate_http_url` unit tests + manifest parsing tests for `host_http` | ~60 new lines |
| `patina-plugin-api/src/lib.rs` | Add `pub mod http` wrapper | ~15 new lines |
| `patina-plugin-api/wit/deps/patina-host/host.wit` | Sync copy with http interface | +15 lines |

**Not changing:** `command.rs`, `main.rs`, `wit/command/`, `src/mcp/`

## Implementation Plan (3 commits)

**Commit 1: WIT + manifest + capabilities**
- Add `interface http` to `wit/deps/patina-host/host.wit`
- Add `interface http` to `wit/mother-child/deps/patina-host/host.wit`
- Add `import patina:host/http@0.1.0` to `wit/mother-child/mother-child.wit`
- Add `host_http_domains: Vec<String>` to `PluginManifest`
- Parse `host_http` in `PluginManifest::from_path()` (same pattern as `host_query_kinds`)
- Add `http_domains: HashSet<String>` to `GrantedCapabilities`
- Extend `granted_capabilities()` to populate `http_domains`
- Extend `check_capabilities()`: validate HTTP domains non-empty, ASCII-only
- Add `validate_http_url()` function (pub(super) for testability):
  HTTPS-only, no IPs, no localhost, domain extraction
- Tests: `validate_http_url` unit tests, manifest parsing for `host_http`,
  `check_capabilities` with HTTP domains

**Commit 2: Host impl + HostState expansion**
- Expand mother-child `HostState` with `grants` + `http_client`
- Build `reqwest::blocking::Client` with cross-domain redirect policy
- Implement `patina::host::http::Host` for `HostState`:
  - `http_get()`: validate URL → check domain → execute → return response
  - `http_post()`: validate URL → check domain → set content-type → execute → return response
- Update `instantiate_child()` to build grants + client
- Sync WIT in `patina-plugin-api/` directory

**Commit 3: Guest API + conformance test**
- Add `pub mod http` to `patina-plugin-api/src/lib.rs`
- Follow existing `pub mod layer` wrapper pattern
- Integration test in `src/plugin/internal/tests.rs`:
  - Test: domain NOT in allowlist → denied at call time
  - Test: non-HTTPS URL → rejected by `validate_http_url`
  - Test: IP address URL → rejected
  - Test: localhost → rejected
- Live HTTP test (optional, gated by fixture): call httpbin.org or similar

## Exit Criteria

- [ ] `interface http` in `wit/deps/patina-host/host.wit`, imported by mother-child world
- [ ] `GrantedCapabilities.http_domains` populated from `PluginManifest.host_http_domains`
- [ ] `check_capabilities()` validates HTTP domains at load time
- [ ] `validate_http_url()` enforces: HTTPS-only, no IPs, no localhost, domain extraction
- [ ] Host impl enforces: domain allowlist, no cross-domain redirect
- [ ] Mother-child `HostState` carries `grants` + `http_client`
- [ ] `instantiate_child()` builds client with redirect policy
- [ ] Guest API `pub mod http` in `patina-plugin-api`
- [ ] Unit tests: URL validation (≥5 cases), manifest parsing, capability gating
- [ ] `cargo test --workspace` passes
- [ ] `./resources/git/pre-push-checks.sh` passes

## Status Log

| Date | Status | Note |
|------|--------|------|
| 2026-02-13 | ready | Extracted from [[plugin-ecosystem]] build order item #2. Design locked in parent spec. Implementation-ready. |
| 2026-02-13 | ready | Refined in session [[20260213-135136]]. RESOLVED: no HttpDispatchFn — reqwest is lib dep (Cargo.toml line 47). Added exact files list, validate_http_url function, "What NOT to Touch" section, commit plan with scalpel discipline. |
