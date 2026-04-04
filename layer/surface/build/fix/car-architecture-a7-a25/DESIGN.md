# Design: CAR Architecture (A7)

## Principle Alignment

- [[dependable-rust]]: preserve black-box boundaries, no reverse imports.
- [[patina-identity]]: scry is protocol core. The retrieval engine is the search verb's library. It must not depend on CLI command modules.

## Why This Matters

Mother serves scry via the daemon (`/api/scry`). The retrieval engine is the library behind that endpoint. If it imports from `commands::scry::internal`, Mother can't use retrieval without pulling in the entire CLI command tree. That's a build dependency that shouldn't exist and will block clean daemon extraction.

## Gate Detail

### A7: Retrieval → Commands Inversion

**Problem:** `src/retrieval/oracles/semantic.rs:13` imports `enrich_results` and `SearchResults` from `commands::scry::internal::enrichment`.

**Why it exists:** `7b64fd83` (performance fix) needed enrichment in the oracle. The code already existed in scry internals. Import was the fast path.

**Fix:** Move `enrich_results()`, `SearchResults`, and `truncate_content()` from `commands/scry/internal/enrichment.rs` to `src/retrieval/enrichment.rs`. Update imports. Leave a re-export shim in scry during transition.

**Scope boundary:** Create `src/retrieval/enrichment.rs`. Move 3 items. Update 2 import sites. Do NOT restructure the scry command tree.

**Verification:**
```
cargo check --workspace                    # compiles clean
cargo test --lib -p patina-ai              # all pass
patina scry "test query"                   # results display correctly
patina scry "test" --explain               # oracle contributions display
```

## A25 Dropped — Not an Inversion

`spec.rs` imports `commands::spec::internal` to provide unified dispatch for both CLI and Mother's spec-manager child. This was deliberately created in `57fe8836` ("close GF11 by moving spec dispatch ownership"). The spec-manager is a Mother-hosted child per [[child-construction-canon]]. The import direction is correct for daemon-first dispatch — library provides the entry point, CLI and Mother both call it.

## Out of Scope

- Correctness/panic fixes from A1-A6.
- Dead code and cleanup removals.
- Spec dispatch restructuring (A25 dropped).
