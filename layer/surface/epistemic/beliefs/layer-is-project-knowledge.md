---
type: belief
id: layer-is-project-knowledge
persona: architect
facets: [architecture, organization]
confidence:
  score: 0.9
  signals:
    evidence: 0.95
    source_reliability: 0.9
    recency: 0.80
    survival: 0.50
    user_endorsement: 0.50
entrenchment: medium
status: active
extracted: 2026-01-22
revised: 2026-01-22
---

# layer-is-project-knowledge

layer/ is project-specific knowledge that accumulates for ANY project via patina init; resources/ is patina's own development artifacts. When adding content, ask: would another project have this? Yes → layer/, No → resources/.

## Statement

layer/ is project-specific knowledge that accumulates for ANY project via patina init; resources/ is patina's own development artifacts. When adding content, ask: would another project have this? Yes → layer/, No → resources/.

## Evidence

- session-20260122-102703: Discussion about /eval/ querysets led to clarification - user asked 'will we want to do this on each project?' triggering the distinction (weight: 0.95)

## Verification

```verify type="sql" label="Layer has core patterns tracked in git" expect=">= 1"
SELECT COUNT(*) FROM git_tracked_files WHERE file_path LIKE 'layer/core/%'
```

```verify type="sql" label="Layer has surface patterns tracked in git" expect=">= 10"
SELECT COUNT(*) FROM git_tracked_files WHERE file_path LIKE 'layer/surface/%'
```

```verify type="sql" label="Layer has sessions tracked in git" expect=">= 10"
SELECT COUNT(*) FROM git_tracked_files WHERE file_path LIKE 'layer/sessions/%'
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

- 2026-01-22: Created (confidence: 0.9)
