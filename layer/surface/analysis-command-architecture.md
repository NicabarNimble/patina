# Analysis: Command Architecture vs Layer/Core Values

**Date**: 2025-12-27
**Status**: Analysis
**Context**: Review large commands against dependable-rust, unix-philosophy, adapter-pattern

---

## Summary

**Verdict**: 2 of 6 large commands violate dependable-rust pattern

| Command | Lines | Structure | Follows Pattern? |
|---------|------:|-----------|------------------|
| scrape | 126 (mod.rs) | ✅ Well-decomposed (code/, git/, sessions/, layer/, github/) | ✅ YES |
| **scry** | **2,141** | ❌ **Monolithic single file, 30 functions** | ❌ **NO** |
| **secrets** | **325** | ⚠️ **Single file, 11 functions** | ⚠️ **BORDERLINE** |
| oxidize | 363 (mod.rs) | ✅ Submodules (dependency, pairs, recipe, temporal, trainer) | ✅ YES |
| init | 118 (mod.rs) | ✅ internal/ (backup, config, patterns, validation) | ✅ YES |
| yolo | 137 (mod.rs) | ✅ Small interface | ✅ YES |

---

## Violation Details

### 🚨 Major: scry (2,141 lines)

**Current structure**:
```
src/commands/scry/
└── mod.rs (2,141 lines, 30 functions)
```

**Functions include**:
- `execute()` - Main entry point
- `scry_text()` - Semantic search
- `scry_lexical()` - FTS5 search
- `scry_file()` - File-specific search
- `execute_hybrid()` - RRF fusion search
- `execute_all_repos()` - Multi-repo search
- `execute_via_mothership()` - Remote execution
- `execute_orient()` - Structural ranking
- `execute_recent()` - Temporal queries
- `execute_why()` - Explain results
- `execute_open()` - Open result files
- `execute_copy()` - Copy to clipboard
- `execute_feedback()` - Log feedback
- `enrich_results()` - Add metadata
- `log_scry_query()` - Query logging
- `prepare_fts_query()` - Query preparation
- ... and 14 more helper functions

**Violations**:
1. ❌ **External interface not separated** - All 30 functions in one file
2. ❌ **No internal module** - No isolation of implementation details
3. ❌ **Multiple responsibilities** - Search, logging, feedback, clipboard, file ops
4. ❌ **Hard to test** - Can't test internals in isolation
5. ❌ **Hard to review** - 2,141 lines is overwhelming

---

### ⚠️ Borderline: secrets (325 lines)

**Current structure**:
```
src/commands/
└── secrets.rs (325 lines, 11 functions)
```

**Assessment**:
- Not egregious (< 500 lines)
- But could benefit from `secrets/mod.rs` + `secrets/internal/` split
- Functions mix CLI handling with vault operations

---

## Recommended Refactoring

### Priority 1: scry → dependable-rust pattern

**Target structure**:
```
src/commands/scry/
├── mod.rs              # External interface (~150 lines)
│   ├── pub fn execute()
│   ├── pub struct ScryOptions
│   └── pub use internal::{ScryResult}
└── internal/
    ├── mod.rs          # Re-exports
    ├── search.rs       # scry_text, scry_lexical, scry_file
    ├── hybrid.rs       # execute_hybrid, RRF fusion
    ├── subcommands.rs  # orient, recent, why, open, copy, feedback
    ├── routing.rs      # mothership, all_repos
    ├── enrichment.rs   # enrich_results, add metadata
    ├── logging.rs      # query logging, feedback tracking
    └── query_prep.rs   # prepare_fts_query, lexical detection
```

**Benefits**:
- ✅ External interface: ~150 lines (was 2,141)
- ✅ Each module < 300 lines
- ✅ Clear separation of concerns
- ✅ Testable in isolation
- ✅ Easy to review changes

---

### Priority 2: secrets → internal pattern

**Target structure**:
```
src/commands/secrets/
├── mod.rs              # External interface
└── internal/
    ├── vault.rs        # Vault operations
    ├── keychain.rs     # macOS Keychain integration
    ├── recipients.rs   # Recipient management
    └── ssh.rs          # SSH execution
```

---

## Core Values Alignment

### dependable-rust: Black-box modules

**Current state**:
- ✅ scrape: Perfect example (126 line mod.rs, subdirs)
- ✅ oxidize: Good (363 line mod.rs, submodules)
- ✅ init: Perfect (118 line mod.rs, internal/)
- ❌ scry: Major violation (2,141 line monolith)
- ⚠️ secrets: Borderline (325 lines, no split)

**Quote from dependable-rust.md**:
> "Keep your public interface small and stable. Hide implementation details in `internal.rs` or `internal/` and never expose them in public signatures."

**scry violates this**: All 30 functions exposed, no separation.

---

### unix-philosophy: Single responsibility

**Current state**:
- ✅ Most commands have single clear purpose
- ❌ scry mixes: search + logging + feedback + clipboard + file ops + routing

**Quote from unix-philosophy.md**:
> "Each component has a single, clear responsibility."

**scry violations**:
- Handles 7+ different subcommands
- Manages query logging
- Clipboard operations
- File opening
- Mothership routing
- All in one file

---

### Pattern Examples from Codebase

**Good examples to follow**:

1. **scrape** (perfect decomposition):
   ```
   scrape/
   ├── mod.rs (126 lines) - coordinator
   ├── code/              - code extraction
   ├── git/               - git history
   ├── sessions/          - session files
   └── layer/             - pattern files
   ```

2. **init** (good internal pattern):
   ```
   init/
   ├── mod.rs (118 lines) - external interface
   └── internal/
       ├── backup.rs
       ├── config.rs
       ├── patterns.rs
       └── validation.rs
   ```

3. **oxidize** (good submodule split):
   ```
   oxidize/
   ├── mod.rs (363 lines) - coordinator
   ├── dependency.rs
   ├── pairs.rs
   ├── recipe.rs
   ├── temporal.rs
   └── trainer.rs
   ```

---

## Proposed Action Plan

### Phase 1: Document the issue
- [x] Create this analysis document

### Phase 2: Refactor scry (HIGH PRIORITY)
- [ ] Create `src/commands/scry/internal/` directory
- [ ] Extract 7 submodules from mod.rs
- [ ] Keep mod.rs as thin coordinator (~150 lines)
- [ ] Update tests to use public interface only
- [ ] Verify benchmarks still pass

### Phase 3: Refactor secrets (MEDIUM PRIORITY)
- [ ] Create `src/commands/secrets/internal/` directory
- [ ] Split into 4 focused modules
- [ ] Keep mod.rs as public interface

### Phase 4: Update patterns documentation
- [ ] Add scry refactoring as case study to dependable-rust.md
- [ ] Show before/after comparison

---

## Benefits of Refactoring

**For scry specifically**:
1. **Reviewability**: Reviewing a 200 line PR vs 2,141 line file
2. **Testability**: Test search logic without mocking clipboard
3. **Maintainability**: Bug in logging doesn't require touching search code
4. **Onboarding**: New contributors can understand one module at a time
5. **Reusability**: Internal modules could be reused by other commands
6. **Performance**: Compiler can parallelize compilation of modules

**Risk**: LOW
- All changes are internal refactoring
- Public API stays identical
- Benchmarks verify no regression

---

## Recommendation

**Start with scry refactoring**:
- It's the biggest violation (2,141 lines)
- It's a core command (used heavily)
- Clear module boundaries already visible
- Would serve as great example for others

**Timeline**:
- Phase 2 (scry refactor): 1 session
- Phase 3 (secrets refactor): 1 session
- Total: 2 focused sessions

**Success criteria**:
- scry/mod.rs < 200 lines
- All functions in internal/
- Benchmarks pass (MRR >= 0.55)
- All tests pass
