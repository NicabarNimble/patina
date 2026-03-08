---
type: refactor
id: connector-owns-tables
status: draft
created: 2026-03-08
sessions:
  origin: 20260308-070818
related:
- pipe-architecture
- schema-driven-projection
- core-extraction
exit_criteria: []
---
# refactor: Connector-Owns-Tables — Schema-Driven DDL and Domain-Specific Materialized Views

> Replace shared forge_issues/forge_prs tables with connector-declared tables. Each schema.toml declares its own DDL, table names, and projection shape. Non-forge connectors (Slack, Google Workspace) get first-class support without fitting into issue/PR shapes.

## Current State

## Target State

## Steps

## Exit Criteria
