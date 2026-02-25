---
type: fix
id: spec-create-related-param
status: ready
created: 2026-02-25
sessions:
  origin: 20260224-212321
---
# fix: Add related parameter to MCP spec.create

> CLI accepts --related but MCP passes empty Vec

## Problem

The CLI `patina spec create` accepts `--related` to set the `related`
field in frontmatter. The MCP `spec.create` tool handler in
`src/mcp/server.rs` does not expose this parameter — it passes
`Vec::new()` to `create_spec_value()`. LLMs creating specs via MCP
cannot declare relationships at creation time.

## Root Cause

When `spec.create` was added to MCP (session 20260224-202650, commit
c737f57e), the `related` parameter was omitted from the tool schema.
The CLI clap definition has it but the MCP schema doesn't.

## Fix

1. Add `related` parameter to `spec.create` tool schema in
   `src/mcp/server.rs` (array of strings, optional)
2. Pass it through to `create_spec_value()` instead of `Vec::new()`
3. Match the CLI behavior: values populate `related:` in frontmatter

Single-file change in `src/mcp/server.rs`.

## Key Files

```
src/mcp/server.rs  — spec.create tool schema + handler
```

## Exit Criteria

- [ ] `spec.create` MCP tool accepts `related` parameter (array of strings)
- [ ] Created spec has `related:` field populated when parameter provided
- [ ] Omitting `related` parameter still works (empty list, no field in YAML)
