---
type: belief
id: spec-drives-tooling
persona: architect
facets: [architecture, versioning, specs]
confidence:
  score: 0.85
  signals:
    evidence: 0.90
    source_reliability: 0.85
    recency: 0.80
    survival: 0.50
    user_endorsement: 0.50
entrenchment: medium
status: active
extracted: 2026-01-31
revised: 2026-01-31
---

# spec-drives-tooling

Spec tables drive version tooling — change the data, not the code

## Statement

Spec tables drive version tooling — change the data, not the code

## Evidence

- session-20260131-150141: Shifting milestones from PATCH to MINOR required zero version command logic changes — only the spec milestone table was updated. patina version milestone reads from spec index, so the tooling followed automatically. (weight: 0.9)

## Verification

```verify type="sql" label="Version command has 3+ spec-reading functions" expect=">= 3"
SELECT COUNT(*) FROM function_facts WHERE file LIKE '%version/internal%' AND name LIKE '%spec%'
```

```verify type="sql" label="Version command has milestone functions from specs" expect=">= 3"
SELECT COUNT(*) FROM function_facts WHERE file LIKE '%version/%' AND name LIKE '%milestone%'
```

## Supports

- spec-is-contract: Specs as source of truth means tooling reads from them, not the reverse

## Attacks

<!-- Add beliefs this defeats -->

## Attacked-By

<!-- Add beliefs that challenge this -->

## Applied-In

- `patina version milestone` reads version from spec milestone table via SQLite index
- Semver alignment (0.9.x PATCH → 0.10.0+ MINOR) required only spec table edits, zero Rust changes

## Revision Log

- 2026-01-31: Created (confidence: 0.85)
