---
type: belief
id: protocol-boundaries-must-be-typed
persona: architect
facets: [rust, mcp, architecture, type-safety]
entrenchment: medium
status: active
endorsed: true
extracted: 2026-03-01
revised: 2026-03-01
---

# protocol-boundaries-must-be-typed

Protocol boundaries must be typed — serde_json::Value at MCP handler boundaries violates compiler-enforced-safety (Patina Identity invariant #5) because string-keyed parameter extraction compiles even when wrong. Deserialize into typed structs at the boundary so the compiler catches misspelled fields, wrong types, and missing required parameters.

## Statement

Protocol boundaries must be typed — serde_json::Value at MCP handler boundaries violates compiler-enforced-safety (Patina Identity invariant #5) because string-keyed parameter extraction compiles even when wrong. Deserialize into typed structs at the boundary so the compiler catches misspelled fields, wrong types, and missing required parameters.

## Evidence

- [[session-20260301-090927]]: Structural audit found 200 type soup operations (65 .get() chains, 65 .as_*() calls, 70 .unwrap_or() fallbacks) across 3 MCP handler files. Cross-referenced with layer/core/patina-identity.md invariant #5 (compiler-enforced safety) and belief [[correctness-by-construction-not-convention]]. Spec [[mcp-typed-handlers]] created to fix. (weight: 0.95)

## Supports

- [[correctness-by-construction-not-convention]] — typed structs make wrong parameter access impossible by construction, not by convention of matching string keys
- [[question-mark-on-option-is-silent-swallower]] — `.unwrap_or("")` on missing params is the same class of silent failure; typed deserialization surfaces the error at the boundary

## Attacks

<!-- Add beliefs this defeats -->

## Attacked-By

<!-- Add beliefs that challenge this -->

## Applied-In

- [[spec-mcp-typed-handlers]] — spec created to eliminate 200 type soup operations in `src/mcp/server/{scry,spec,assay}.rs` by replacing `&serde_json::Value` with `#[derive(Deserialize)]` structs
- `layer/surface/reports/audit/2026-03-01-structural-audit-type-soup.md` — audit report finding F-001 (Critical) documents the violation and remediation path

## Revision Log

- 2026-03-01: Created — metrics computed by `patina scrape`
