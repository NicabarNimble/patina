---
type: rule
id: implement-after-measurement
persona: architect
rule_type: synthesized
confidence: 0.82
derived_from: [measure-first, spec-first, dont-build-what-exists]
status: active
extracted: 2026-01-16
---

# rule: implement-after-measurement

## Conditions

- [[measure-first]] (confidence > 0.7)
- [[spec-first]] (confidence > 0.7)

## Conclusion

Before implementing new infrastructure:
1. Write a spec describing the problem
2. Measure the current baseline
3. Prove the gap exists with data
4. Only then implement

## Rationale

Combining spec-first (design before code) with measure-first (prove with data) ensures we don't build unnecessary infrastructure. The mother-graph implementation followed this pattern: G0 measured, proved 0% repo recall, then G1-G2 built the solution.

## Exceptions

- [[trivial-fix]] - Bug fixes under 20 lines don't need measurement
- [[security-patch]] - Urgent security issues proceed immediately
- [[user-requested]] - Explicit user request overrides (with acknowledgment)

## Applied-In

- [[spec/mothership-graph]] - G0 phase before G1
- [[spec-quality-gates]] - Measured MRR regression before fixing
- [[spec-ref-repo-storage]] - Measured storage before optimization

## Evidence

- [[session-20260105]] - Graph routing followed this pattern
- [[spec-review-q4-2025]] - Pattern observed across 39 specs

## Revision Log

- 2026-01-16: Synthesized from belief combination
