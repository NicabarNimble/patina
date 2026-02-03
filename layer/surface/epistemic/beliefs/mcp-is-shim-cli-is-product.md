---
type: belief
id: mcp-is-shim-cli-is-product
persona: architect
facets: [architecture, adapter-pattern, mcp, cli]
entrenchment: medium
status: active
endorsed: true
extracted: 2026-02-03
revised: 2026-02-03
---

# mcp-is-shim-cli-is-product

MCP exists as a discovery shim so LLM adapters know what tools to call — the CLI is the real interface and MCP should be a thin wrapper delegating to CLI logic, not a parallel implementation

## Statement

MCP exists as a discovery shim so LLM adapters know what tools to call — the CLI is the real interface and MCP should be a thin wrapper delegating to CLI logic, not a parallel implementation

## Evidence

- [[session-20260203-120615]]: Discovered CLI and MCP scry are different pipelines; user directive that CLI is the product, MCP is necessary evil for tool discovery (weight: 0.95)
- [[layer/surface/build/feat/mother-delivery/design.md]]: ADR-7 documents the bifurcation — MCP had its own QueryEngine path while CLI used direct search. The fix (D0) unifies under CLI-first. (weight: 0.9)
- [[layer/core/adapter-pattern.md]]: Adapter pattern says same capability regardless of delivery channel — MCP should wrap CLI, not reimplement. (weight: 0.8)

## Supports

- [[cli-unifies-code-separates]] — CLI is the UX layer, code remains independent modules
- [[adapter-pattern]] — same behavior regardless of interface

## Attacks

<!-- None identified -->

## Attacked-By

- MCP provides richer parameter typing (JSON schema) than CLI flags — thin wrapper may lose expressiveness

## Applied-In

- [[layer/surface/build/feat/mother-delivery/d0-unified-search/SPEC.md]] — D0 unifies search so CLI owns the pipeline, MCP wraps it
- Current MCP `server.rs` has its own `format_results()` and `get_project_context()` — violations of this belief that D0 should fix

## Revision Log

- 2026-02-03: Created — metrics computed by `patina scrape`
