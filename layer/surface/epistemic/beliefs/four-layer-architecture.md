---
type: belief
id: four-layer-architecture
persona: architect
facets: [architecture, epistemics, core-principle]
entrenchment: high
status: active
endorsed: true
extracted: 2026-02-09
revised: 2026-02-09
---

# four-layer-architecture

Patina's architecture is four layers with beliefs at the center: beliefs (normative — what should be true, the product), assay (evidentiary — what is verifiably true, serving belief grounding), scry (cross-vocabulary memory — what is conceptually related, serving belief discovery), and mother (convergence engine — what keeps showing up across projects, serving belief evolution). Beliefs are the purpose; the other three layers are infrastructure that makes the belief system work.

## Statement

Patina's architecture is four layers with beliefs at the center: beliefs (normative — what should be true, the product), assay (evidentiary — what is verifiably true, serving belief grounding), scry (cross-vocabulary memory — what is conceptually related, serving belief discovery), and mother (convergence engine — what keeps showing up across projects, serving belief evolution). Beliefs are the purpose; the other three layers are infrastructure that makes the belief system work.

## Evidence

- [[session-20260209-160017]]: [[20260209-160017]] - Crystallized after Phase 5d post-mortem. The projection removal proved scry's job is cross-vocabulary bridging (pre-trained LM capability), not learned compression. Each layer has a distinct epistemic function that survives scale from single-project to multi-user. (weight: 1.0)

## Supports

- [[dependable-rust]] — each layer is a black-box module with a clear "do X" job
- [[unix-philosophy]] — one layer, one epistemic function, done well
- [[ablate-before-optimizing]] — scry's job is cross-vocabulary bridging, which means its core technology is a pre-trained LM, not a trained projection. Removing the projection aligned scry with its architectural role.
- [[corpus-composition-over-model]] — assay's evidentiary role means FTS5 coverage matters; scry's memory role means embedding quality matters. Different layers, different optimization targets.

## Attacks

- Monolithic retrieval systems that fuse semantic + factual + structural into one pipeline (the pre-split scry architecture with 5 oracles and 25-parameter tuning)

## Attacked-By

- "Four layers is over-engineered for a single project" (status: active, scope: "true at small scale — a single project with 84 beliefs fits in an LLM context window. The architecture earns its complexity at multi-project/multi-user scale via mother.")
- [[patina-is-beliefs-plus-action]] — The beliefs+action framing revealed that assay, scry, and mother are not peer layers with beliefs — they serve beliefs. This prompted the revision from co-equal layers to hierarchical layers with beliefs at the center.

## Applied-In

- [[semantic-structural-split]] — Phase 1 separated assay (evidentiary) from scry (cross-vocabulary memory). Phase 5d removed the projection to align scry with raw pre-trained LM capability.
- Belief verification queries ground beliefs in assay's evidentiary layer (SQL checks against codebase structure)
- `patina context --topic` fuses assay (factual matches) + scry (semantic matches) + beliefs (normative layer) — three of the four layers in one consumer

## Revision Log

- 2026-02-09: Created — metrics computed by `patina scrape`
- 2026-03-04: Revised — from co-equal layers to hierarchical. Beliefs are the center/product; assay, scry, mother are infrastructure serving beliefs. Prompted by patina-is-beliefs-plus-action.
