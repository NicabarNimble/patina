---
type: fix
id: spec-metadata-mutations
status: ready
created: 2026-02-25
sessions:
  origin: 20260224-212321
beliefs:
- specs-drives-tooling
---
# fix: Mutation commands for spec metadata fields

> beliefs, schemas, references, milestones fields defined in frontmatter but only editable via raw YAML

## Problem

`SpecFrontmatter` has 22 fields. Only `status`, `blocked_by`,
`blocked_reason`, `blocked_date`, `paused_reason`, `paused_date`, and
`paused_at_tag` are writable via commands. The remaining metadata fields
(`beliefs`, `schemas`, `references`, `milestones`, `related`, `target`)
require the LLM to read the YAML, edit it manually, and write it back —
error-prone and invisible to the knowledge graph until next scrape.

## Root Cause

The spec command system was built lifecycle-first: promote, pause,
complete, etc. Metadata enrichment was deferred. The fields exist in
the struct because the scraper reads them, but no mutation path writes
them.

## Fix

Add `patina spec set <id> <field> <value>` command:

Supported fields (first pass):
- `beliefs` — append/remove belief IDs
- `related` — append/remove spec/file references
- `target` — set version target
- `references` — append/remove external links

Operations:
- `patina spec set my-spec beliefs +sync-first` (append)
- `patina spec set my-spec beliefs -old-belief` (remove)
- `patina spec set my-spec target v0.33.0` (set)

Implementation:
1. New `set_spec_value()` in mutations.rs using `load_and_mutate`
2. Parse `+`/`-` prefix for list fields, plain value for scalars
3. Git stage + commit after mutation
4. Wire as `spec.set` MCP tool

Defer milestones and schemas to a later spec — they have more complex
structure (nested objects, status tracking).

## Key Files

```
src/commands/spec/internal/mutations.rs  — new set_spec_value()
src/commands/spec/mod.rs                 — new Set subcommand
src/mcp/server.rs                        — new spec.set tool
```

## Exit Criteria

- [ ] `patina spec set <id> beliefs +<belief-id>` appends to beliefs list
- [ ] `patina spec set <id> beliefs -<belief-id>` removes from beliefs list
- [ ] `patina spec set <id> related +<ref>` appends to related list
- [ ] `patina spec set <id> target <version>` sets target field
- [ ] `spec.set` MCP tool with same capabilities
- [ ] Git commit created for each mutation
