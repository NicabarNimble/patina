---
type: belief
id: patina-is-domain-agnostic-knowledge-system
persona: architect
facets: [architecture, identity, plugins, data-lakes]
entrenchment: medium
status: active
endorsed: true
extracted: 2026-03-02
revised: 2026-03-02
---

# patina-is-domain-agnostic-knowledge-system

Patina is a general-purpose knowledge system, not a development tool. The core engine — event sourcing, beliefs, search, embeddings, Mother — is domain-agnostic. Plugins determine what domain Patina operates in: code grammars for development, Google Workspace for business automation, Slack/Zoom for communication intelligence. A production Patina instance monitoring a data lake and building business beliefs uses the same engine as a development Patina instance tracking code architecture. You build with Patina, then deploy AS Patina.

## Statement

Patina is a general-purpose knowledge system, not a development tool. The core engine — event sourcing, beliefs, search, embeddings, Mother — is domain-agnostic. Plugins determine what domain Patina operates in: code grammars for development, Google Workspace for business automation, Slack/Zoom for communication intelligence. A production Patina instance monitoring a data lake and building business beliefs uses the same engine as a development Patina instance tracking code architecture. You build with Patina, then deploy AS Patina.

## Evidence

- [[session-20260302-072907]]: Design session: traced the path from data lakes to production agents. Key realization: a client's production system (monitoring emails, automating business) IS a Patina instance with business plugins instead of code plugins. Same beliefs engine, different scrape plugins. You build it with Patina then set it free as its own Patina system. (weight: 0.95)
- [[session-20260302-072907]]: Three-layer external data model (lake → projection → consumption) works identically for development (code) and production (business data). The layers don't assume development context. (weight: 0.8)
- [[session-20260302-061023]]: QMD analysis reframed Patina around the belief↔reality loop. The loop works regardless of what "reality" is — git commits or email streams. (weight: 0.7)

## Supports

- [[beliefs-are-the-product]] — Strengthened: beliefs are the product regardless of domain. Business beliefs about email patterns have the same structure as architecture beliefs about code.
- [[patina-is-knowledge-protocol]] — Evolves: the protocol framing is correct but scoped too narrowly to "development knowledge." This belief drops the qualifier — it's a knowledge protocol, period.
- [[four-layer-architecture]] — The four layers (beliefs, assay, scry, mother) are domain-agnostic by construction. Nothing in them assumes code.

## Attacks

- "Patina is a development tool" — The crates.io description says "Context orchestration for AI development." This belief says development is one application, not the identity. The engine doesn't change when plugins change.

## Attacked-By

- "Generalization dilutes focus" — Valid tension. Patina's current user base is developers. Broadening the identity risks losing the development story before it's proven. Counter: the plugin system already decouples core from domain.
- "Business users won't use a CLI tool" — Valid. Production Patina instances may need different interfaces (API, web). The core is the same but the surface needs to adapt.

## Applied-In

- [[passive-operational-memory-ledger]] — Google Workspace data project: builds a data lake that a production Patina instance could consume via scrape plugins, forming business beliefs instead of code beliefs

## Revision Log

- 2026-03-02: Created — metrics computed by `patina scrape`
