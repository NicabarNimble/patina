---
type: belief
id: frontmatter-id-is-identity
persona: architect
facets: [rust, indexing, data-model]
confidence:
  score: 0.92
  signals:
    evidence: 0.97
    source_reliability: 0.92
    recency: 0.80
    survival: 0.50
    user_endorsement: 0.50
entrenchment: medium
status: active
extracted: 2026-01-29
revised: 2026-01-29
---

# frontmatter-id-is-identity

The frontmatter ID field is the canonical identity for indexed content, not the filename

## Statement

The frontmatter ID field is the canonical identity for indexed content, not the filename

## Evidence

- session-20260129-084757: Bug discovery — pruning used file stems but DB uses frontmatter IDs, causing specs like SPEC.md with id:v1-release to be incorrectly pruned (weight: 0.95)

## Verification

```verify type="assay" label="Frontmatter callers across multiple files" expect=">= 3"
callers --pattern "frontmatter" | count(distinct file)
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

- 2026-01-29: Created (confidence: 0.92)
