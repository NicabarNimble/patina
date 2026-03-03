---
type: belief
id: measure-is-ambient-health-for-llm-context
persona: architect
facets: [architecture, measure, mcp, llm-tools]
entrenchment: medium
status: active
endorsed: true
extracted: 2026-03-03
revised: 2026-03-03
---

# measure-is-ambient-health-for-llm-context

The highest leverage of the measure surface is ambient health awareness — baking measure health into context/MCP responses so every LLM interaction carries project health signals without explicit measure calls

## Statement

The highest leverage of the measure surface is ambient health awareness — baking measure health into context/MCP responses so every LLM interaction carries project health signals without explicit measure calls

## Evidence

- [[session-20260303-054447]]: Session 2 of data-measure-surface — after building the full typed report, identified that measure data is underused because LLMs must explicitly call measure separately from context (weight: 0.9)
- [[data-measure-surface]]: SPEC delivered FullMeasureReport with health summary, diagnostics, freshness — the infrastructure exists but isn't wired into ambient context (weight: 0.8)
- [[session-20260303-054447]]: [[mcp-is-shim-cli-is-product]] — MCP is the LLM interface; measure already returns typed FullMeasureReport via MCP, but context tool doesn't include it (weight: 0.7)

## Supports

- [[measure-reads-tables-not-events]] — measure is read-only dashboard; more probes emitting = richer dashboard without measure changing
- [[structure-over-content-for-llm-tools]] — typed health summary is exactly the structure LLMs need
- [[mcp-is-shim-cli-is-product]] — MCP as shim means context tool can include health cheaply

## Attacks

<!-- None identified -->

## Attacked-By

<!-- Potential: adding measure to context increases response size and latency -->

## Applied-In

- `src/commands/measure/internal.rs`: `build_full_report()` and `mcp_measure()` — the typed infrastructure that context could call
- `src/mcp/server/mod.rs`: `handle_measure()` — already serializes FullMeasureReport for MCP consumers

## Revision Log

- 2026-03-03: Created — metrics computed by `patina scrape`
