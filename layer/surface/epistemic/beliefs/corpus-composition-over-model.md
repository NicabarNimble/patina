---
type: belief
id: corpus-composition-over-model
persona: architect
facets: [retrieval, embeddings, architecture]
entrenchment: medium
status: active
endorsed: true
extracted: 2026-02-08
revised: 2026-02-08
---

# corpus-composition-over-model

Corpus composition matters more than model choice or training signal — removing noise from the index produces larger retrieval gains than changing the embedding model or training strategy.

## Statement

Corpus composition matters more than model choice or training signal — removing noise from the index produces larger retrieval gains than changing the embedding model or training strategy.

## Evidence

- [[session-20260208-113613]]: Phase 2 knowledge domain: removing 92% session event pollution (35K → 1,903 items) improved eval from 0%/0%/0.000 to 4.3%/5.6%/0.107 with identical E5 model and commit-based training (weight: 0.95)
- [[session-20260208-103844]]: Phase 1 eval baseline documented 0% across all metrics — the semantic index was polluted with 27K session events (88%), trained on session co-occurrence not semantic meaning (weight: 0.8)

## Supports

- [[error-analysis-over-architecture]]: Analyzing the corpus composition (failure case) before adding model complexity
- [[andrew-ng-over-shoulder]]: Honest eval before/after proved the composition was the bottleneck, not the model

## Attacks

<!-- Add beliefs this defeats -->

## Attacked-By

<!-- Add beliefs that challenge this -->

## Applied-In

- [[semantic-structural-split]] Phase 2: `query_knowledge_corpus()` in `src/commands/oxidize/mod.rs` — replaced 6-source 35K-item corpus with 3-source 1,903-item knowledge domain
- `src/commands/oxidize/mod.rs:query_knowledge_corpus()` — the concrete implementation that proves the belief

## Revision Log

- 2026-02-08: Created — metrics computed by `patina scrape`
