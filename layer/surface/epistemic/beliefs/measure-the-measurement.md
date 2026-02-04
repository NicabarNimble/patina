---
type: belief
id: measure-the-measurement
persona: architect
facets: [measurement, tooling, epistemic]
entrenchment: medium
status: active
endorsed: true
extracted: 2026-01-31
revised: 2026-01-31
---

# measure-the-measurement

When computed metrics contradict known reality, the measurement tool is wrong before the data is. Fix the instrument, not the observation.

## Statement

When computed metrics contradict known reality, the measurement tool is wrong before the data is. Fix the instrument, not the observation.

## Evidence

- [[session-20260131-160252]] - belief evidence verification showed 39% (31/80) verified, but beliefs were well-grounded. Fixing the verifier to recognize bare session-ID references (not just [[wikilinks]]) raised verification to 79% (63/80). The data was real, the tooling was too narrow. (weight: 0.95)

## Supports

- [[measure-first]] - measuring the measurement is a recursive application of measure-first
- [[error-analysis-over-architecture]] - when metrics look wrong, analyze the error before changing the system

## Attacks

<!-- Add beliefs this defeats -->

## Attacked-By

<!-- Add beliefs that challenge this -->

## Applied-In

- `src/commands/scrape/beliefs/mod.rs`: `verify_evidence_section()` rewritten to handle bare session-ID references alongside [[wikilinks]], fixing false-negative verification
- E4 confidence redesign: fake LLM-fabricated 0.88 scores replaced with computed use/truth metrics from real cross-reference data

## Revision Log

- 2026-01-31: Created — metrics computed by `patina scrape`
