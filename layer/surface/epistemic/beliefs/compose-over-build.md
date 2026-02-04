---
type: belief
id: compose-over-build
persona: architect
facets: [architecture, unix-philosophy, maintainability]
confidence:
  score: 0.85
entrenchment: medium
status: active
extracted: 2026-01-21
revised: 2026-01-21
---

# compose-over-build

When adding capability, prefer composing existing tools over building parallel systems. Composed solutions improve automatically when components improve; parallel systems require independent maintenance.

## Statement

When adding capability, prefer composing existing tools over building parallel systems. Composed solutions improve automatically when components improve; parallel systems require independent maintenance.

## Evidence

- session-20260121-071314: Discovered while designing belief validation - using scry/git/grep for validation means improvements cascade automatically, unlike the abandoned Prolog system which required separate maintenance (weight: 0.95)

## Verification

```verify type="sql" label="Scry/assay/oxidize reused across 10+ command files" expect=">= 10"
SELECT COUNT(DISTINCT file) FROM call_graph WHERE (callee LIKE '%scry%' OR callee LIKE '%assay%' OR callee LIKE '%oxidize%') AND file LIKE '%commands/%'
```

## Supports

- [[dont-build-what-exists]] - composition reuses what exists
- [[measure-first]] - composed tools already have measurement infrastructure

## Attacks

- [[parallel-systems-for-isolation]] (status: scoped, reason: isolation has value for truly independent concerns, but not for features that should improve together)

## Attacked-By

- [[specialized-tools-for-performance]] (status: active, confidence: 0.4, scope: "when composition overhead matters")
- [[separation-of-concerns]] (status: active, confidence: 0.3, scope: "when independent evolution is needed")

## Applied-In

- **Belief validation system**: Uses scry for semantic verification, git for link verification, grep for content matching - no separate validation engine needed
- **Prolog system rejection**: The Nov 2025 neuro-symbolic system (src/reasoning/, src/storage/) built parallel infrastructure that would require separate maintenance; abandoned in favor of composition
- **QueryEngine oracles**: Semantic, lexical, temporal oracles compose existing databases rather than building separate indices

## Revision Log

- 2026-01-21: Created (confidence: 0.85)
