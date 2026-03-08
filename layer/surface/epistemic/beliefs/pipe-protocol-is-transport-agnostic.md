---
type: belief
id: pipe-protocol-is-transport-agnostic
persona: architect
facets: [architecture, pipes, protocol, mcp]
entrenchment: medium
status: active
endorsed: true
extracted: 2026-03-05
revised: 2026-03-05
---

# pipe-protocol-is-transport-agnostic

The pipe protocol (config in, facts out) works over any transport — stdio for local, HTTP+SSE for remote, Streamable HTTP for shared — same message format, same fact schema, different wire, following the MCP pattern where deployment topology doesn't dictate protocol design

## Statement

The pipe protocol (config in, facts out) works over any transport — stdio for local, HTTP+SSE for remote, Streamable HTTP for shared — same message format, same fact schema, different wire, following the MCP pattern where deployment topology doesn't dictate protocol design

## Evidence

- [[session-20260305-224446]]: [[session-20260305-224446]] - Analyzed why MCP has three transports (stdio, HTTP+SSE, Streamable HTTP): deployment topology. Mapped same reasoning to pipes: local (spawn+stdio), remote (VPS over HTTP), shared (multi-Mother). Protocol constant, transport variable (weight: 0.9)

## Supports

- [[pipes-are-processes-not-wasm]] — process-based pipes naturally support multiple transports
- [[patina-is-domain-agnostic-knowledge-system]] — transport agnosticism means pipes work in any deployment: dev laptop, production server, edge worker
- [[wit-defines-pipe-contract-not-runtime]] — WIT defines the message types, transport carries them

## Attacks

- Attacks "stdio-only" simplicity arguments — a single transport limits deployment topology

## Attacked-By

- "YAGNI — just build stdio" — multiple transports adds complexity before there's a remote pipe use case. Counter: designing transport-agnostic from the start costs nothing if the protocol layer (message format) is the constant. Don't build HTTP now, just don't hardcode stdio assumptions

## Applied-In

- [[spec-pipe-architecture]] — pipe protocol design
- `src/mcp/server/mod.rs` — Patina's MCP server already demonstrates stdio transport; same pattern for pipes

## Revision Log

- 2026-03-05: Created — metrics computed by `patina scrape`
