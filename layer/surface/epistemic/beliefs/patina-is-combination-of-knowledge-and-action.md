---
type: belief
id: patina-is-combination-of-knowledge-and-action
persona: architect
facets: [architecture, product, local-first, p2p, identity]
entrenchment: medium
status: active
endorsed: true
extracted: 2026-03-12
revised: 2026-03-12
---

# patina-is-combination-of-knowledge-and-action

Patina is the combination of knowledge and action: Mother governs authority and continuity, children orchestrate work, toys bound capability, and apps/interfaces expose user-facing workflows over a decentralized, local-first network.

## Statement

Patina is the combination of knowledge and action: Mother governs authority and continuity, children orchestrate work, toys bound capability, and apps/interfaces expose user-facing workflows over a decentralized, local-first network.

## Evidence

- [[session-20260312-160150]]: User corrected the framing from either/or to both/and: Patina must be both knowledge layer and actionable layer, "like a child and their toy". (weight: 0.97)
- [[layer/surface/build/refactor/ducklake-knowledge-child-cutover/SPEC.md]]: Cutover requires Mother-authoritative enqueue/bounded-wait runtime, explicit child orchestration, and grant-scoped capability paths, matching this belief's control/agency/boundary split. (weight: 0.85)

## Supports

- [[patina-is-knowledge-layer]] — Actionable app-layer framing builds on Patina as substrate, clarifying where user-facing workflow execution sits.
- [[local-first-edge-deployable]] — Keeps local-first substrate as primary while allowing app-layer consumers/UI at the edge.
- [[children-have-agency-toys-are-capabilities]] — Explicitly adopts child orchestration agency with toy-bounded authority.

## Attacks

- "Patina should be primarily an LLM agent framework" — Defeated: agents are optional behavior inside the actionable layer; deterministic workflow + bounded capability remains primary.
- "Apps should call host capabilities directly" — Defeated: app interfaces route through Mother/children; toys stay runtime-internal authority objects.

## Attacked-By

- "This layering feels like a Frankenstein stack" — Valid concern. Counter: stable taxonomy (apps/interfaces/children/toys) and spec-bound seams reduce conceptual drift and tighten scope.

## Applied-In

- `src/broker/mod.rs` — `Destination::Lake` route models actionable-layer orchestration by enqueueing DuckLake work through Mother state/checkpoint semantics.
- `src/plugin/internal/knowledge_child.rs` — connector sync and OAuth resolution remain host-authoritative while child logic stays orchestration-focused.
- `src/commands/mother/mod.rs` — parity command exposes operator-facing interface while preserving runtime authority boundaries.

## Revision Log

- 2026-03-12: Created — metrics computed by `patina scrape`
