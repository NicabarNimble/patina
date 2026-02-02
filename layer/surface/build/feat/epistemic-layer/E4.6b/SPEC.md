---
type: feat
id: epistemic-e4.6b
status: design
created: 2026-02-02
updated: 2026-02-02
sessions:
  origin: 20260202-134539
related:
  - layer/surface/build/feat/epistemic-layer/SPEC.md
  - layer/surface/build/feat/epistemic-layer/E4.6a-fix/SPEC.md
---

# feat: Belief-to-Belief Semantic Relationships (E4.6b)

> Discover which beliefs cluster, conflict, or stand alone — from embeddings and wikilinks.

**Parent:** [epistemic-layer](../SPEC.md) phase E4.6
**Prerequisite:** E4.6a (same infrastructure, different filter). E4.6a-fix not required —
belief↔belief cosine works (same distribution).

---

## Problem

Belief→belief is the same operation as belief→commit with a different ID range filter.
This phase adds structure on top of raw similarity. Currently, belief relationships
(supports/attacks/evidence) exist as wikilinks in markdown but nothing computes over them.

---

## Scope

1. **Belief-to-belief semantic similarity** — kNN within the belief ID range. Surface clusters
   in audit: "these 3 beliefs are semantically close." Detect isolated beliefs with no semantic
   neighbors (potential gaps or orphans).

2. **Relationship type awareness** — Parse supports/attacks/evidence wikilinks with directionality.
   Currently all wikilinks add +1 to `cited_by_beliefs`. Instead: track support edges, attack
   edges, and evidence edges separately. Store in a `belief_edges` table (from_id, to_id,
   edge_type, section_source).

3. **Semantic conflict detection** — When two beliefs are close in embedding space but have
   opposing verification results (one passes, one contested), or are connected by attack edges,
   surface this as a warning in audit. This is *detection*, not resolution (E5 handles resolution).

4. **Belief neighborhood in scry** — When scry returns a belief, also return its closest semantic
   neighbors and its support/attack edges. Give the LLM context about where a belief sits in the
   network, not just the belief in isolation.

---

## Build Steps

- [ ] 1. Compute belief-to-belief cosine similarity from usearch embeddings (reuse E4.6a infra)
- [ ] 2. Create `belief_edges` table — parse supports/attacks/evidence with edge type
- [ ] 3. Update `cross_reference_beliefs()` to populate `belief_edges` instead of flat count
- [ ] 4. Add semantic clustering to `patina belief audit` — group similar beliefs
- [ ] 5. Add conflict detection — semantic proximity + opposing verification status
- [ ] 6. Update scry enrichment to show belief neighborhood (nearest beliefs + edges)
- [ ] 7. Surface orphaned beliefs (no edges, no semantic neighbors)

---

## Exit Criteria

- [ ] `belief_edges` table populated with typed edges (support, attack, evidence)
- [ ] Audit shows belief clusters and semantic conflict warnings
- [ ] Scry belief results include neighborhood context
- [ ] Orphaned/isolated beliefs identified

---

## Status Log

| Date | Status | Note |
|------|--------|------|
| 2026-02-02 | design | Extracted from epistemic-layer monolith during spec decomposition |
