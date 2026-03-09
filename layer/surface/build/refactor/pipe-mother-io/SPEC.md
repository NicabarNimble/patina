---
type: refactor
id: pipe-mother-io
status: active
created: 2026-03-07
blocked_by:
- pipe-protocol-types
- pipe-native-transport
related:
- pipe-architecture
- mother-broker
beliefs:
- host-proxied-io-is-the-security-model
- pipes-are-processes-not-wasm
exit_criteria:
- id: pipe-http-requests
  text: 'Mother exposes `pipe/http` JSON-RPC method for native children: takes url/method/headers/body, enforces manifest domains, executes HTTPS request, returns response. ChildConnection in harness.rs handles pipe/http requests via HttpHandler callback.'
  checked: true
  verify: '`cargo test -p patina-pipe harness::tests::integration_pipe_http_domain_enforcement` passes. Allowed domain returns 200, denied domain gets explicit rejection.'
- id: sandbox-denies-outbound
  text: Native child sandbox profile blocks all outbound network (no 443 escape hatch). Only Mother performs HTTP. Landlock+macOS profiles updated and tested.
  checked: true
  verify: 'macOS: `cargo test -p patina-pipe sandbox::macos_tests::profile_denies_all_network` passes — no network-outbound rules in profile. Fork test applies sandbox, verifies enforcement. Linux: `./scripts/test-linux.sh -p patina-pipe` verifies Landlock denies ALL ports including 443.'
- id: patina-pipe-helper
  text: '`patina-pipe` exposes `PipeIo` (builder pattern via get/post/header/send) so child code uses proxied HTTP through Mother. PipeIo combines fact emission and HTTP — the unified context for Child::fetch().'
  checked: true
  verify: '`cargo test -p patina-pipe pipe_io::tests` passes. Example child (examples/test-http-child) compiles using PipeIo with zero direct reqwest references.'
- id: measure-instrumentation
  text: Every pipe/http call emits Measure events (duration, bytes, policy decision, manifest id) and integrates with PATINA_SANDBOX_DEBUG logging for auditability.
  checked: false
  verify: '`patina mother run test` prints request audit lines. `sqlite3 patina.db "SELECT count(*) FROM measure_events WHERE event_type = ''pipe.http''"` > 0 after test.'
---
# refactor: Pipe Mother I/O — Proxied HTTP for Native Children

> Give native children the same host-proxied networking contract that
> WASM children already rely on. Mother terminates every HTTP request,
> enforces manifest domains, injects credentials, logs everything.
> The OS sandbox blocks direct sockets; the only way out is pipe/http.

## How We Got Here

[[spec-pipe-native-transport]] built the native child runtime: Child
trait, run() dispatcher, FactEmitter, macOS sandbox via sandbox_init(),
Linux Landlock. All working, all tested.

During testing (session 20260307-092539), we discovered the OS sandbox
has a fundamental limitation: both macOS SBPL and Linux Landlock
operate on ports and IPs, not hostnames. The sandbox can block port 80
but cannot distinguish `api.github.com` from `evil.com` — both use
port 443. The manifest's `domains` field was unenforced.

The original spec assumed domain-level filtering at the OS sandbox
layer. That assumption was wrong. This spec fixes it.

**The fix:** mirror what WASM children already have. WASM children
call `host_http_request()` and Mother checks the domain. Native
children call `pipe/http` over stdio and Mother does the same check.
The OS sandbox blocks ALL direct sockets — children can't bypass
Mother. The belief [[host-proxied-io-is-the-security-model]] is
delivered, not aspirational.

**What exists now (from pipe-native-transport):**
- `crates/patina-pipe/src/sandbox.rs` — macOS SBPL and Linux Landlock
  profiles currently allow port 443 + DNS (port-level restriction)
- Fork-based tests verify enforcement on both platforms
- `generate_macos_profile()` and `apply_landlock()` are the functions
  this spec will tighten (remove 443/53 rules)

**What this spec adds:**
- `pipe/http` JSON-RPC method (Mother-side handler)
- `MotherHttpClient` helper (child-side, in patina-pipe)
- Sandbox tightening (deny ALL outbound sockets)
- Measure instrumentation for auditability

pipe/http is the universal HTTP proxy for ALL native children —
GitHub, Google Workspace, Slack, any REST API. Each connector's
manifest declares its allowed domains; Mother enforces them
identically regardless of provider.

## Solution

Mirror the WASM `host_http_request` pattern for native children:

- Define `pipe/http` JSON-RPC method in the pipe protocol (same shape
  as existing WASM host call: method, url, headers, auth, streaming
  body support).
- Mother validates each request against the child manifest domains,
  credentials, and media-type limits, then performs the HTTPS request
  itself using its trusted networking stack.
- Responses stream back over stdio (status, headers, body chunks)
  with backpressure and cancellation support.
- Native children link `patina-pipe::MotherHttpClient`, a wrapper that
  mimics `reqwest::Client` ergonomics but forwards over the pipe.
- The OS sandbox profile becomes “default deny network.” Children that
  try raw sockets fail fast; PATINA_SANDBOX_DEBUG explains why.
- Measure instrumentation records every request (host, latency,
  bytes, allow/deny) for auditing.

## Steps

1. **Protocol definition:** Add `pipe/http` request/response schema to
   `patina-pipe-types` (method enum, headers map, streaming chunk
   format) and document in DESIGN.md.
2. **Mother handler:** Implement handler inside `patina mother run`
   that decodes requests, enforces domains, injects credentials, and
   executes HTTPS using `reqwest` (or shared client). Include deny
   path logging + Measure events.
3. **Child helper:** Add `MotherHttpClient` to `patina-pipe` with the
   same builder ergonomics as `reqwest::Client`. Provide blocking and
   async APIs as needed. Helper encodes JSON-RPC messages and handles
   streaming responses.
4. **Sandbox tightening:** Update macOS SBPL + Linux Landlock profiles
   to block ALL outbound sockets (no 443 allowlist). Provide short
   diagnostic flag for debugging blocked syscalls.
5. **Testing + examples:** Extend `examples/test-child` to fetch via
   `MotherHttpClient`. Include integration test that tries both
   allowlisted and denylisted domains, verifying proper errors and
   instrumentation.

## Non-Goals

- Building protocol adapters beyond HTTP (WebSockets, gRPC, SSE).
  Those get their own specs when needed.
- Credential acquisition workflows (OAuth, device flows). This spec
  assumes credentials already exist in Mother.
- Remote execution transports (SSH, gRPC services). Only local stdio
  children are covered here.
