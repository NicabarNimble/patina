---
type: refactor
id: pipe-mother-io
status: draft
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
  text: 'Mother exposes `pipe/http` JSON-RPC method for native children: takes url/method/headers/body, enforces manifest domains, executes HTTPS request, streams response metadata/body fragments back.'
  verify: '`cargo test -p patina-mother pipe_http::*` (unit tests) AND `patina mother run test-http-child` (integration) both succeed. Capture stderr logs showing allowlisted host passes and denylisted host rejected with explicit error.'
  checked: false
- id: sandbox-denies-outbound
  text: 'Native child sandbox profile blocks all outbound network (no 443 escape hatch). Only Mother performs HTTP. Landlock+macOS profiles updated and tested.'
  verify: 'Spawn test child that calls `reqwest::get("https://example.com")` directly. Expect EACCES/EPERM before connect. `patina mother run test` with PATINA_SANDBOX_DEBUG=1 shows sandbox block message.'
  checked: false
- id: patina-pipe-helper
  text: '`patina-pipe` exposes `MotherHttpClient` (builder pattern) so child code replaces `reqwest::Client` with helper. Helper speaks pipe/http under the hood, returns familiar response structs.'
  verify: '`cargo test -p patina-pipe mother_http::*` passes. Example child (examples/test-child) compiles using helper with zero direct reqwest references.'
  checked: false
- id: measure-instrumentation
  text: 'Every pipe/http call emits Measure events (duration, bytes, policy decision, manifest id) and integrates with PATINA_SANDBOX_DEBUG logging for auditability.'
  verify: '`patina mother run test` prints request audit lines. `sqlite3 patina.db "SELECT count(*) FROM measure_events WHERE event_type = ''pipe.http''"` > 0 after test.'
  checked: false
---
# refactor: Pipe Mother I/O — Proxied HTTP for Native Children

> Give native children the same host-proxied networking contract that
> WASM children already rely on. Mother terminates every HTTP request,
> enforces manifest domains, injects credentials, logs everything.
> The OS sandbox blocks direct sockets; the only way out is pipe/http.

## Problem

Native children currently expect to call `reqwest` directly. The OS
sandbox can only filter by port/IP, so the manifest `domains` field
is unenforced: declaring `[“api.github.com”]` or
`[“www.googleapis.com”]` does nothing once DNS resolves. The real
solution is host-proxied I/O. Until this exists the security story
is dishonest.

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
