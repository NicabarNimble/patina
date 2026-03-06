---
type: belief
id: mother-holds-connections-pipes-transform
persona: architect
facets: [architecture, pipes, streaming, mother]
entrenchment: medium
status: active
endorsed: true
extracted: 2026-03-05
revised: 2026-03-05
---

# mother-holds-connections-pipes-transform

Mother manages all external connections (WebSockets, webhooks, polling) while pipes are stateless transform functions (data in via stdin, facts out via stdout) — keeping the pipe interface uniform regardless of source transport

## Statement

Mother manages all external connections (WebSockets, webhooks, polling) while pipes are stateless transform functions (data in via stdin, facts out via stdout) — keeping the pipe interface uniform regardless of source transport

## Evidence

- [[session-20260305-224446]]: [[session-20260305-224446]] - Solved WebSocket/streaming problem: Mother holds Slack WebSocket, buffers messages, feeds batches to pipe over stdin. Pipe doesn't know about WebSocket. Same interface for REST poll, WebSocket push, webhook receive, RSS (weight: 0.9)

## Supports

- [[unix-philosophy]] — pipes as single-purpose transforms, Mother as orchestrator — composition of simple tools
- [[pipes-are-processes-not-wasm]] — stateless transform model works because pipe is a process that reads stdin and writes stdout
- [[scrape-is-local-capture]] — scrape handles local (git), Mother+pipes handle external — clear separation

## Attacks

- Attacks "pipes should own their connections" for real-time sources — Mother's daemon lifecycle is the right place for long-lived connections, not ephemeral pipe processes

## Attacked-By

- "First-party stream pipes can own connections" — trusted eth-pipe could hold its own WebSocket. Counter: this is acceptable for first-party, but the default model should be Mother-managed for uniformity. First-party pipes that own connections are an optimization, not the architecture

## Applied-In

- [[spec-pipe-architecture]] — Mother pipe manager design
- [[spec-continuous-operation]] — Mother daemon already manages child lifecycle; pipe management extends this

## Revision Log

- 2026-03-05: Created — metrics computed by `patina scrape`
