---
type: refactor
id: http-proxy-extraction
status: active
created: 2026-03-10
sessions:
  origin: 20260310-074810
related:
- patina-connect
- ducklake
- pipe-architecture
beliefs:
- telemetry-is-process-owned
- children-have-agency-toys-are-capabilities
- initialize-is-capability-grant
exit_criteria:
- id: proxy-in-patina-pipe
  text: 'patina-pipe crate exports a production HTTP proxy handler behind an `http-proxy` feature flag: domain validation, credential injection, leak detection'
  checked: true
- id: broker-consumes-shared
  text: broker/http.rs imports the proxy from patina-pipe instead of implementing its own; behavior unchanged
  checked: true
- id: security-preserved
  text: 'All security properties preserved: HTTPS-only, domain allowlist, no IP/localhost, credential injection (Bearer/Header/InProcess), cross-domain redirect rejection, response leak detection'
  checked: true
- id: harness-is-production
  text: ChildConnection in patina-pipe is acknowledged as production API, not test harness; spawn_with_http is a first-class method
  checked: true
- id: mapping-tests
  text: Exhaustive tests verify AuthPlan → HttpProxyConfig mapping covers all InjectionStrategy variants correctly
  checked: true
---
# refactor: Extract HTTP proxy to shared crate as toy capability

> The HTTP proxy (domain validation, credential injection, leak
> detection) lives in broker/http.rs today. Only Mother can proxy
> HTTP for connector toys. Move it to patina-pipe so any child
> that uses connector toys can provide the same secured proxy.
>
> The proxy is a toy — a capability that enforces policy when used.
> It does not emit telemetry. Each actor that uses the proxy owns
> its own observability. Per [[children-have-agency-toys-are-capabilities]].

## Current State

HTTP proxy security stack is split across two locations:

- `src/http_util.rs` — low-level utilities already extracted:
  `validate_http_url`, `build_http_client`, `leak_check`
- `src/broker/http.rs` — `build_production_handler` wires them
  into a full proxy: domain allowlist check → build request →
  inject credential → make call → leak check response

`build_production_handler` returns an `HttpHandler` (closure) that
`ChildConnection` calls when a child sends a `pipe/http` request.
It depends on `AuthPlan`, `InjectionStrategy`, `ResolvedCredential`
from `src/connect/`.

Today only the broker can create this handler. Children that use
connector toys (like the DuckLake child using a github-connector
toy) cannot build their own proxy from approved capabilities.

Additionally, `ChildConnection` in `harness.rs` presents itself
as test infrastructure but is used in production by the broker
(`broker/spawn.rs:190`). Children that use connector toys need
`ChildConnection` to talk to those toys — it is production
substrate, not test infrastructure.

## Target State

### Shared HTTP Proxy

`patina-pipe` exports a proxy builder behind an `http-proxy`
feature flag. Any binary with credentials and a domain allowlist
can use it:

```rust
// crates/patina-pipe/src/http_proxy.rs
pub fn build_http_proxy(config: HttpProxyConfig) -> HttpHandler {
    // domain allowlist check
    // credential injection (Bearer, Header, or InProcess)
    // leak detection on response
    // cross-domain redirect rejection
    // NO telemetry — caller's concern
}

pub struct HttpProxyConfig {
    pub allowed_domains: Vec<String>,
    pub credential: Option<ProxyCredential>,
}

pub struct ProxyCredential {
    pub value: String,
    pub injection: ProxyInjection,
}

pub enum ProxyInjection {
    Bearer,
    Header { name: String },
    InProcess,  // credential exists but delivered via pipe/initialize, not HTTP
}
```

The types are proxy-specific — no dependency on `AuthPlan` or
`connect` module. The broker maps `AuthPlan → HttpProxyConfig`.
The DuckLake child maps its approved toy capabilities from
`pipe/initialize` → `HttpProxyConfig`.

### Feature-Gated reqwest

```toml
# crates/patina-pipe/Cargo.toml
[features]
http-proxy = ["reqwest"]

[dependencies]
reqwest = { version = "0.12", features = ["blocking", "rustls-tls"], optional = true }
```

Children that proxy HTTP opt in: `patina-pipe = { features = ["http-proxy"] }`.
Children that don't get the lean protocol-only crate.

Note: Cargo features are additive — if any crate in the build
graph enables `http-proxy`, it's on for that package. This is a
packaging convenience and API boundary, not a strong isolation
guarantee. If a third "host service" accumulates in patina-pipe
beyond protocol and HTTP proxy, that's the signal to split crates.

### ChildConnection as Production API

`harness.rs` is renamed or restructured to acknowledge that
`ChildConnection` is production infrastructure:

- `spawn()` — spawn a child process (existing)
- `spawn_with_http(path, handler)` — spawn with HTTP proxy (new)
- `request()` — send pipe protocol request (existing)
- `shutdown()` — clean shutdown (existing)

Test-only helpers (if any) stay in a `#[cfg(test)]` block or
move to a `test_support` module.

### Broker as Thin Wrapper

`broker/http.rs` becomes a mapping layer + telemetry:

```rust
pub fn build_production_handler(auth_plan: &AuthPlan, child_name: &str) -> Result<HttpHandler> {
    let proxy = patina_pipe::http_proxy::build_http_proxy(HttpProxyConfig {
        allowed_domains: auth_plan.allowed_domains.clone(),
        credential: auth_plan.credential.as_ref().map(|c| ProxyCredential {
            value: c.value.clone(),
            injection: match &c.injection {
                InjectionStrategy::Bearer => ProxyInjection::Bearer,
                InjectionStrategy::Header { name } => ProxyInjection::Header { name: name.clone() },
                InjectionStrategy::InProcess => ProxyInjection::InProcess,
            },
        }),
    });

    // Broker wraps the proxy with its own measurement
    let child = child_name.to_string();
    Ok(Box::new(move |req: &PipeHttpRequest| {
        let start = std::time::Instant::now();
        let result = proxy(req);
        // emit crate::measure event with duration, status, child name
        emit_http_measure(&child, start, &result);
        result
    }))
}
```

The proxy enforces security policy. The broker adds telemetry.
The DuckLake child can wrap the same proxy with its own telemetry
(or not — its choice).

## Design Decisions

### Proxy enforces policy, not telemetry

The shared proxy does not emit measurements, log, or call back.
It validates domains, injects credentials, detects leaks, and
returns results. Each process that uses the proxy decides what
to observe.

This follows [[telemetry-is-process-owned]]: Mother measures
orchestration, children measure their own operations. Shared
substrate enforces behavior, not observability.

### Config mapping is a security boundary

The mapping from `AuthPlan → HttpProxyConfig` is where policy
decisions live. Exhaustive tests verify every `InjectionStrategy`
variant maps to the correct `ProxyInjection`. The dangerous
direction is `InProcess → Bearer` or `Header` — that would
change the secret delivery channel and create unintended HTTP
exposure. Tests catch this.

### patina-pipe is broker substrate

This extraction acknowledges that `patina-pipe` is not just wire
protocol. It is the substrate for any process that uses toys —
spawning connector toys, proxying their HTTP, managing pipe
protocol communication. `ChildConnection` was already there.
HTTP proxy makes it explicit.

Children with agency (like DuckLake) use approved connector toys
directly. The child becomes a mini-broker for its toys, using
the same substrate Mother uses. Per
[[children-have-agency-toys-are-capabilities]]: children use
approved toys on their own; Mother grants capabilities but
stays out of the data path.

## Steps

1. Promote `harness.rs` — rename or restructure to acknowledge
   `ChildConnection` as production API. Add `spawn_with_http`.
2. Add `http-proxy` feature flag to patina-pipe Cargo.toml
3. Move `http_util.rs` functions into `patina-pipe/src/http_proxy.rs`
4. Add `HttpProxyConfig`, `ProxyCredential`, `ProxyInjection`
   types to patina-pipe
5. Add `build_http_proxy` — full proxy handler, no telemetry
6. Refactor `broker/http.rs` to wrap shared proxy + add measurement
7. Exhaustive mapping tests for all InjectionStrategy variants
8. Verify all existing tests pass, security properties unchanged

## Key Files

**Move to patina-pipe:**
- `src/http_util.rs` → `crates/patina-pipe/src/http_proxy.rs`

**Restructure in patina-pipe:**
- `crates/patina-pipe/src/harness.rs` — promote to production API

**Refactor:**
- `src/broker/http.rs` — thin wrapper: mapping + measurement

**Unchanged:**
- `src/connect/` — AuthPlan types stay, broker maps to proxy types

## Non-Goals

- Changing the connector sandbox model
- Adding new security checks
- Modifying the pipe protocol
- Async HTTP (stays blocking for v1)
- Telemetry hooks or callbacks in the proxy
- Splitting patina-pipe into multiple crates (revisit if a third
  host service accumulates)
