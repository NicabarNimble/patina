---
type: belief
id: mcp-is-shim-cli-is-product
persona: architect
facets: [architecture, adapter-pattern, mcp, cli]
entrenchment: medium
status: scoped
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

- [[mcp-is-discovery-cli-is-execution]] — defeated by the same reframing; the "discovery vs execution" split was a symptom of the real problem (dual implementations)

## Attacked-By

- MCP provides richer parameter typing (JSON schema) than CLI flags — thin wrapper may lose expressiveness

## Applied-In

- [[layer/surface/build/feat/mother-delivery/d0-unified-search/SPEC.md]] — D0 unifies search so CLI owns the pipeline, MCP wraps it
- Current MCP `server.rs` has its own `format_results()` and `get_project_context()` — violations of this belief that D0 should fix

## Scope Rationale

Scoped by [[agents-are-guests-mother-is-infrastructure]] (2026-03-21). The "no parallel implementation" principle remains valid — capabilities should live in one place (children with toys), not be reimplemented per delivery channel. But the "CLI is the product, MCP is the shim" framing is stale. Neither CLI nor MCP is "the product." Mother and the belief system are the product. CLI and MCP are both guest agents connecting to Mother's infrastructure. The surviving principle: **don't duplicate business logic across delivery channels** — capabilities live in children, all agents access them the same way.

## Revision Log

- 2026-02-03: Created — metrics computed by `patina scrape`
- 2026-03-21: Scoped — "no parallel implementation" survives, but "CLI is product / MCP is shim" framing replaced by [[agents-are-guests-mother-is-infrastructure]]
