---
type: refactor
id: github-child-owns-forge
status: draft
created: 2026-03-08
sessions:
  origin: 20260307-222328
related:
- github-connector
- pipe-architecture
- core-extraction
exit_criteria: []
---
# refactor: GitHub Child Owns All GitHub Interaction

> ForgeWriter bypasses the pipe architecture by shelling out to gh CLI directly. The github-connector emits events but they don't project into searchable materialized views. Consolidate all GitHub interaction into the github child so mother manages it.

## Current State

## Target State

## Steps

## Exit Criteria
