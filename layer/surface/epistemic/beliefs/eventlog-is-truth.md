---
type: belief
id: eventlog-is-truth
persona: architect
facets: [data-architecture, persistence]
confidence:
  score: 0.92
  signals:
    evidence: 0.95
    source_reliability: 0.90
    recency: 0.90
    survival: 0.95
    user_endorsement: 0.85
entrenchment: very-high
status: active
extracted: 2026-01-14
revised: 2026-01-16
---

# eventlog-is-truth

Eventlog is source of truth; tables are materialized views.

## Statement

The append-only eventlog is the canonical source of truth. All tables (commits, functions, patterns) are derived materialized views that can be rebuilt from the eventlog. This follows Pat Helland's principle.

## Evidence

- [[session-20260115-121358]] - "L2 eventlog captures surface decisions as source of truth" (weight: 0.95)
- [[session-20260114-114833]] - "Git IS the eventlog for git data" (weight: 0.90)
- [[spec-ref-repo-storage]] - Eventlog for expensive/original knowledge (weight: 0.85)
- [[helland-paper]] - Academic grounding (weight: 0.80)

## Verification

```verify type="sql" label="insert_event callers" expect=">= 20"
SELECT COUNT(*) FROM call_graph WHERE callee LIKE '%insert_event%'
```

```verify type="assay" label="insert_event across files" expect=">= 5"
callers --pattern "insert_event" | count(distinct file)
```

## Supports

- [[rebuild-from-source]]
- [[deterministic-first]]

## Attacks

- [[mutable-state-simpler]] (status: defeated, reason: loses history)

## Attacked-By

- [[storage-overhead]] (status: scoped, scope: "ref repos use lean storage")

## Implications

- L1 eventlog: git/code/sessions → patina.db (deterministic, rebuildable)
- L2 eventlog: surface decisions → layer/surface/ (non-deterministic boundary)
- Non-deterministic boundary = new source of truth (Helland's principle)

## Applied-In

- patina.db architecture
- Lean storage for ref repos
- L2 eventlog design

## Revision Log

- 2026-01-14: Extracted from ref-repo-storage work (confidence: 0.80)
- 2026-01-15: L2 eventlog insight added (confidence: 0.80 → 0.92)
