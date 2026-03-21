---
type: belief
id: mcp-is-discovery-cli-is-execution
persona: architect
facets: [architecture, mcp, cli, adapter-pattern]
entrenchment: medium
status: defeated
endorsed: true
extracted: 2026-03-01
revised: 2026-03-01
---

# mcp-is-discovery-cli-is-execution

MCP's value is tool discovery (schema, parameters, descriptions) not parallel execution — handlers should be thin protocol wrappers delegating to CLI internals, not reimplementations of business logic. The spec.rs pattern (pure _value() delegation) is the reference architecture; scry/assay bifurcation is the counter-example.

## Statement

MCP's value is tool discovery (schema, parameters, descriptions) not parallel execution — handlers should be thin protocol wrappers delegating to CLI internals, not reimplementations of business logic. The spec.rs pattern (pure _value() delegation) is the reference architecture; scry/assay bifurcation is the counter-example.

## Evidence

- [[session-20260301-100052]]: Structural analysis of MCP server post [[mcp-typed-handlers]] revealed the bifurcation spectrum — spec.rs is pure delegation (zero business logic), scry.rs has 1,225 LOC of parallel implementation (orient/recent/why/detail/use handlers, format_results, log_mcp_query), assay.rs reimplements 7 SQL query types verbatim. The contrast between spec and scry/assay proves the pattern. (weight: 0.95)
- [[session-20260301-100052]]: [[mcp-is-shim-cli-is-product]] established MCP-as-shim principle; this belief refines it with the specific architectural pattern (thin wrapper vs parallel implementation) and names spec.rs as the reference implementation. (weight: 0.9)
- [[session-20260301-100052]]: [[bridges-become-permanent]] — the MCP execution path grew from a discovery shim into permanent parallel infrastructure because no retirement spec was created. Each new feature (expanded_terms, impact, query_id) was added to MCP directly rather than to CLI internals. (weight: 0.85)

## Supports

- [[mcp-is-shim-cli-is-product]] — refines the "shim" principle with a concrete architectural pattern and reference implementation
- [[correctness-by-construction-not-convention]] — thin wrappers make duplication impossible by construction; parallel implementations rely on convention ("update both places")
- [[bridges-become-permanent]] — explains HOW the bifurcation happened (no retirement spec)

## Attacks

- [[mcp-is-shim-cli-is-product]] — supersedes the weaker formulation; "shim" is vague, "discovery + thin delegation" is prescriptive

## Attacked-By

- MCP-unique features (expanded_terms, impact annotation, query_id feedback loop) exist because LLM consumers have different needs than CLI users — some MCP logic is genuinely novel, not duplicated
- Performance: thin wrappers that shell out to CLI add process overhead vs direct function calls — but calling internal `_value()` functions avoids this (no shell involved)

## Applied-In

- `src/mcp/server/spec.rs` — reference implementation: every match arm calls a `_value()` function from `crate::commands::spec`, zero business logic in MCP handler
- `src/mcp/server/scry.rs` — counter-example: `handle_orient()`, `handle_recent()`, `handle_detail()`, `format_results()`, `log_mcp_query()` are parallel implementations that should be `_json()` functions in CLI internals
- `src/mcp/server/assay.rs` — counter-example: Inventory, Imports, Importers, Functions, Callers, Callees, Derive query types have SQL duplicated from CLI; only Search, Cochange, Belief delegate to `_json()` functions

## Defeat Rationale

Defeated by [[agents-are-guests-mother-is-infrastructure]] (2026-03-21). The MCP-vs-CLI distinction this belief names doesn't exist in the new architectural model. Mother is the daemon that owns all query responsibility (scry, assay, measure, context). Both CLI and MCP are guest agents connecting to Mother — neither has a privileged "discovery" or "execution" role. The bifurcation problem this belief identified was real, but the solution is "Mother owns queries, all callers delegate" not "MCP discovers, CLI executes."

The underlying principle — don't duplicate business logic across delivery channels — survives in [[agents-are-guests-mother-is-infrastructure]] and [[children-have-agency-toys-are-capabilities]]. Capabilities live in children with toys. All agents access the same children.

## Revision Log

- 2026-03-01: Created — metrics computed by `patina scrape`
- 2026-03-21: Defeated — superseded by [[agents-are-guests-mother-is-infrastructure]]; the MCP/CLI distinction dissolves when both are guest agents connecting to Mother
