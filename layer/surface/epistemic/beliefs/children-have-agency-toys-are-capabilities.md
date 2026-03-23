---
type: belief
id: children-have-agency-toys-are-capabilities
persona: architect
facets: [architecture, mother-child, security]
entrenchment: medium
status: active
endorsed: true
extracted: 2026-03-10
revised: 2026-03-21
---

# children-have-agency-toys-are-capabilities

Children have bounded agency — they make decisions and own workflow within the sandbox Mother grants. Toys are composable WASM components (capabilities) that children play with. Mother grants toys at init time; children use them independently. Classification is by agency, not by runtime. Toys are the WIT interface IS the capability — compile-time enforcement, not runtime permission flags.

## Statement

Children have bounded agency — they make decisions and own workflow within the sandbox Mother grants. Toys are composable WASM components (capabilities) that children play with. Mother grants toys at init time; children use them independently. Classification is by agency, not by runtime. Toys are the WIT interface IS the capability — compile-time enforcement, not runtime permission flags.

## Evidence

- [[session-20260310-094749]]: Converged through brainstorm: connector has no agency (toy), ducklake orchestrator makes decisions (child). Audit agent confirmed: classify by agency not runtime. (weight: 0.95)
- [[session-20260310-094749]]: "Mother knows WHERE everything is, doesn't know WHAT everything is doing at all times, BUT can know" — user's framing that led to capability-grant model. (weight: 0.9)
- [[session-20260310-094749]]: Audit agent: "initialize becomes a serious security boundary — it is no longer just startup config." Init payload is the capability token set. (weight: 0.85)
- [[session-20260310-094749]]: Audit agent: "Don't force child=WASM yet. That feels like a runtime choice, not the core model." Agency and runtime are orthogonal axes. (weight: 0.9)

## Supports

- [[agents-are-guests-mother-is-infrastructure]] — children are the composable workers Mother manages; agents are guests
- [[mother-holds-connections-pipes-transform]] — Mother orchestrates, children execute autonomously
- [[telemetry-is-process-owned]] — each actor owns its own observability (agency over telemetry)
- [[connectors-own-tables-schemas-are-contracts]] — connectors are tools with contracts, not decision-makers

## Attacks

- [[raw-lake-ingestion]] design (Mother drives pipe/ingest to passive lakehouse) — superseded by autonomous child model

## Attacked-By

- Practical concern: if children spawn toy subprocesses, sandbox enforcement becomes the child's responsibility. Mother can't enforce sandbox on toys she didn't spawn. Mitigated by: design toy interfaces as capability boundaries, enforce at the interface level.
- Practical concern: "approved toys" in initialize is a large trust surface. Mitigated by: treat initialize as security boundary, keep toy approvals explicit and coarse.

## Applied-In

- [[ducklake]] spec — first child that uses approved toys (connector, lake path, HTTP proxy, credentials)
- [[http-proxy-extraction]] — HTTP proxy as an approved capability toy, not embedded in child
- [[measure-process-owned]] — measure vocabulary as shared substrate, children own their telemetry

## Revision Log

- 2026-03-10: Created in [[session-20260310-094749]] — converged through brainstorm with audit agent. Four iterations: WASM-first → agency-not-runtime → approved-toys → capability-grant model.
- 2026-03-21: Revised in [[session-20260320-212325-011658000]] — refined "autonomous" to "bounded agency" (agency within sandbox Mother grants). Reframed toys from permission flags to composable WASM components. Added: WIT interface IS the capability (compile-time enforcement). Linked to [[agents-are-guests-mother-is-infrastructure]].
