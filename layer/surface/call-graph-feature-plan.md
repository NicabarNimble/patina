# Call Graph Feature Plan

## Overview
Implement call graph analysis in the scrape command to track function relationships and dependencies. This will enable understanding of code flow, dependency analysis, and impact assessment for refactoring.

## Current State
- `extract_call_target` function is defined for each language in LanguageSpec
- Currently extracts the target name from call expressions
- Not integrated into the scrape workflow
- No database schema for storing call relationships

## Goals
1. Track which functions call which other functions
2. Build a queryable graph of function relationships
3. Enable queries like:
   - "What functions does X call?"
   - "What functions call Y?"
   - "What's the call chain from main() to function Z?"
   - "What functions are never called (dead code)?"

## Implementation Plan

### Phase 1: Database Schema (30 mins)
Add new table to store call relationships:

```sql
CREATE TABLE IF NOT EXISTS call_graph (
    caller_file TEXT NOT NULL,
    caller_function TEXT NOT NULL,
    callee_function TEXT NOT NULL,
    call_count INTEGER DEFAULT 1,
    is_external BOOLEAN DEFAULT FALSE,
    PRIMARY KEY (caller_file, caller_function, callee_function)
);
```

### Phase 2: Function Body Traversal (1 hour)
Currently, we only look at function signatures. We need to:

1. Add function to traverse function bodies:
```rust
fn extract_function_calls(
    function_node: Node,
    source: &[u8],
    file_path: &str,
    function_name: &str,
    sql: &mut String,
    language: Language,
)
```

2. Walk the function body AST to find:
   - Direct function calls: `foo()`
   - Method calls: `obj.method()`
   - Static calls: `String::new()`
   - Indirect calls (function pointers, callbacks)

### Phase 3: Call Target Resolution (1 hour)
Use the existing `extract_call_target` functions to:

1. Extract the function/method name being called
2. Determine if it's:
   - Local to current file
   - From same crate/module (internal)
   - From external dependency (external)

3. Handle special cases:
   - Recursive calls
   - Calls through traits/interfaces
   - Dynamic dispatch

### Phase 4: Integration Points (30 mins)
Modify `extract_function_facts` to:

1. After extracting function signature, traverse its body
2. Find all call expressions within the function
3. Extract targets and store relationships
4. Count multiple calls to same target

### Phase 5: Testing (1 hour)

Create test files with known call patterns:

```rust
// test_calls.rs
fn main() {
    helper();
    helper(); // Called twice
    external::function();
}

fn helper() {
    println!("Hello"); // External call
}

fn unused() {} // Never called
```

Verify:
- Correct call counts
- External vs internal classification
- Dead code detection

### Phase 6: Queries and Analysis (1 hour)

Add query helpers:
1. Find call chains (BFS/DFS through graph)
2. Detect circular dependencies
3. Calculate function importance (PageRank-style)
4. Find unused functions

## Technical Considerations

### Performance
- Function body traversal adds overhead
- Consider adding flag: `--with-call-graph`
- Cache results for large codebases

### Accuracy
- Dynamic languages (Python, JS) harder to analyze
- Trait/interface calls need special handling
- Macro-generated calls in Rust

### Language-Specific Challenges

**Rust:**
- Macro expansions
- Trait method calls
- Closure captures

**Go:**
- Interface method calls
- Goroutine launches
- Deferred calls

**Python:**
- Dynamic function calls: `getattr(obj, 'method')()`
- Decorators that wrap functions
- Import aliases

**JavaScript/TypeScript:**
- Callbacks and promises
- Dynamic property access
- Arrow functions

## Success Metrics
- [ ] Call relationships correctly extracted
- [ ] External vs internal calls classified
- [ ] Dead code identified
- [ ] Call counts accurate
- [ ] Performance acceptable (<2x slowdown)

## Future Enhancements
1. **Call context**: Track parameters passed
2. **Call site info**: Line numbers of calls
3. **Async tracking**: Track async call chains
4. **Visualization**: Export to GraphViz/D3.js
5. **Change impact**: "If I change X, what breaks?"

## Estimated Time
**Total: 5-6 hours**
- Phase 1: 30 minutes
- Phase 2: 1 hour
- Phase 3: 1 hour
- Phase 4: 30 minutes
- Phase 5: 1 hour
- Phase 6: 1 hour

## Next Steps
1. Create branch: `call-graph-feature`
2. Start with Phase 1 (database schema)
3. Implement basic traversal for one language (Rust)
4. Expand to other languages
5. Add queries and analysis tools