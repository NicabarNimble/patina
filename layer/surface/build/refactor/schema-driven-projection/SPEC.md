---
type: refactor
id: schema-driven-projection
status: draft
created: 2026-03-08
sessions:
  origin: 20260307-234302
related:
- pipe-architecture
- core-extraction
- github-child-owns-forge
exit_criteria: []
---
# refactor: Schema-Driven Projection — Pipeline Reads Schemas, Not Hardcoded Event Types

> Projection, FTS5, search, and oxidize hardcode event type strings (forge.issue, github.issue). A new connector (gitea, gitlab) requires modifying core code. The schema system already declares event_type → table mappings — the pipeline should read them.

## Current State

## Target State

## Steps

## Exit Criteria
