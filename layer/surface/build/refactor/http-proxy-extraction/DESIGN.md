# Design: Extract HTTP proxy to shared crate for child reuse

## Approach

The HTTP proxy security stack currently lives in two places in the
main binary:

- `src/http_util.rs` — `build_http_client` (redirect rejection),
  `validate_http_url` (HTTPS-only, no IP/localhost), `leak_check`
  (credential scan). Pure functions, no state.
- `src/broker/http.rs` — `build_production_handler` wires http_util
  into a closure that validates domains, injects credentials per
  `InjectionStrategy`, emits a Measure event, and returns an
  `HttpHandler` (`Box<dyn FnMut(&PipeHttpRequest) -> Result<...>>`).

The extraction moves the security stack to `patina-pipe` behind
an `http-proxy` feature flag and introduces proxy-local types so
patina-pipe has no dependency on the main binary's `connect` module.
The broker becomes a thin mapper: `AuthPlan -> HttpProxyConfig`
plus telemetry wrapping.

`ChildConnection` in `harness.rs` is already used in production
(`broker/spawn.rs:190`). This refactor acknowledges it as production
substrate and adds `spawn_with_http` as a first-class method
(replacing the free function `spawn_child_with_handler`).

### What moves, what stays

| Code | From | To | Notes |
|------|------|----|-------|
| `build_http_client` | `src/http_util.rs:10` | `patina-pipe/src/http_proxy.rs` | Behind `http-proxy` feature |
| `validate_http_url` | `src/http_util.rs:30` | `patina-pipe/src/http_proxy.rs` | Behind `http-proxy` feature |
| `leak_check` | `src/http_util.rs:61` | `patina-pipe/src/http_proxy.rs` | Behind `http-proxy` feature |
| `normalize_domain` | `src/broker/http.rs:143` | `patina-pipe/src/http_proxy.rs` | Private, used by proxy |
| Domain allowlist logic | `src/broker/http.rs:28-53` | `patina-pipe/src/http_proxy.rs` | Part of `build_http_proxy` |
| Credential injection | `src/broker/http.rs:77-89` | `patina-pipe/src/http_proxy.rs` | Via `ProxyInjection` enum |
| Request dispatch | `src/broker/http.rs:56-94` | `patina-pipe/src/http_proxy.rs` | Inside proxy closure |
| Measure emission | `src/broker/http.rs:119-136` | Stays in `src/broker/http.rs` | Broker wraps proxy with own telemetry |
| `build_production_handler` | `src/broker/http.rs:24` | Stays, becomes thin wrapper | Maps AuthPlan -> HttpProxyConfig |
| `HttpHandler` type | `patina-pipe/src/harness.rs:18` | Stays in harness.rs | Already in the right crate |
| `ChildConnection` | `patina-pipe/src/harness.rs:26` | Stays, gains `spawn_with_http` | Promoted from test to production API |

### Security properties preserved

Every property in `broker/http.rs` transfers to the shared proxy:

1. **HTTPS only** — `validate_http_url` rejects `http://`
2. **No IP/localhost** — `validate_http_url` rejects IPs, localhost
3. **Domain allowlist** — normalized set, case-insensitive, port-stripped
4. **Credential injection** — `Bearer`, `Header { name }`, `InProcess`
5. **Cross-domain redirect rejection** — `build_http_client` policy
6. **Leak detection** — `leak_check` scans response body
7. **SANDBOX_DEBUG logging** — configurable domain rejection logging

The proxy returns `Result<PipeHttpResponse, String>` matching the
existing `HttpHandler` signature. No new error types needed.

## Commits

1. **`pipe: add http-proxy feature with proxy types`**
   Add `[features] http-proxy = ["reqwest"]` to `crates/patina-pipe/Cargo.toml`.
   Create `crates/patina-pipe/src/http_proxy.rs` with `HttpProxyConfig`,
   `ProxyCredential`, `ProxyInjection` types and `build_http_proxy()`.
   Move `build_http_client`, `validate_http_url`, `leak_check`, `normalize_domain`
   from main binary. The module is `#[cfg(feature = "http-proxy")]`.
   Tests: domain validation, credential injection for all three strategies,
   leak detection, redirect rejection, SANDBOX_DEBUG.

   **Why:** The shared proxy must exist before the broker can consume it
   or the DuckLake child can use it. Feature-gating keeps patina-pipe
   lean for children that don't need HTTP.

2. **`pipe: promote ChildConnection with spawn_with_http`**
   Add `ChildConnection::spawn_with_http(path, handler)` as a constructor.
   Update module docs to reflect production usage. The existing
   `spawn_child_with_handler` free function becomes a thin delegate to
   the new constructor (backwards compat, no breakage).

   **Why:** Children with agency need to spawn connector toys using
   ChildConnection directly. It must be a documented production API,
   not "test harness".

3. **`broker: consume shared proxy, keep telemetry`**
   Refactor `src/broker/http.rs`: `build_production_handler` maps
   `AuthPlan -> HttpProxyConfig`, calls `patina_pipe::http_proxy::build_http_proxy`,
   wraps the result with `measure::emit_or_warn`. Delete broker's inline
   domain validation, credential injection, client construction. Keep
   `normalize_domain` tests as regression tests against the shared impl.
   Delete `src/http_util.rs` — all functions moved to patina-pipe.

   **Why:** Broker becomes a thin mapper + telemetry wrapper. The
   security stack lives in one place. DuckLake child will use the
   same proxy with its own (or no) telemetry.

4. **`broker: exhaustive AuthPlan -> HttpProxyConfig mapping tests`**
   Add targeted tests for the mapping boundary: Bearer -> ProxyInjection::Bearer,
   Header { name } -> ProxyInjection::Header { name },
   InProcess -> ProxyInjection::InProcess. Verify the dangerous direction
   (InProcess should NOT inject into HTTP headers) is covered.

   **Why:** The mapping is a security boundary — the only place where
   policy decisions translate to proxy behavior. Exhaustive tests catch
   regressions if new InjectionStrategy variants are added.

## Key Files

### New
- `crates/patina-pipe/src/http_proxy.rs` — shared proxy: `build_http_proxy`, `HttpProxyConfig`, `ProxyCredential`, `ProxyInjection`, plus moved `build_http_client`, `validate_http_url`, `leak_check`, `normalize_domain`

### Modified
- `crates/patina-pipe/Cargo.toml` — add `http-proxy` feature, optional `reqwest` dep
- `crates/patina-pipe/src/lib.rs` — conditional `pub mod http_proxy`
- `src/broker/http.rs` — thin wrapper: AuthPlan -> HttpProxyConfig mapping + measure emission
- `src/broker/spawn.rs` — import change (HttpHandler stays in same crate, no code change needed)

### Deleted
- `src/http_util.rs` — functions move to `patina-pipe/src/http_proxy.rs`

### Unchanged
- `src/connect/` — AuthPlan, InjectionStrategy, ResolvedCredential stay
- `crates/patina-pipe/src/harness.rs` — HttpHandler type stays, gains spawn_with_http
- `crates/patina-pipe-types/src/http.rs` — PipeHttpRequest/Response types stay

## Open Questions

1. **reqwest version alignment.** Main binary already depends on reqwest
   (via `Cargo.toml`). patina-pipe's feature-gated reqwest must use the
   same version to avoid duplicate compilation. Verify with `cargo tree -d`.

2. **http_util.rs consumers.** `src/http_util.rs` docstring says it's used
   by `plugin/internal/host_support.rs` and `broker/http.rs`. Verify
   host_support.rs usage before deleting — if it uses http_util, it needs
   to import from patina-pipe or keep a thin re-export.
