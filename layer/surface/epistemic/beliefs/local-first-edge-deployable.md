---
type: belief
id: local-first-edge-deployable
persona: architect
facets: [architecture, deployment, local-first, edge, cloudflare]
entrenchment: medium
status: active
endorsed: true
extracted: 2026-03-04
revised: 2026-03-04
---

# local-first-edge-deployable

Patina core is local-first, always. Mac/Linux, git, SQLite, wasmtime. This is the primary implementation, not an abstraction over pluggable backends. Edge deployment (Cloudflare, Vercel) is for Patina apps — belief consumers that act on what the protocol produces. The local Patina is the belief factory. The edge app is a belief consumer. The protocol runs locally. The results deploy to wherever the action layer lives. Edge apps generate events that flow back to local Patina via Mother.

## Statement

Patina core is local-first, always. Mac/Linux, git, SQLite, wasmtime. This is the primary implementation, not an abstraction over pluggable backends. Edge deployment (Cloudflare, Vercel) is for Patina apps — belief consumers that act on what the protocol produces. The local Patina is the belief factory. The edge app is a belief consumer. The protocol runs locally. The results deploy to wherever the action layer lives. Edge apps generate events that flow back to local Patina via Mother.

## Evidence

- [[session-20260304-120702]]: User: "where patina will live is mac/linux local-first is the primary but there should be a way to connect with cloudflare... a patina web app for an online chat agent needs a way to exist." Established: don't abstract local, design the interface between Patina and apps. (weight: 0.95)
- [[session-20260304-120702]]: Cloudflare capability mapping — D1 (SQLite), Workers (WASM), Vectorize (vectors), Workers AI (embeddings). Protocol maps well, but oxidize needs a different backend (Workers AI vs local ONNX). Edge is a consumer with adapted backends, not a full port. (weight: 0.85)
- [[session-20260304-120702]]: Edge apps generate new evidence (chat conversations, user interactions) that flows BACK to local Patina to feed the belief lifecycle. The edge app is a child node, Mother is the connection. (weight: 0.8)

## Supports

- [[patina-is-beliefs-plus-action]] — Local Patina is the belief factory. Edge apps are action layers that consume beliefs. The belief+action unit spans local and edge.
- [[safety-boundaries]] — Local-first means user data stays on their machine by default. Edge deployment is explicit opt-in.
- [[wit-is-contract-wasm-is-one-runtime]] — Same WIT contracts, different runtimes. Local uses wasmtime, edge uses Workers.

## Attacks

- "Abstract storage for portability" — Defeated: premature abstraction. Build the local implementation, design the edge interface. Don't abstract SQLite behind a trait when SQLite IS the right answer locally.

## Attacked-By

- "Some users will only have edge access" — If Patina is a product for teams, not everyone runs local. Counter: Mother federation handles this. Edge nodes connect through Mother to a local Patina that runs the full protocol. The edge doesn't need the full protocol.
- "Latency of local→edge sync" — Valid. If beliefs are stale by the time they reach the edge app, the action layer acts on outdated knowledge. Counter: Mother's continuous operation keeps sync tight. And beliefs change slowly compared to real-time data.

## Applied-In

- Current Patina: entirely local-first. git, SQLite, wasmtime, ONNX. This is the implementation.
- Chat agent scenario (session 20260304-120702): Cloudflare Worker consumes beliefs, generates conversation events, Mother syncs back to local.

## Revision Log

- 2026-03-04: Created — metrics computed by `patina scrape`
