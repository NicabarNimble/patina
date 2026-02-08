---
type: belief
id: beliefs-are-entities-not-documents
persona: architect
facets: [belief-system, semantic-layer, system-design]
entrenchment: medium
status: active
endorsed: true
extracted: 2026-02-08
revised: 2026-02-08
---

# beliefs-are-entities-not-documents

Beliefs and specs are entities in a semantic+factual system that the system helps connect — not documents that humans wire together manually. The system should discover relevant beliefs, surface evidence, and flag contradictions, not just store what the author remembers to cite.

## Statement

Beliefs and specs are entities in a semantic+factual system that the system helps connect — not documents that humans wire together manually. The system should discover relevant beliefs, surface evidence, and flag contradictions, not just store what the author remembers to cite.

## Evidence

- [[session-20260208-070221]]: Analysis of 6 sessions of retrieval tuning revealed that belief↔spec↔code connections were entirely manual. Beliefs had MRR 0.241 in the mixed pipeline — found but ranked poorly. The semantic-structural split (scry for meaning, assay for facts) enables two clean interaction paths: assay grounds beliefs in factual evidence (code, commits, specs that reference them), scry discovers beliefs by conceptual relevance (finding [[never-tune-on-eval]] when you're writing a spec about parameter optimization, even without keyword overlap). The belief→reality→spec cycle becomes a system-supported loop, not a manually maintained line. (weight: 0.90)

## Supports

- [[dependable-rust]] — black-box modules with stable interfaces enable clean belief↔system interaction paths
- [[unix-philosophy]] — separate tools for separate concerns (assay for facts, scry for meaning) serving the belief system

## Attacks

<!-- none yet -->

## Attacked-By

<!-- none yet -->

## Applied-In

- [[semantic-structural-split]] spec — the split architecture is designed to make this belief operational
- Phase 5 (domain discovery) — each new semantic domain adds a dimension of belief↔reality connection

## Revision Log

- 2026-02-08: Created — distilled from session discussion about how beliefs/specs should interact with the semantic-structural split
