---
type: belief
id: cli-unifies-code-separates
persona: architect
facets: [architecture, cli, ux]
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
extracted: 2026-01-26
revised: 2026-01-26
---

# cli-unifies-code-separates

CLI can unify independent modules under one namespace without implying architectural coupling - the CLI is a UX layer that serves user mental models, while the code remains honest about actual dependencies.

## Statement

CLI can unify independent modules under one namespace without implying architectural coupling - the CLI is a UX layer that serves user mental models, while the code remains honest about actual dependencies.

## Evidence

- session-20260126-134036: patina secrets presents unified UX for vault+scanner, but src/secrets/ and src/scanner/ are independent modules with no shared code (weight: 0.9)

## Verification

```verify type="sql" label="Zero cross-command-module imports" expect="= 0"
SELECT COUNT(*) FROM import_facts WHERE file LIKE '%/commands/%' AND import_path LIKE '%commands/%' AND file <> import_path
```

```verify type="assay" label="CLI dispatches to multiple command modules" expect=">= 10"
importers --pattern "commands" | count(distinct file)
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
