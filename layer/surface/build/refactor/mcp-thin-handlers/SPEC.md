---
type: refactor
id: mcp-thin-handlers
status: draft
created: 2026-03-01
sessions:
  origin: 20260301-100052
related:
- mcp-typed-handlers
- mcp-server-hardening
beliefs:
- mcp-is-discovery-cli-is-execution
exit_criteria: []
---
# refactor: Collapse MCP handlers to thin CLI wrappers

> MCP scry/assay handlers reimplement ~2,500 LOC of business logic that exists in CLI internals. Collapse to thin delegation (the spec.rs pattern) by creating _json() functions in CLI modules.

## Current State

## Target State

## Steps

## Exit Criteria
