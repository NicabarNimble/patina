---
type: belief
id: ablate-before-optimizing
persona: architect
facets: [evaluation, architecture, ml-engineering]
entrenchment: medium
status: active
endorsed: true
extracted: 2026-02-09
revised: 2026-02-09
---

# ablate-before-optimizing

When building a pipeline A→B→C, test A→C (skip B) before investing in tuning B — prove each component adds value through ablation, not assumption.

## Statement

When building a pipeline A→B→C, test A→C (skip B) before investing in tuning B — prove each component adds value through ablation, not assumption.

## Evidence

- [[session-20260209-160017]]: [[20260209-160017]] - Phase 5a-5d spent 1.5 days tuning a projection MLP (gradient fix, belief pairs, determinism) that destroyed E5's pre-trained structure. One 60-second raw E5 test showed P@10 52.5% vs projected 9.2%. The component being optimized was net-negative. (weight: 1.0)

## Supports

- [[measure-first]] — ablation IS measurement; this belief makes measure-first concrete for pipelines
- [[andrew-ng-over-shoulder]] — "measure the simplest thing first" — raw E5 was the simplest thing
- [[error-analysis-over-architecture]] — ablation is a form of error analysis: isolate which component causes the problem

## Attacks

- Cargo-culted ML wisdom that "you always need a projection head on top of embeddings"

## Attacked-By

- "Ablation takes time that could be spent building" — defeated: the Phase 5a-5d saga proves skipping ablation costs MORE time (1.5 days vs 60 seconds)

## Applied-In

- [[semantic-structural-split]] Phase 5d: `patina eval --scry-raw` ablated the projection MLP, revealing raw E5 (768-dim, P@10 52.5%) dramatically outperforms projected (256-dim, P@10 9.2%). Led to deleting the projection for knowledge/sessions domains. Commit [[e603121b]].

## Revision Log

- 2026-02-09: Created — metrics computed by `patina scrape`
