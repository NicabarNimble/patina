---
type: belief
id: eval-stays-compiled
persona: architect
facets: [architecture, plugins, testing]
entrenchment: medium
status: active
endorsed: true
extracted: 2026-02-14
revised: 2026-02-14
---

# eval-stays-compiled

eval+bench should stay compiled because their value is ablation testing of retrieval internals (QueryEngine, RetrievalConfig, FusedResult, create_embedder) — extracting them limits testing to the host/query surface, losing the ability to test retrieval component combinations.

## Statement

eval+bench should stay compiled because their value is ablation testing of retrieval internals (QueryEngine, RetrievalConfig, FusedResult, create_embedder) — extracting them limits testing to the host/query surface, losing the ability to test retrieval component combinations.

## Evidence

- [[session-20260214-110957]]: [[plugin-extraction-map]] Section 1.7 — eval/mod.rs and bench/internal.rs have 5+ internal imports across retrieval, assay, oxidize, embeddings subsystems; coupling score HIGH with UNSTABLE format stability (weight: 0.9)

## Supports

- [[patina-identity]] Protocol Test — "Can Patina function without it?" Yes, but quality assurance degrades. eval is protocol quality tooling, not protocol tooling.

## Attacks

<!-- None identified -->

## Attacked-By

- Community customization: extracting eval would let users build custom quality suites. Counter: custom eval would be limited to host/query surface (black-box testing), losing ablation power. Community members wanting different eval strategies can contribute to the compiled module instead.
- Binary size: eval+bench add code to the binary that most users never run. Counter: the code is small relative to the retrieval engine it tests, and the cost is bounded.

## Applied-In

- `src/commands/eval/mod.rs` — imports `crate::retrieval::{FusedResult, QueryEngine, RetrievalConfig}` (direct internal access)
- `src/commands/eval/internal/scry_eval.rs` — imports `crate::commands::oxidize`, `patina::embeddings::create_embedder` (pipeline internals)
- `src/commands/bench/internal.rs` — imports `crate::retrieval::{QueryEngine, QueryOptions, RetrievalConfig}` (engine configuration)
- [[plugin-extraction-map]] Section 6 — classified as Tier 4 ("Likely stays compiled") in the updated extraction table

## Revision Log

- 2026-02-14: Created — metrics computed by `patina scrape`
