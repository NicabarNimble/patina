# Scrape Language Registry Refactor Plan

## Goal
Consolidate all language-specific logic scattered across 19+ locations in `src/commands/scrape/code.rs` into a single registry pattern at the top of the file.

## Success Criteria
- [ ] All language logic in ONE place
- [ ] No modification to external interface
- [ ] All tests still pass
- [ ] Adding a language requires changing ONE location
- [ ] Code remains in single file (no splitting)

## Refactor Steps

### Phase 1: Setup Registry Infrastructure
- [x] Add LanguageSpec struct definition (lines ~100-150)
- [x] Define all required function signatures in LanguageSpec
- [x] Add lazy_static dependency if not present (used std::sync::LazyLock instead)
- [x] Create empty LANGUAGE_REGISTRY HashMap
- [x] Commit: "refactor: add language registry infrastructure"

### Phase 2: Create Rust Language Specification
- [x] Define RUST_SPEC constant with all Rust rules
- [x] Implement all function fields for Rust:
  - [x] is_doc_comment
  - [x] parse_visibility  
  - [x] has_async
  - [x] has_unsafe
  - [x] extract_params
  - [x] extract_return_type
  - [x] extract_generics
  - [x] get_symbol_kind
  - [x] extract_call_target
- [x] Add RUST_SPEC to registry
- [x] Commit: "refactor: implement Rust language specification"

### Phase 3: Create Go Language Specification
- [ ] Define GO_SPEC constant with all Go rules
- [ ] Implement all function fields for Go
- [ ] Add GO_SPEC to registry
- [ ] Commit: "refactor: implement Go language specification"

### Phase 4: Create Python Language Specification
- [ ] Define PYTHON_SPEC constant with all Python rules
- [ ] Implement all function fields for Python
- [ ] Add PYTHON_SPEC to registry
- [ ] Commit: "refactor: implement Python language specification"

### Phase 5: Create JavaScript/TypeScript Specifications
- [ ] Define JS_SPEC constant
- [ ] Define TS_SPEC constant
- [ ] Define JSX_SPEC constant
- [ ] Define TSX_SPEC constant
- [ ] Add all to registry
- [ ] Commit: "refactor: implement JavaScript/TypeScript specifications"

### Phase 6: Create Solidity Specification
- [ ] Define SOLIDITY_SPEC constant
- [ ] Implement all function fields for Solidity
- [ ] Add SOLIDITY_SPEC to registry
- [ ] Commit: "refactor: implement Solidity specification"

### Phase 7: Replace Scattered Logic with Registry Lookups
- [ ] Replace parse_visibility match statements (line ~1677)
- [ ] Replace has_async match statements (line ~1707)
- [ ] Replace has_unsafe match statements (line ~1731)
- [ ] Replace extract_doc_comment match statements (line ~971)
- [ ] Replace get_symbol_kind match statements (lines 1292-1298)
- [ ] Replace extract_call_expressions match statements (lines 1088-1098)
- [ ] Replace extract_function_facts match statements (line ~1776)
- [ ] Replace extract_type_definition match statements (line ~1874)
- [ ] Replace extract_import_fact match statements (line ~2043)
- [ ] Commit: "refactor: replace match statements with registry lookups"

### Phase 8: Testing & Validation
- [ ] Run `cargo build` to ensure compilation
- [ ] Run `cargo test` to ensure tests pass
- [ ] Run `cargo clippy` to check for issues
- [ ] Test `patina scrape code` on current directory
- [ ] Verify output matches pre-refactor behavior
- [ ] Commit: "refactor: validate and test registry implementation"

### Phase 9: Cleanup
- [ ] Remove any dead code from old match statements
- [ ] Add documentation comments to LanguageSpec
- [ ] Add example in comments for adding new language
- [ ] Run `cargo fmt` to ensure formatting
- [ ] Commit: "refactor: cleanup and documentation"

### Phase 10: Final Validation
- [ ] Full test of scraping with all languages
- [ ] Document any behavior changes (should be none)
- [ ] Update scrape-code-analysis.md with results
- [ ] Final commit: "refactor: complete language registry migration"

## Rollback Plan
If issues arise:
1. Each commit is atomic and can be reverted
2. Original logic remains until Phase 7
3. Can run both systems in parallel for validation

## Notes
- Keep all changes in single file
- Maintain backward compatibility
- No external interface changes
- Each commit should compile and pass tests