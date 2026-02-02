---
type: refactor
id: verification-module-split
status: building
created: 2026-02-01
sessions:
  origin: 20260201-222931
related:
  - layer/core/dependable-rust.md
  - layer/core/unix-philosophy.md
  - layer/surface/build/feat/belief-verification/SPEC.md
---

# refactor: Split verification.rs to Follow dependable-rust

**Parent:** [feat/belief-verification/SPEC.md](../feat/belief-verification/SPEC.md) — prerequisite for closing exit criteria.

---

## Problem Statement

`src/commands/scrape/beliefs/verification.rs` is 1737 lines containing 5 distinct concerns separated by manual section headers instead of files:

1. **Parsing** — block/attribute extraction from markdown (pure functions)
2. **Assay DSL** — command registry, parser, SQL builder (mini-language)
3. **Temporal DSL** — parser, SQL builder (mini-language)
4. **Safety** — query validation (pure functions)
5. **Execution** — dispatch, result building, storage (DB access)

`dependable-rust` says: "Split into files when it grows: `internal/{mod,parse,exec,validate}.rs`". The section comment headers are the file system trying to speak through comments.

Secondary issue: the assay command registry (`get_assay_command`) duplicates knowledge from the actual assay command implementation. Both map command names to tables/columns. This is coupling that `unix-philosophy` flags — one source of truth, not two. Noted but **out of scope** for this refactor (tracked as future work).

---

## Solution

Convert `verification.rs` (flat sibling file) into `verification/` (black-box module with internal):

```
src/commands/scrape/beliefs/
├── mod.rs                              # Belief scraper (unchanged)
└── verification/
    ├── mod.rs                          # Public interface: types + re-exports
    └── internal/
        ├── mod.rs                      # Wire internal modules
        ├── parse.rs                    # Block/attribute parsing
        ├── assay.rs                    # Assay DSL: registry, parser, SQL builder
        ├── temporal.rs                 # Temporal DSL: parser, SQL builder
        ├── safety.rs                   # Query validation
        └── exec.rs                     # Execution dispatch, result building, storage
```

Each file passes the "Do X" test:
- `parse.rs`: "Parse verification blocks from belief markdown"
- `assay.rs`: "Translate assay DSL commands into counting SQL"
- `temporal.rs`: "Translate temporal DSL commands into counting SQL"
- `safety.rs`: "Validate that verification queries are safe to execute"
- `exec.rs`: "Execute verification queries and store results"

---

## Public Interface (unchanged)

The public API used by `beliefs/mod.rs` does not change:

```rust
// Types
pub struct VerificationQuery { ... }
pub struct VerificationAggregates { ... }
pub enum VerificationStatus { ... }
pub struct VerificationResult { ... }

// Functions
pub fn parse_verification_blocks(content: &str) -> Vec<VerificationQuery>;
pub fn run_verification_queries(conn, belief_id, queries, freshness) -> (Vec<VerificationResult>, VerificationAggregates);
pub fn create_tables(conn: &Connection) -> Result<()>;
```

Zero changes to `beliefs/mod.rs`. Zero changes to `belief/mod.rs` (audit). This is a pure internal restructure.

---

## Dependency Graph (internal)

```
mod.rs (types + pub re-exports)
  │
  └── internal/
        exec.rs ──→ safety.rs
            │  ──→ assay.rs
            │  ──→ temporal.rs
        safety.rs ──→ assay.rs (parse for validation)
                  ──→ temporal.rs (parse for validation)
        assay.rs (independent)
        temporal.rs (independent)
        parse.rs (independent)
```

No cycles. DSL modules are leaves. Safety depends on DSL parsers. Exec depends on everything.

---

## Build Steps

- [ ] 1. Create `verification/mod.rs` — public types + `mod internal` + re-exports
- [ ] 2. Create `internal/mod.rs` — wire submodules
- [ ] 3. Create `internal/parse.rs` — move parsing code + tests
- [ ] 4. Create `internal/assay.rs` — move assay DSL code + tests
- [ ] 5. Create `internal/temporal.rs` — move temporal DSL code + tests
- [ ] 6. Create `internal/safety.rs` — move safety validation code + tests
- [ ] 7. Create `internal/exec.rs` — move execution/storage code + tests
- [ ] 8. Delete old `verification.rs`
- [ ] 9. Verify: `cargo test --workspace` passes (all 50 tests)
- [ ] 10. Verify: `cargo clippy --workspace` clean
- [ ] 11. Fix duplicate condition bug in `beliefs/mod.rs:307` (found during review)

---

## Exit Criteria

- [ ] All 50 existing tests pass without modification
- [ ] `cargo clippy --workspace` clean
- [ ] No file in `verification/internal/` exceeds 400 lines
- [ ] `beliefs/mod.rs` has zero changes to its `use verification::` imports
- [ ] Duplicate condition bug fixed (`beliefs/mod.rs:307`)

---

## Out of Scope

- Assay command registry deduplication (future: shared registry between assay command and verification)
- f64 EPSILON fix for float expectations (no float queries exist today)
- Refactoring the double-parse in safety→exec path (cost is trivial)
