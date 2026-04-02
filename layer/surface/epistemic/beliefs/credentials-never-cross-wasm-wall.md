---
type: belief
id: credentials-never-cross-wasm-wall
persona: architect
facets: [security, architecture, toys, wasm]
entrenchment: medium
status: active
endorsed: true
extracted: 2026-03-26
revised: 2026-03-26
---

# credentials-never-cross-wasm-wall

Credentials never cross the WASM boundary. Children operate through opaque connection-name handles via patina:connect. Mother injects credentials on the host side. A compromised child can use a connection but cannot steal the credential. This is stronger than Cloudflare Workers where Workers see secret values as strings.

## Statement

Credentials never cross the WASM boundary. Children operate through opaque connection-name handles via patina:connect. Mother injects credentials on the host side. A compromised child can use a connection but cannot steal the credential. This is stronger than Cloudflare Workers where Workers see secret values as strings.

## Evidence

- [[session-20260325-150227-161735000]] - Discovered that Cloudflare Workers see env.GITHUB_TOKEN as a string (exfiltrable). Patina's patina:connect uses opaque WASM resource handles — the credential never enters WASM memory. Mother injects Authorization headers host-side at dispatch time. (weight: 0.95)

## Supports

- [[children-have-agency-toys-are-capabilities]] — toys are capability grants, not credential distributors
- [[children-are-wasm]] — WASM sandbox provides the real isolation boundary that makes this enforceable
- [[host-proxied-io-is-the-security-model]] — all IO goes through the host; credentials are a special case of this

## Attacks

- Cloudflare Workers secret model (secrets as environment strings) — weaker because Workers can read and exfiltrate secret values. Patina's opaque resource handles prevent this.

## Attacked-By

- Performance: host-side credential injection adds overhead per HTTP call. Mitigated by: the overhead is one hash lookup + header injection, negligible compared to network latency.
- Debugging: children can't inspect their own auth headers for troubleshooting. Mitigated by: Mother can log full request details (including injected headers) in audit trail; child sees the response.
- Third-party children may want direct credential access for complex auth flows (OAuth refresh, mTLS). Mitigated by: Mother handles auth flows host-side; child only sees the refreshed connection handle.

## Applied-In

- [[toy-collapse-wasi-alignment]] — `patina:connect` bridge design with opaque `connection` resource type. Binding mechanics section defines the definitive contract.
- [[cloudflare-worker-child]] — portable child spec documents how credential injection differs between Patina (host-side) and Cloudflare (env string).
- Default-deny raw http policy in [[toy-collapse-wasi-alignment]] — connect-mediated access is the secure default.

## Revision Log

- 2026-03-26: Created — metrics computed by `patina scrape`
