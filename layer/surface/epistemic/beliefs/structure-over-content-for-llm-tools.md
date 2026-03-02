---
type: belief
id: structure-over-content-for-llm-tools
persona: architect
facets: [architecture, mcp, llm-integration]
entrenchment: medium
status: active
endorsed: true
extracted: 2026-02-27
revised: 2026-02-27
---

# structure-over-content-for-llm-tools

MCP tools should return structure (metadata + outlines + paths) by default, not full content — LLMs navigate better with a map than a dump

## Statement

MCP tools should return structure (metadata + outlines + paths) by default, not full content — LLMs navigate better with a map than a dump

## Evidence

- [[session-20260227-062333]]: spec.show redesign reduced [[spec-data-architecture-v2]] from ~17k tokens to ~1k tokens (93% reduction) with zero information loss — LLM sees every heading and can Read any section (weight: 0.9)
- [[commit-07e31786]]: ShowResult returns outline + design_outline + path + design_path; body/design are Option, only populated when full=true (weight: 0.8)

## Supports

- [[measure-reads-tables-not-events]] — same principle: give consumers structured data, not raw streams

## Attacks

<!-- None known -->

## Attacked-By

- Full content may be needed when LLM must search across a spec for a specific detail not captured in headings — mitigated by providing file paths for targeted Read

## Applied-In

- `src/commands/spec/internal/queries.rs` — ShowResult struct, extract_outline(), show_spec_value(id, full)
- `src/mcp/server/spec.rs` — spec.show handler passes full parameter (default false)
- `src/mcp/server/tools.rs` — MCP tool schema with full: boolean parameter

## Revision Log

- 2026-02-27: Created — metrics computed by `patina scrape`
