---
type: belief
id: dispatch-on-strategy-not-provider
persona: architect
facets: [architecture, auth, runtime-design]
entrenchment: medium
status: active
endorsed: true
extracted: 2026-03-09
revised: 2026-03-09
---

# dispatch-on-strategy-not-provider

Runtime should dispatch on auth strategy (Bearer, Header, InProcess), not provider identity (GitHub, Slack). The connection record carries enough durable metadata to drive injection without provider-specific code in the broker.

## Statement

Runtime should dispatch on auth strategy (Bearer, Header, InProcess), not provider identity (GitHub, Slack). The connection record carries enough durable metadata to drive injection without provider-specific code in the broker.

## Evidence

- [[session-20260309-131917]] - patina-connect design session: broker/http.rs hardcoded Bearer injection (GitHub model baked into runtime). Redesigned to dispatch on InjectionStrategy enum driven by connection metadata, so adding providers never touches broker code (weight: 0.9)

## Supports

- [[defense-in-depth-over-perfect-isolation]] — Strategy dispatch is a layer of defense: even if a provider impl is wrong, the broker only does what the strategy enum allows.
- [[persistence-is-the-center]] — The connection record carries durable auth metadata that drives dispatch. No provider knowledge needed at runtime.

## Attacks

## Attacked-By

## Applied-In

- [[spec-patina-connect]] DESIGN.md §2 — `AuthPlan.credential.injection` dispatches Bearer/Header/InProcess in `broker/http.rs`, replacing the hardcoded Bearer at line 80. The broker never checks provider identity.
- `src/broker/http.rs:79-86` — Current code hardcodes Bearer (the anti-pattern this belief corrects).

## Revision Log

- 2026-03-09: Created — metrics computed by `patina scrape`
