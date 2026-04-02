---
type: belief
id: children-have-agency-toys-are-capabilities
persona: architect
facets: [architecture, mother-child, security]
entrenchment: medium
status: active
endorsed: true
extracted: 2026-03-10
revised: 2026-03-26
---

# children-have-agency-toys-are-capabilities

Children have bounded agency — they make decisions and own workflow within the sandbox Mother grants. Children are WASM runtime workers in the active architecture. Toys are capability surfaces Mother grants at init time; children use them independently within those bounds. Toys are capability contracts, not standalone child runtimes.

## Statement

Children have bounded agency — they make decisions and own workflow within the sandbox Mother grants. Children are WASM runtime workers in the active architecture. Toys are capability surfaces Mother grants at init time; children use them independently within those bounds. Toys are capability contracts, not standalone child runtimes.

## Evidence

- [[session-20260310-094749]]: Converged through brainstorm: connector has no agency (toy), ducklake orchestrator makes decisions (child). Audit agent confirmed: classify by agency not runtime. (weight: 0.95)
- [[session-20260310-094749]]: "Mother knows WHERE everything is, doesn't know WHAT everything is doing at all times, BUT can know" — user's framing that led to capability-grant model. (weight: 0.9)
- [[session-20260310-094749]]: Audit agent: "initialize becomes a serious security boundary — it is no longer just startup config." Init payload is the capability token set. (weight: 0.85)
- [[session-20260319-071818-503477000]]: Native child dual-system was removed as dead architecture; child runtime lane stayed WASM-centered after SDK cleanup. (weight: 1.0)
- [[session-20260325-064204-876122000]]: Architecture decision locked: Mother internal services + external WASM children; toys remain granted capabilities, not child runtime replacements. (weight: 1.0)

## Supports

- [[agents-are-guests-mother-is-infrastructure]] — children are the composable workers Mother manages; agents are guests
- [[children-are-wasm]] — child runtime lane is explicitly WASM-only in active doctrine
- [[telemetry-is-process-owned]] — each actor owns its own observability (agency over telemetry)
- [[connectors-own-tables-schemas-are-contracts]] — connectors are source-boundary adapters with schema contracts, not decision-makers

## Attacks

- [[raw-lake-ingestion]] design (Mother drives pipe/ingest to passive lakehouse) — superseded by autonomous child model

## Attacked-By

- Practical concern: if children spawn toy subprocesses, sandbox enforcement becomes the child's responsibility. Mother can't enforce sandbox on toys she didn't spawn. Mitigated by: toy interfaces are WIT capability boundaries enforced at the host import level (compile-time + runtime).
- Practical concern: toy grants in init are a trust surface. Mitigated by: `GrantedCapabilities` resolved from explicit `[needs].toys` in child.toml manifests; grants are coarse, typed, and fail-closed on mismatch.

## Applied-In

- [[child-construction-canon]] — codifies this belief as hard rule 2 ("Toys are explicit grants") and hard rule 3 ("Least-privilege toyboxes"). Registry of reusable children proven across 3 MVPs.
- [[ducklake]] spec — first child using granted toys via `GrantedCapabilities` resolved from `[needs].toys` in `child.toml`
- `src/child/internal/mod.rs` — `GrantedCapabilities` struct built at init time from manifest; capabilities are resolved once and checked at call-time via Host impl
- [[toy-collapse-wasi-alignment]] — collapsed 22 toys to 10 primitives (connect, store, events, task, peer, git + WASI http, fs + shimmed log, state); init-time grant model survived and strengthened

## Revision Log

- 2026-03-10: Created in [[session-20260310-094749]] — converged through brainstorm with audit agent. Four iterations: WASM-first → agency-not-runtime → approved-toys → capability-grant model.
- 2026-03-21: Revised in [[session-20260320-212325-011658000]] — refined "autonomous" to "bounded agency" (agency within sandbox Mother grants). Reframed toys from permission flags to composable WASM components. Added: WIT interface IS the capability (compile-time enforcement). Linked to [[agents-are-guests-mother-is-infrastructure]].
- 2026-03-25: Revised — retired runtime-orthogonal wording after native child lane removal; children remain WASM workers, toys remain granted capability surfaces.
- 2026-03-26: Revised — updated Applied-In and Attacked-By to reflect post-toy-collapse reality (22→10 toys, `GrantedCapabilities` from `[needs].toys`, WIT per-interface packages). Removed stale HTTP proxy / credentials toy references.
