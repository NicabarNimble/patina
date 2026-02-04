---
type: belief
id: verification-is-annotation-not-retrieval
persona: architect
facets: [retrieval, architecture, oracles]
entrenchment: medium
status: active
endorsed: true
extracted: 2026-02-04
revised: 2026-02-04
---

# verification-is-annotation-not-retrieval

Quality signals about results (evidence counts, freshness, verification status) belong as post-fusion annotations, not as separate retrieval oracles — oracles retrieve, annotations qualify.

## Statement

Quality signals about results (evidence counts, freshness, verification status) belong as post-fusion annotations, not as separate retrieval oracles — oracles retrieve, annotations qualify.

## Evidence

- [[session-20260204-125850]]: EvidenceOracle discussion — RRF dedup by doc_id means only one content string survives merge; verification metadata from a separate oracle would be lost (weight: 0.9)
- `src/retrieval/engine.rs:429-488`: `populate_annotations()` already implements this pattern for structural signals (importer_count, activity_level, is_entry_point) as post-fusion enrichment (weight: 0.8)
- [[session-20260204-125850]]: BeliefOracle content string includes evidence metrics inline — verification signal baked into retrieval content, not a separate channel (weight: 0.6)

## Supports

- [[dependable-rust]] — black-box modules with single responsibility: oracles do retrieval, annotations do qualification

## Attacks

<!-- No beliefs defeated -->

## Attacked-By

<!-- Potential challenge: a dedicated VerificationOracle could use doc_id matching to boost rather than return new docs — but this collapses to a weight modifier, not a retrieval channel -->

## Applied-In

- `src/retrieval/engine.rs`: `populate_annotations()` enriches FusedResult with StructuralAnnotations after fusion — structural signals are annotation, not retrieval
- D1 BeliefOracle: evidence metrics (evidence_count, evidence_verified, applied_in) embedded in content string rather than as a separate verification oracle

## Revision Log

- 2026-02-04: Created — metrics computed by `patina scrape`
