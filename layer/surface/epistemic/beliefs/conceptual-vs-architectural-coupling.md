---
type: belief
id: conceptual-vs-architectural-coupling
persona: architect
facets: [architecture, modularity, rust]
confidence:
  score: 0.85
entrenchment: medium
status: active
extracted: 2026-01-26
revised: 2026-01-26
---

# conceptual-vs-architectural-coupling

Related concepts don't imply shared modules - conceptual grouping (both about 'secrets') is not the same as architectural coupling (shared types, state, or dependencies). The test: can I delete one without touching the other?

## Statement

Related concepts don't imply shared modules - conceptual grouping (both about 'secrets') is not the same as architectural coupling (shared types, state, or dependencies). The test: can I delete one without touching the other?

## Evidence

- session-20260126-134036: vault and scanner are both 'about secrets' but share no types, state, or dependencies - keeping them separate honors independence (weight: 0.9)

## Verification

```verify type="sql" label="Zero cross-command-module imports" expect="= 0"
SELECT COUNT(*) FROM import_facts WHERE file LIKE '%/commands/%' AND import_path LIKE '%commands/%' AND file <> import_path
```

## Supports

<!-- Add beliefs this supports -->

## Attacks

<!-- Add beliefs this defeats -->

## Attacked-By

<!-- Add beliefs that challenge this -->

## Applied-In

<!-- Add concrete applications -->

## Revision Log

- 2026-01-26: Created (confidence: 0.85)
