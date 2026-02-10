---
type: belief
id: score-merge-needs-quotas
persona: architect
facets: [retrieval, architecture, fusion]
entrenchment: medium
status: active
endorsed: true
extracted: 2026-02-09
revised: 2026-02-09
---

# score-merge-needs-quotas

When fusing results across semantic domains of different sizes via score-merge, large domains dominate small ones — per-domain minimum quotas preserve retrieval quality for minority domains.

## Statement

When fusing results across semantic domains of different sizes via score-merge, large domains dominate small ones — per-domain minimum quotas preserve retrieval quality for minority domains.

## Evidence

- [[session-20260209-181228]]: [[semantic-structural-split]] Phase 5e — exact search reveals 4 queries where knowledge results (615 items) are pushed out of top-10 by session results (2,749 items), dropping P@10 from 52.5% to 41.7% (weight: 0.95)

## Supports

- [[corpus-composition-over-model]] — domain size imbalance is a form of corpus composition problem; the session domain's 4.5x size advantage drowns knowledge results
- [[four-layer-architecture]] — each epistemic layer (beliefs, assay, scry, mother) should contribute proportionally, not be drowned by volume

## Attacks

- The SPEC's own score-merge rationale: "same model → score-merge; different models → RRF." Score-merge assumes scores are comparable, but doesn't account for domain size asymmetry causing positional dominance

## Attacked-By

- "Score-merge is correct for same-metric domains" — cosine scores ARE comparable across domains. The issue isn't score comparability, it's result set composition. A domain with 4.5x more items has 4.5x more chances to place high-scoring results. Quotas trade some ranking purity for diversity.

## Applied-In

- `src/retrieval/engine.rs` — `query_local()` currently does unrestricted score-merge across knowledge + sessions domains. Fix: per-domain minimum quota before final merge.

## Revision Log

- 2026-02-09: Created — metrics computed by `patina scrape`
