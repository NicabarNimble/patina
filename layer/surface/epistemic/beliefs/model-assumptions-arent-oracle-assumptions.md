---
type: belief
id: model-assumptions-arent-oracle-assumptions
persona: architect
facets: [architecture, models, onnx, retrieval]
entrenchment: medium
status: active
endorsed: true
extracted: 2026-02-07
revised: 2026-02-07
---

# model-assumptions-arent-oracle-assumptions

Oracle performance depends on the underlying model — semantic contributing 0% may be E5-base-v2's limitation, not proof that semantic search is useless. Don't bake model-specific observations into architectural decisions.

## Statement

Oracle performance depends on the underlying model — semantic contributing 0% may be E5-base-v2's limitation, not proof that semantic search is useless. Don't bake model-specific observations into architectural decisions.

## Evidence

- [[session-20260207-094828]]: [[session-20260207-094828]] - SemanticOracle returned 0% P@K on all 25 NL queries using E5-base-v2 via ONNX. Temptation was to suppress semantic oracle via hardcoded weights, but ONNX flexibility means models can be swapped — a different model could make semantic the dominant oracle. (weight: 0.9)

## Supports

- [[never-tune-on-eval]] — model-dependent observations shouldn't drive weight tuning
- [[error-analysis-over-architecture]]

## Attacks

<!-- none yet -->

## Attacked-By

<!-- none yet -->

## Applied-In

- [[retrieval-tuning]] spec — observations section explicitly notes E5-base-v2 dependency
- `src/retrieval/intent.rs` — weights kept uniform rather than suppressing semantic

## Revision Log

- 2026-02-07: Created — metrics computed by `patina scrape`
