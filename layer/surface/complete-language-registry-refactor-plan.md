# Complete Language Registry Refactor Plan

## Overview
The language registry refactor is partially complete. This plan outlines the work needed to finish migrating all language-specific logic from scattered match statements to the centralized LanguageSpec registry.

## Current State

### ✅ Already Migrated to Registry
- `is_doc_comment` - Checking if comments are documentation
- `parse_visibility` - Determining public/private visibility  
- `has_async` - Checking async functions
- `has_unsafe` - Checking unsafe functions
- `get_symbol_kind` - Basic symbol type detection
- `get_symbol_kind_complex` - Complex symbol type detection with node inspection

### ❌ Still Using Match Statements (9 remaining)
1. **Doc comment extraction** (~line 1690)
2. **Clean doc comments** (~line 1700) 
3. **Extract symbol name** (for special cases)
4. **Extract parameters** (~line 2420) - 85 lines
5. **Extract return type** (~line 2520) - 40 lines
6. **Count generics** (~line 2560)
7. **Determine visibility** (~line 2610) - 50 lines
8. **Extract imports** (~line 2688) - 100+ lines
9. **File type checking** (~line 1196)

### 🚫 Unused Registry Fields
- `extensions` - File extensions for each language
- `function_nodes` - AST nodes representing functions
- `struct_nodes` - AST nodes representing structs  
- `trait_nodes` - AST nodes representing traits/interfaces
- `import_nodes` - AST nodes representing imports
- `extract_params` - Function to extract parameters (defined but not called)
- `extract_return_type` - Function to extract return types (defined but not called)
- `extract_generics` - Function to extract generics (defined but not called)
- `extract_call_target` - Function to extract call targets (defined but not called)

## Implementation Plan

### Phase 1: Wire Up Already-Defined Functions (30 mins)
These functions are already implemented in each LanguageSpec but aren't being called:

1. **Replace parameter extraction match** (~line 2420)
   - Delete the 85-line match statement
   - Replace with: `let params = (spec.extract_params)(&node, source)`
   - Test with sample code from each language

2. **Replace return type extraction match** (~line 2520)
   - Delete the 40-line match statement  
   - Replace with: `let return_type = (spec.extract_return_type)(&node, source)`
   - Verify return types are still captured correctly

3. **Replace generics extraction match** (~line 2560)
   - Delete the small match statement
   - Replace with: `let generics = (spec.extract_generics)(&node, source)`

### Phase 2: Add Missing Functions to LanguageSpec (1 hour)

1. **Add `clean_doc_comment` function**
   ```rust
   clean_doc_comment: fn(&str) -> String,
   ```
   - Move logic from match statement at ~line 1700
   - Implement for each language (Rust, Go, Python, JS/TS)

2. **Add `extract_import_details` function**
   ```rust
   extract_import_details: fn(&Node, &[u8]) -> ImportDetails,
   ```
   - Move logic from match statement at ~line 2688
   - Return struct with: `imported_item`, `imported_from`, `is_external`

3. **Add `extract_special_name` function** (if needed)
   ```rust
   extract_special_name: fn(&Node, &[u8]) -> Option<String>,
   ```
   - For special cases like Go's type_spec

### Phase 3: Utilize Data Fields (30 mins)

1. **Use `extensions` field**
   - Replace file type checking match at ~line 1196
   - Use: `spec.extensions.contains(&extension)`

2. **Document node type fields** (for future use)
   - Add comments explaining when `function_nodes`, `struct_nodes`, etc. would be used
   - These might be for future query/search functionality

### Phase 4: Testing & Validation (1 hour)

1. **Create test file for each language**
   - Rust: functions, structs, traits, generics, async/unsafe
   - Go: functions, structs, interfaces, imports
   - Python: functions, classes, decorators, imports
   - TypeScript: functions, classes, interfaces, exports

2. **Run before/after comparison**
   - Scrape a test project before changes
   - Scrape same project after changes
   - Diff the resulting databases to ensure same data extracted

3. **Performance check**
   - Ensure registry lookups aren't slower than match statements
   - Should be negligible difference

### Phase 5: Cleanup (30 mins)

1. **Remove all match language statements**
2. **Ensure consistent pattern throughout**
3. **Update documentation**
4. **Run clippy and fix any warnings**

## Success Criteria

- [ ] All 9 remaining match statements replaced with registry lookups
- [ ] All LanguageSpec fields are utilized (no dead code warnings)
- [ ] All tests pass
- [ ] Scrape output remains identical for test projects
- [ ] Code is more maintainable and consistent

## Benefits After Completion

1. **Single source of truth** - All language-specific logic in one place per language
2. **Easy language addition** - Just create a new LanguageSpec
3. **LLM-friendly** - Clear, consistent patterns easier to understand
4. **Maintainable** - Want to change how Rust extracts parameters? Only one place to look
5. **Type-safe** - Compiler ensures all languages implement all required behavior

## Risk Mitigation

- **Incremental approach** - Complete one function at a time
- **Test after each change** - Don't batch changes
- **Keep Cairo working** - Ensure our new Cairo support isn't broken
- **Git commits** - Commit after each successful migration for easy rollback

## Estimated Time

**Total: 3-4 hours**
- Phase 1: 30 minutes
- Phase 2: 1 hour  
- Phase 3: 30 minutes
- Phase 4: 1 hour
- Phase 5: 30 minutes

## Next Steps

1. Create a branch off current work: `complete-registry-refactor`
2. Start with Phase 1 (lowest risk, already implemented)
3. Test thoroughly after each phase
4. Merge back to work branch when complete