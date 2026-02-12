---
type: belief
id: no-deferrals-without-blockers
persona: architect
facets: [process, governance, specs]
entrenchment: medium
status: active
endorsed: true
extracted: 2026-02-12
revised: 2026-02-12
---

# no-deferrals-without-blockers

If a fix, feature, or documentation task isn't blocked by another task, it goes in the current spec — deferring unblocked work is how gaps accumulate and rot.

## Statement

If a fix, feature, or documentation task isn't blocked by another task, it goes in the current spec — deferring unblocked work is how gaps accumulate and rot.

## Evidence

- [[session-20260212-093831]]: [[session-20260212-093831]] - 5 discoveries initially marked deferred (doc later, Phase 2 concern, design before community plugins); none were actually blocked; all 5 folded into [[plugin-system-final-audit-fixes]] fix spec for immediate build (weight: 0.95)

## Supports

- [[spec-driven-design]] — specs authorize action; unblocked tasks belong in specs, not session notes
- [[work-triages-specs]] — let the build determine what matters; if work is ready, it ships

## Attacks

- "Phase 2 concern" as a deferral pattern — unless blocked by Phase 1 incomplete work, Phase 2 labels are procrastination

## Attacked-By

- Over-engineering risk — doing everything now could add scope. Counter: the belief says "not blocked AND not over-engineering." If it's truly premature, it IS blocked (by missing requirements).

## Applied-In

- [[plugin-system-final-audit-fixes]] — 5 discoveries initially deferred, all folded into fix spec as F0-F5 after applying this principle
- F4 (toy capability gating) — was "design before community plugins" (deferred); infrastructure already existed, built immediately
- F5 (design doc comments) — was "document later"; 3 comments, zero reason to wait

## Revision Log

- 2026-02-12: Created — metrics computed by `patina scrape`
