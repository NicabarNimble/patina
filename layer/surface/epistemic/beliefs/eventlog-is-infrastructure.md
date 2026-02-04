---
type: belief
id: eventlog-is-infrastructure
persona: architect
facets: [architecture, rust, unix-philosophy]
confidence:
  score: 0.80
entrenchment: medium
status: active
extracted: 2026-01-29
revised: 2026-01-29
---

# eventlog-is-infrastructure

The eventlog is shared infrastructure, not a scrape implementation detail. When multiple commands write events, the eventlog module must live outside any single command.

## Statement

The eventlog is shared infrastructure, not a scrape implementation detail. When multiple commands write events, the eventlog module must live outside any single command.

## Evidence

- session-20260129-123019: Deep dive into session system revealed scrape/database.rs owns insert_event() but session commands, scry feedback, and git scraper all need to write events. Cross-command import is architectural debt. (weight: 0.9)

## Verification

```verify type="sql" label="insert_event called from 3+ command domains" expect=">= 3"
SELECT COUNT(DISTINCT CASE WHEN file LIKE '%/scrape/%' THEN 'scrape' WHEN file LIKE '%/session/%' THEN 'session' WHEN file LIKE '%/scry/%' THEN 'scry' ELSE file END) FROM call_graph WHERE callee LIKE '%insert\_event%' ESCAPE '\' AND file LIKE '%commands/%'
```

```verify type="assay" label="insert_event callers across 5+ files" expect=">= 5"
callers --pattern "insert_event" | count(distinct file)
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

- 2026-01-29: Created (confidence: 0.80)
