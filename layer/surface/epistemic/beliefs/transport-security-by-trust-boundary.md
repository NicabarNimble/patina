---
type: belief
id: transport-security-by-trust-boundary
persona: architect
facets: [security, architecture, networking]
entrenchment: medium
status: active
endorsed: true
extracted: 2026-02-03
revised: 2026-02-03
---

# transport-security-by-trust-boundary

Split transport by trust boundary: Unix socket for local (file permissions are auth), TLS or SSH tunnel for network (mutual key auth), never plain HTTP on a TCP port as default.

## Statement

Split transport by trust boundary: Unix socket for local (file permissions are auth), TLS or SSH tunnel for network (mutual key auth), never plain HTTP on a TCP port as default.

## Evidence

- [[session-20260203-164327]] - Security hardening review: bearer-over-HTTP stops blind CSRF but not network observers. The serve token (`PATINA_SERVE_TOKEN`) bypasses Patina's own vault/keychain secrets infrastructure. (weight: 0.9)
- [[security-hardening]] SPEC P0 review — Content-Length trust, thread leak, semaphore TOCTOU all stem from bolting security onto HTTP rather than choosing a transport that's secure by construction. (weight: 0.8)
- Prior art: PostgreSQL defaults to Unix socket, requires explicit `listen_addresses` for TCP. Docker daemon uses `/var/run/docker.sock`. Both use file permissions as local auth. (weight: 0.7)
- djb design principle (NaCl, qmail, djbdns): make the default safe, make the insecure path hard. Current design inverts this — HTTP port is default, security requires extra config. (weight: 0.7)

## Supports

- [[dependable-rust]] — small stable interface, hide transport details behind `internal.rs`
- [[mcp-is-shim-cli-is-product]] — if CLI is the product, local transport (Unix socket) is the primary path

## Attacks

<!-- Add beliefs this defeats -->

## Attacked-By

- Containers can't use Unix sockets across network boundaries — the TCP path must exist for docker host↔container. Belief doesn't eliminate HTTP, it constrains it to explicit opt-in with mutual auth.

## Applied-In

- `src/commands/serve/internal.rs` — P0 hardening (bearer token, read cap, limit cap, security headers) as interim measure while transport is HTTP
- `src/secrets/session.rs` — session cache talks to serve daemon over localhost HTTP; would benefit from Unix socket path
- `src/mother/internal.rs` — Mother client calls `/api/scry` without auth token; needs transport-aware redesign

## Revision Log

- 2026-02-03: Created — metrics computed by `patina scrape`
