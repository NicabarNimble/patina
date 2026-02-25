---
type: fix
id: spec-show-mcp
status: active
created: 2026-02-25
sessions:
  origin: 20260224-212321
beliefs:
- specs-as-context-sources
---
# fix: Add spec.show MCP tool returning body + metadata

> MCP returns metadata but not spec body; LLM needs 3-8 file reads to load context

## Problem

To work on a spec, the LLM must: call `spec.next` (get ID), read
SPEC.md (file read), read DESIGN.md if it exists (another file read),
read referenced beliefs (N reads), read key files. No single MCP call
returns the spec body, exit criteria, and implementation plan together.

The belief [[specs-as-context-sources]] identifies this: specs should be
queryable context sources, not passive documents requiring manual discovery.

## Root Cause

All spec MCP tools return structured metadata (status, blocked_by, etc.)
but none return the SPEC.md body content. The body is where the problem
statement, fix description, and exit criteria live — the information the
LLM most needs to execute.

## Fix

Add `spec.show` MCP tool that returns:

```json
{
  "id": "spec-complete-atomicity",
  "status": "active",
  "frontmatter": { ... },
  "body": "## Problem\n\n...",
  "design": "## Commits\n\n..." | null,
  "files": ["src/commands/spec/internal/mutations.rs"]
}
```

Implementation:
1. Add `show_spec_value(id)` in `src/commands/spec/internal/queries.rs`
2. Use `load_spec()` to get frontmatter + body
3. Check for DESIGN.md in the same directory, include if present
4. Extract `## Key Files` section and return file list
5. Wire as `spec.show` MCP tool in `src/mcp/server.rs`

## Key Files

```
src/commands/spec/internal/queries.rs  — new show_spec_value()
src/mcp/server.rs                      — new spec.show tool handler
```

## Exit Criteria

- [ ] `spec.show <id>` returns frontmatter + body + design (if exists)
- [ ] DESIGN.md content included when present, null when absent
- [ ] Key files extracted from body and returned as list
- [ ] Error on nonexistent spec ID
