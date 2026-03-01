---
type: refactor
id: type-measure-domain
status: draft
created: 2026-03-01
sessions:
  origin: 20260301-165723
related:
- enum-status-types
exit_criteria: []
---
# refactor: Type the measure domain model

> measure/internal.rs uses serde_json::Value as domain state causing 80+ .get().as_*().unwrap_or() chains — replace with typed structs

## Current State

## Target State

## Steps

## Exit Criteria
