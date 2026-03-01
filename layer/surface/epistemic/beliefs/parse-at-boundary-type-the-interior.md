---
type: belief
id: parse-at-boundary-type-the-interior
persona: architect
facets: [rust, architecture, type-safety, data-flow]
entrenchment: high
status: active
endorsed: true
extracted: 2026-03-01
revised: 2026-03-01
---

# parse-at-boundary-type-the-interior

Data enters as bytes/JSON/Value at system boundaries and must be parsed into typed structs immediately. The interior of the system flows typed data only. Every `.get("key")` chain and `.as_str()` conversion in non-boundary code is evidence of a missed parse.

## Statement

Data enters as bytes/JSON/Value at system boundaries and must be parsed into typed structs immediately. The interior of the system flows typed data only. Every `.get("key")` chain and `.as_str()` conversion in non-boundary code is evidence of a missed parse.

## Evidence

- [[session-20260301-165723]]: Structural audit found 551 `.get("key")` chains, 202 `.as_*()` conversions, and 622 `.unwrap_or()` calls across the codebase. The worst offender is `measure/internal.rs` (1,182 LOC) where `latest_metrics: serde_json::Value` flows as a domain type, causing 27 `.get()` + 25 `.as_*()` + 28 `.unwrap_or()` calls downstream. (weight: 0.95)
- [[session-20260301-165723]]: `session/internal.rs` has 43 `.unwrap_or()` calls in ~500 lines (8.6% density) — all symptomatic of untyped data flowing past the parse boundary. (weight: 0.85)

## Supports

- [[protocol-boundaries-must-be-typed]] — this belief generalizes beyond MCP to all system boundaries (DB rows, config files, JSON events)
- [[correctness-by-construction-not-convention]] — typed interiors make wrong field access impossible by construction

## Attacks

<!-- None known -->

## Attacked-By

- Performance cost of deserialization at boundary — mitigated by the fact that `.get().and_then().as_*().unwrap_or()` chains are more expensive than a single `serde::Deserialize`

## Applied-In

- `src/mcp/server/scry.rs` — ScryArgs typed struct at MCP boundary (already done via [[mcp-typed-handlers]])
- `src/mcp/server/assay.rs` — AssayArgs typed struct at MCP boundary (already done)

## Revision Log

- 2026-03-01: Created from structural audit findings
