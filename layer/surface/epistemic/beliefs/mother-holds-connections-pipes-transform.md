---
type: belief
id: mother-holds-connections-pipes-transform
persona: architect
facets: [architecture, children, streaming, mother, broker]
entrenchment: medium
status: active
endorsed: true
extracted: 2026-03-05
revised: 2026-03-06
---

# mother-holds-connections-pipes-transform

Transport children hold external connections (WebSockets, webhooks), connector children transform data into facts. Mother orchestrates both but doesn't hold connections or transform data — she spawns, monitors, routes, and validates. Each actor has one job.

## Statement

Transport children hold external connections (WebSockets, webhooks), connector children transform data into facts. Mother orchestrates both but doesn't hold connections or transform data — she spawns, monitors, routes, and validates. Each actor has one job.

## Evidence

- [[session-20260305-224446]]: Solved WebSocket/streaming problem: transport child holds connection, buffers messages, feeds to connector child via pipe protocol. Connector doesn't know about WebSocket — same interface for REST poll, WebSocket push, webhook receive, RSS. (weight: 0.9)
- [[session-20260306-123021]]: Architecture reframe: Mother = broker (Netflix/Kafka pattern). Mother routes facts from sources to destinations. Transport children hold connections. Connector children transform. Mother orchestrates but doesn't do either. (weight: 0.9)

## Supports

- [[unix-philosophy]] — children as single-purpose services (one job each), Mother as orchestrator — composition of simple tools
- [[pipes-are-processes-not-wasm]] — child taxonomy (connector, transport, lakehouse, transform) maps to clear responsibilities
- [[scrape-is-local-capture]] — scrape handles local (git), children handle external — clear separation

## Attacks

- Attacks "Mother should own connections" — Mother is the broker, not the transport. Transport children own connections. This keeps Mother stateless with respect to external services.
- Attacks "children should own their routing" — routing is Mother's job (broker pattern). Children emit facts, Mother routes them.

## Attacked-By

- "Simpler if Mother holds connections for small setups" — for 1-2 sources, a transport child is overhead. Counter: the architecture should be right at scale. Small setups still work (connector children handle simple REST polling directly, transport children only needed for complex connections like WebSockets).

## Applied-In

- [[spec-pipe-architecture]] — child taxonomy (connector, transport, lakehouse, transform) and Mother as broker
- [[spec-continuous-operation]] — Mother daemon manages child scheduling; transport children hold long-lived connections

## Revision Log

- 2026-03-05: Created — Mother holds connections, pipes transform
- 2026-03-06: Revised — transport children hold connections, connector children transform, Mother orchestrates. Aligned with broker model.
