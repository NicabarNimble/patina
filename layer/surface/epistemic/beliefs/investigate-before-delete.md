---
type: belief
id: investigate-before-delete
persona: architect
facets: [process, maintenance, knowledge-management]
confidence:
  score: 0.90
  signals:
    evidence: 0.95
    source_reliability: 0.90
    recency: 0.80
    survival: 0.50
    user_endorsement: 0.50
entrenchment: medium
status: active
extracted: 2026-01-27
revised: 2026-01-27
---

# investigate-before-delete

Always trace an artifact's lineage before removing it. Use tools (scry, assay, grep, session history) to understand why something exists, what replaced it, and whether anything still depends on it. The investigation itself documents the rationale for deletion.

## Statement

Always trace an artifact's lineage before removing it. Use tools (scry, assay, grep, session history) to understand why something exists, what replaced it, and whether anything still depends on it. The investigation itself documents the rationale for deletion.

## Evidence

- session-20260126-211444: Traced layer/lab, layer/personas, docker-compose, Dockerfile, and tests/ subdirectories through scry, assay, grep, and session history before removal. Discovered rqlite lasted one day, personas evolved into patina persona command, lab queries became patina eval/bench. (weight: 0.95)

## Supports

<!-- Add beliefs this supports -->

## Attacks

<!-- Add beliefs this defeats -->

## Attacked-By

<!-- Add beliefs that challenge this -->

## Applied-In

<!-- Add concrete applications -->

## Revision Log

- 2026-01-27: Created (confidence: 0.90)
