---
type: refactor
id: mcp-typed-handlers
status: draft
created: 2026-03-01
sessions:
  origin: 20260301-090927
related:
- mcp-server-hardening
- data-architecture-v2
exit_criteria: []
---
# refactor: MCP Typed Handlers — Eliminate Value Soup at Protocol Boundary

> MCP server handlers receive serde_json::Value and manually extract parameters via 400+ .get()/.as_*()/.unwrap_or() chains. Replace with #[derive(Deserialize)] structs per handler.

## Current State

## Target State

## Steps

## Exit Criteria
