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
1. Add maximum iteration limit (failsafe):
   ```rust
   const MAX_ITERATIONS: usize = 100;
   let mut iterations = 0;
   
   loop {
       if iterations >= MAX_ITERATIONS {
           eprintln!("Warning: Max iterations reached for declarator in {:?}", file_path);
           return None;
       }
       iterations += 1;
       // ... rest of logic
   }
   ```

2. Add debug logging (optional):
   ```rust
   #[cfg(debug_assertions)]
   if iterations > 20 {
       eprintln!("Deep nesting detected: {} levels", iterations);
   }
   ```

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

1. **Create branch**: `fix-c-parser-recursion`
2. **Implement iterative function** with safety limits
3. **Add unit tests** for declarator patterns
4. **Test on SDL** repository
5. **Run full test suite** (`cargo test`)
6. **Update documentation** if needed
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

## Alternative Approaches (Rejected)

1. **Depth limit only**: Still recursive, arbitrary limit
2. **Skip complex declarators**: Loses data
3. **Tree-sitter cursor API**: More complex, less clear
4. **Restore behavioral_hints**: Masks problem, not a fix

## References

- Original iterative fix: Commit 7104d0a (Sept 1, 2025)
- Session: layer/sessions/20250901-164140.md
- Current recursive function: src/commands/scrape/code.rs:2229

## Timeline

Estimated: 1-2 hours for implementation and testing

---
*Created: 2025-09-04*
*Status: DRAFT*
*Author: AI-assisted implementation plan*