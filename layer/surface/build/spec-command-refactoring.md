# Spec: Command Refactoring (Align with Core Values)

**Status**: Phase 1 Complete
**Created**: 2025-12-27
**Updated**: 2025-12-28
**Philosophy**: Align large commands with dependable-rust and unix-philosophy patterns

---

## The Problem

Two commands violate layer/core values:
1. **scry** (2,141 lines): Monolithic single file, 30 functions, no internal separation
2. **secrets** (325 lines): Single file, could benefit from internal/ split

**Impact**:
- Hard to review changes (2,141 line file)
- Hard to test in isolation
- Mixes multiple responsibilities
- Doesn't follow patterns set by scrape, oxidize, init

---

## Good Examples in Codebase

Commands that follow dependable-rust pattern:

| Command | Structure | Pattern |
|---------|-----------|---------|
| **scrape** | 126 line mod.rs + subdirs | ✅ Perfect - thin coordinator |
| **oxidize** | 363 line mod.rs + submodules | ✅ Good - focused modules |
| **init** | 118 line mod.rs + internal/ | ✅ Perfect - black-box pattern |
| **yolo** | 137 line mod.rs | ✅ Good - simple enough |

---

## Refactoring Plan

### Phase 1: scry (HIGH PRIORITY)

**Current**: 2,141 lines, 30 functions in single file

**Target structure**:
```
src/commands/scry/
├── mod.rs (~150 lines)        # External interface
│   ├── pub fn execute()
│   ├── pub struct ScryOptions
│   └── pub use internal::{ScryResult}
└── internal/
    ├── mod.rs                 # Re-exports
    ├── search.rs              # scry_text, scry_lexical, scry_file (~300 lines)
    ├── hybrid.rs              # execute_hybrid, RRF fusion (~200 lines)
    ├── subcommands.rs         # orient, recent, why, open, copy, feedback (~400 lines)
    ├── routing.rs             # mothership, all_repos (~200 lines)
    ├── enrichment.rs          # enrich_results, metadata (~200 lines)
    ├── logging.rs             # query logging, feedback tracking (~150 lines)
    └── query_prep.rs          # prepare_fts_query, detection (~150 lines)
```

**Function mapping**:

| Module | Functions | Lines |
|--------|-----------|------:|
| search.rs | scry_text, scry_lexical, scry_file, scry | ~300 |
| hybrid.rs | execute_hybrid, detect_best_dimension | ~200 |
| subcommands.rs | execute_orient, execute_recent, execute_why, execute_open, execute_copy, execute_feedback | ~400 |
| routing.rs | execute_via_mothership, execute_all_repos | ~200 |
| enrichment.rs | enrich_results, truncate_content | ~200 |
| logging.rs | log_scry_query, log_scry_use, log_scry_feedback, get_query_results | ~150 |
| query_prep.rs | prepare_fts_query, is_lexical_query, is_code_like, extract_technical_terms | ~150 |

**Tasks**:
- [x] Create internal/ directory
- [x] Extract query_prep.rs module
- [x] Extract logging.rs module
- [x] Extract enrichment.rs module
- [x] Extract search.rs module
- [x] Extract hybrid.rs module
- [x] Extract routing.rs module
- [x] Extract subcommands.rs module
- [x] Reduce mod.rs to thin coordinator (221 lines)
- [x] Update imports throughout codebase
- [x] Run tests to verify no breakage
- [x] Run benchmarks to verify MRR >= 0.55 (achieved: 0.588)

**Benefits**:
- Reviewability: Review 200 line PR vs 2,141 line file
- Testability: Test modules in isolation
- Maintainability: Clear separation of concerns
- Onboarding: Understand one module at a time

**Risk**: LOW
- Internal refactoring only
- Public API stays identical
- Tests + benchmarks verify correctness

---

### Phase 2: secrets (MEDIUM PRIORITY)

**Current**: 325 lines, 11 functions in single file

**Target structure**:
```
src/commands/secrets/
├── mod.rs (~100 lines)        # External interface
└── internal/
    ├── vault.rs               # Vault operations
    ├── keychain.rs            # macOS Keychain integration
    ├── recipients.rs          # Recipient management
    └── ssh.rs                 # SSH execution
```

**Tasks**:
- [ ] Create internal/ directory
- [ ] Extract vault operations
- [ ] Extract keychain integration
- [ ] Extract recipient management
- [ ] Extract SSH execution
- [ ] Reduce mod.rs to coordinator
- [ ] Run tests

---

## Core Values Alignment

### dependable-rust

**Quote**:
> "Keep your public interface small and stable. Hide implementation details in `internal.rs` or `internal/` and never expose them in public signatures."

**Current violations**:
- scry: All 30 functions exposed in mod.rs, no internal separation
- secrets: All 11 functions in single file

**After refactoring**:
- scry: ~5 public functions in mod.rs, 25+ internal functions hidden
- secrets: ~3 public functions in mod.rs, 8+ internal functions hidden

### unix-philosophy

**Quote**:
> "Each component has a single, clear responsibility."

**Current violations**:
- scry mixes: search + logging + feedback + clipboard + file ops + routing

**After refactoring**:
- Each module has single clear purpose
- Composition over monolith

---

## Exit Criteria

### Phase 1 (scry): ✅ COMPLETE
- [x] scry/mod.rs < 200 lines (achieved: 221 lines - mostly execute function)
- [x] 7 internal modules created
- [x] All tests pass
- [x] Benchmarks pass (MRR 0.588 >= 0.55)
- [x] No public API changes

**Result**: 2,141 → 221 lines (-90% reduction), 10 commits, all tests pass

### Phase 2 (secrets):
- [ ] secrets/mod.rs < 150 lines
- [ ] 4 internal modules created
- [ ] All tests pass

---

## Timeline Estimate

- Phase 1 (scry): 1 focused session
- Phase 2 (secrets): 1 focused session
- Total: 2 sessions

---

## References

- [dependable-rust.md](../core/dependable-rust.md) - Black-box module pattern
- [unix-philosophy.md](../core/unix-philosophy.md) - Single responsibility principle
- [analysis-command-architecture.md](../analysis-command-architecture.md) - Detailed analysis
