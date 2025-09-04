# Fix C Parser Stack Overflow - Implementation Plan

## Problem Statement
The `extract_c_function_name()` function in `src/commands/scrape/code.rs` uses recursion to handle nested C declarators (function pointers, etc.). This causes stack overflow when processing files with deeply nested declarations, particularly in the SDL repository (1332 C files).

## Root Cause Analysis
- Function recurses on lines 2239 and 2246 without depth limits
- Each recursion adds stack frames for complex declarators like `void (*(*func_ptr)(int))(char)`
- Previously masked by behavioral_hints processing, now exposed after cleanup
- Affects both `work` and `cleanup-unused-tables` branches

## Proposed Solution: Iterative Traversal

### Implementation Strategy
Convert `extract_c_function_name()` from recursive to iterative using a loop-based approach:

```rust
fn extract_c_function_name(declarator: tree_sitter::Node) -> Option<tree_sitter::Node> {
    let mut current = declarator;
    
    loop {
        // Check if we found the identifier
        if current.kind() == "identifier" {
            return Some(current);
        }
        
        // Navigate through nested declarators iteratively
        if current.kind() == "function_declarator" || current.kind() == "pointer_declarator" {
            if let Some(inner) = current.child_by_field_name("declarator") {
                current = inner;
                continue;
            }
        }
        
        // Fallback: check first child
        if let Some(child) = current.child(0) {
            if child.kind() == "identifier" {
                return Some(child);
            }
        }
        
        break;
    }
    
    None
}
```

### Safety Measures
1. **Include maximum iteration limit**: Set to 100 iterations as failsafe
   - Prevents infinite loops on malformed AST nodes
   - Returns None with warning if limit reached
   - 100 is generous - real code rarely exceeds 10 levels

2. **No debug logging**: Keep function pure and fast
   - Stack overflow fix should be silent
   - Performance matters when processing thousands of files

## Testing Plan

### Phase 1: Unit Tests
Create test cases for various C declarator patterns:
- Simple function: `void func(int x)`
- Function pointer: `void (*func_ptr)(int)`
- Nested function pointer: `void (*(*func_ptr)(int))(char)`
- Array of function pointers: `void (*funcs[10])(int)`
- Complex real-world example from SDL

### Phase 2: Integration Testing
1. Test on small C file with known complex declarations
2. Test on SDL repository (1332 C files, 84k+ symbols)
3. Verify symbol count matches previous successful runs
4. Check for any missing function names

### Phase 3: Regression Testing
Run on other repositories to ensure no breakage:
- dust (mixed languages)
- dagger (Go heavy)
- Small test repos with various C patterns

## Implementation Steps

1. **Create branch**: `fix-c-parser-recursion` ✓ (done)
2. **Implement iterative function** with 100 iteration limit
3. **Test on SDL** repository first (prove the fix works)
4. **Add unit tests** for edge cases discovered
5. **Test on other repos** (dust, dagger) for regression
6. **Run full test suite** (`cargo test`)
7. **Merge to work branch** after validation

## Success Criteria

- [ ] SDL repository processes without stack overflow
- [ ] Extracts same number of symbols (±1%) as before
- [ ] No performance degradation (< 10% slower)
- [ ] All existing tests pass
- [ ] Code is cleaner and more maintainable

## Risk Assessment

**Low Risk**: 
- Change is localized to one function
- Pattern already proven with `process_c_cpp_iterative()`
- Easy to revert if issues found

**Potential Issues**:
- Edge cases in declarator patterns we haven't seen
- Possible slight differences in name extraction
- May need adjustment for C++ specific declarators

## Decision Rationale

**Why iterative over alternatives:**
- Already proven pattern in `process_c_cpp_iterative()`  
- Simpler than tree-sitter cursor API
- Complete fix vs workarounds (depth limits/skipping)
- Maintains all functionality while fixing root cause

**Implementation choices:**
- 100 iteration limit: Generous but prevents pathological cases
- Test-after approach: Fix the burning issue (SDL) first, then strengthen
- Silent operation: No logging in hot path for performance
- Preserve exact same behavior: Only change is recursion → iteration

## References

- Original iterative fix: Commit 7104d0a (Sept 1, 2025)
- Session: layer/sessions/20250901-164140.md
- Current recursive function: src/commands/scrape/code.rs:2229

## Execution Order

1. Implement iterative version with limit (15 minutes)
2. Test on SDL to confirm fix (5 minutes)  
3. Check symbol counts match (5 minutes)
4. Test on other repos (10 minutes)
5. Write unit tests for edge cases (30 minutes)
6. Final validation and merge (15 minutes)

**Total: ~1.5 hours**

---
*Created: 2025-09-04*
*Status: READY FOR IMPLEMENTATION*
*Branch: fix-c-parser-recursion*