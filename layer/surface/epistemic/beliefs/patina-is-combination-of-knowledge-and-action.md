---
type: belief
id: patina-is-combination-of-knowledge-and-action
persona: architect
facets: [architecture, product, local-first, p2p, identity]
entrenchment: medium
status: active
endorsed: true
extracted: 2026-03-12
revised: 2026-04-02
---

# patina-is-combination-of-knowledge-and-action

Patina is the combination of knowledge and action: Mother governs authority and continuity, children orchestrate work, toys bound capability, and apps/interfaces expose user-facing workflows over a decentralized, local-first network.

## Statement

Patina is the combination of knowledge and action: Mother governs authority and continuity, children orchestrate work, toys bound capability, and apps/interfaces expose user-facing workflows over a decentralized, local-first network.

## Evidence

- [[session-20260312-160150]]: User corrected the framing from either/or to both/and: Patina must be both knowledge layer and actionable layer, "like a child and their toy". (weight: 0.97)
- [[layer/surface/build/refactor/ducklake-retirement/SPEC.md]]: Retirement of legacy ducklake coupling reinforced this belief's boundary split: Mother authority + child orchestration + toy-bounded capability. (weight: 0.9)

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

- `src/mother/broker/mod.rs` — Mother-owned source broker performs authoritative routing while keeping child/runtime boundaries explicit.
- `src/child/internal/mod.rs` and `src/child/internal/knowledge_child.rs` — capability grants are resolved by Mother and enforced at child host-call boundaries.
- `src/commands/mother/mod.rs` — operator-facing Mother commands expose orchestration without collapsing authority boundaries.

## Revision Log

- 2026-03-12: Created — metrics computed by `patina scrape`
- 2026-04-02: Revised — replaced stale ducklake cutover/code anchors with current broker + child boundary anchors.
