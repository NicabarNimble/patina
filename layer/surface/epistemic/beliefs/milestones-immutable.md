---
type: belief
id: milestones-immutable
persona: architect
facets: [architecture, versioning, workflow]
confidence:
  score: 0.85
  signals:
    evidence: 0.90
    source_reliability: 0.85
    recency: 0.95
    survival: 0.50
    user_endorsement: 0.90
entrenchment: medium
status: active
extracted: 2026-01-26
revised: 2026-01-26
---

# milestones-immutable

Milestones are immutable goals. If a goal was wrong, create a new milestone - don't edit the old one. Like git commits, append rather than rewrite.

## Statement

Milestones are immutable goals that define the "what" (destination), while spec content evolves as the "how" (approach). If you discover a milestone goal was wrong, create a new milestone rather than editing the old one. This mirrors git's append-only model - history is preserved, not rewritten.

## Evidence

- session-20260126-074256: User articulated workflow: "milestones should not change... if it does change then we learned our milestone was wrong and we need to go back and make a new milestone" (weight: 0.95)
- [[session-20260126-074256]]: Parallel to git philosophy: commits are immutable, branches move forward (weight: 0.8)

## Verification

```verify type="sql" label="No edit/modify/delete milestone functions" expect="= 0"
SELECT COUNT(*) FROM function_facts WHERE name LIKE '%edit\_milestone%' ESCAPE '\' OR name LIKE '%modify\_milestone%' ESCAPE '\' OR name LIKE '%delete\_milestone%' ESCAPE '\'
```

```verify type="assay" label="Append-only: bump_milestone exists" expect=">= 1"
functions --pattern "bump_milestone"
```

## Supports

- milestones-in-specs: Strengthens the case for milestones in version control - immutability is a git property

## Attacks

<!-- Add beliefs this defeats -->

## Attacked-By

<!-- Add beliefs that challenge this -->

## Applied-In

- `patina version milestone` - completes milestones, never modifies past ones
- go-public SPEC.md milestone definitions

## Revision Log

- 2026-01-26: Created (confidence: 0.85)
