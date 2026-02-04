---
type: belief
id: archive-completed-work
persona: architect
facets: []
confidence:
  score: 0.88
entrenchment: medium
status: active
extracted: 2026-01-22
revised: 2026-01-22
---

# archive-completed-work

Archive completed work promptly. Completed specs cluttering the active directory create noise and hide what's actually in progress.

## Statement

Archive completed work promptly. Completed specs cluttering the active directory create noise and hide what's actually in progress.

## Evidence

- session-20260122-083510: Cleaned 49 specs down to 15 by archiving 17 completed specs that had git tags but files never deleted

## Verification

```verify type="sql" label="No completed specs in active directories" expect="= 0"
SELECT COUNT(*) FROM patterns WHERE status = 'complete' AND (file_path LIKE '%build/feat/%' OR file_path LIKE '%build/refactor/%' OR file_path LIKE '%build/fix/%')
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

- 2026-01-22: Created (confidence: 0.88)
