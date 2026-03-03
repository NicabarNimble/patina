---
type: feat
id: cli-skills-over-mcp
status: draft
created: 2026-03-03
sessions:
  origin: 20260303-135648
related:
- drift-detection
beliefs:
- patina-tools-are-patina-interface
- plugins-are-three-prong-bundles
- probe-emits-dashboard-displays
exit_criteria:
- id: search-skill-uses-cli
  text: A `/search` or `/scry` skill exists that calls `patina scry --json` via Bash, not MCP
  checked: false
- id: spec-skill-uses-cli
  text: '`/spec` command instructs LLM to use `patina spec` CLI commands, no MCP references'
  checked: false
- id: context-skill-uses-cli
  text: A `/context` skill exists that calls `patina context --json` via Bash
  checked: false
- id: all-adapters-have-skills
  text: Claude, Gemini, and OpenCode adapters all install CLI-based search/context skills via `patina adapter refresh`
  checked: false
- id: mcp-not-required-in-claudemd
  text: CLAUDE.md Patina section references CLI commands, not MCP tools
  checked: false
---
# feat: CLI Skills as Primary LLM Interface — Deprecate MCP Dependency

> Adapter skills should use `patina` CLI (via Bash) as the execution
> layer, not MCP tools. MCP server adds a stale proxy between LLM and
> knowledge — skills calling CLI are always current, debuggable, composable.

## Problem

Patina's LLM interface is split across two delivery mechanisms:

1. **MCP tools** (`mcp__patina__scry`, `mcp__patina__context`, `mcp__patina__assay`)
   — require a running MCP server process, go stale when binary is updated,
   add latency, can't compose with pipes/jq.

2. **CLI commands** (`patina scry`, `patina context`, `patina spec next`)
   — always current binary, debuggable by humans, composable, zero moving parts.

All 3 adapters (Claude Code, Gemini CLI, OpenCode) use skills/commands as
their primary extension mechanism. Skills expand to prompts that instruct
the LLM to call tools via Bash. The MCP server is a parallel path that
duplicates what the CLI already does, but worse.

Evidence: session 20260204-110139 decision — "CLI is primary LLM interface,
not MCP. Skills system pushes toward CLI." Session 20260303-135648 — MCP
server served stale index after `cargo install` + `patina oxidize` because
it was a separate process running the old binary.

## Solution

Add CLI-based skills for the core search/query commands that LLMs need:

### 1. `/search` (or `/scry`) skill
Instruct LLM to use `patina scry "query" --json` via Bash for semantic
search, `patina assay --query "query" --json` for structural/FTS5 search.
Explain when to use which (meaning vs keywords).

### 2. Update `/spec` command
Remove MCP references. Instruct LLM to use `patina spec next --json`,
`patina spec list --json`, etc. via Bash.

### 3. `/context` skill
Call `patina context --topic "topic" --json` via Bash. Returns patterns,
beliefs, and search results for a topic.

### 4. Update CLAUDE.md
Replace the MCP tools section with CLI command references. The LLM should
reach for `patina` CLI, not `mcp__patina__*`.

### 5. All adapters
`patina adapter refresh` installs these skills for all 3 adapters in their
native format (markdown for Claude/OpenCode, TOML for Gemini).

## Non-Goals

- **Removing MCP server** — it can stay as an optional interface for
  non-CLI consumers. Just not the primary path.
- **Changing CLI output formats** — `--json` already works on all commands.
- **New CLI commands** — everything needed already exists.
