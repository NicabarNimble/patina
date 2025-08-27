# Scrape Language-Aware Modularization Plan

## Context
After a failed attempt to modularize the 2000+ line monolithic `scrape.rs` that lost critical language-specific extraction logic, we're taking a more careful, trait-based approach that preserves each language's unique requirements.

## Problem Analysis

### What Went Wrong Previously
- Generic extraction using `child_by_field_name("name")` only works for Rust
- Lost language-specific visibility rules (Go uppercase, Python underscore)
- Lost language-specific AST node names and structures
- Resulted in 58% MORE functions and 11x MORE relations (incorrect behavior)

### Current State
- `src/commands/scrape.rs`: 2072 lines, monolithic but WORKING
- Supports: Rust, Go, Python, JavaScript/JSX, TypeScript/TSX, Solidity
- Each language has 100-200 lines of specific logic scattered throughout

## Proposed Architecture

### Core Trait System
```rust
pub trait LanguageExtractor: Send + Sync {
    // Core extraction entry point
    fn extract(&self, path: &Path, source: &str) -> Result<Extraction>;
    
    // Language-specific node interpretation
    fn get_node_name(&self, node: &Node, source: &str) -> Option<String>;
    fn is_public(&self, node: &Node, source: &str, name: &str) -> bool;
    fn extract_parameters(&self, node: &Node, source: &str) -> Vec<Parameter>;
    fn extract_return_type(&self, node: &Node, source: &str) -> ReturnInfo;
    fn extract_documentation(&self, node: &Node, source: &str) -> Option<Documentation>;
    
    // Language-specific node type queries
    fn is_function(&self, node: &Node) -> bool;
    fn is_struct(&self, node: &Node) -> bool;
    fn is_trait(&self, node: &Node) -> bool;
    fn is_import(&self, node: &Node) -> bool;
    fn is_call(&self, node: &Node) -> bool;
}
```

### Module Structure
```
src/semantic/
├── extractor/
│   ├── mod.rs           # Trait definition + shared utilities
│   ├── common.rs        # Shared extraction logic (tree walking, SQL)
│   ├── rust.rs          # RustExtractor impl
│   ├── go.rs            # GoExtractor impl  
│   ├── python.rs        # PythonExtractor impl
│   ├── javascript.rs    # JavaScriptExtractor impl (handles JS/JSX/TS/TSX)
│   └── solidity.rs      # SolidityExtractor impl
├── patterns/
│   ├── visibility.rs    # Language-specific visibility rules
│   ├── naming.rs        # Naming conventions per language
│   └── documentation.rs # Doc comment patterns
└── store/
    └── duckdb.rs        # Unified storage interface
```

## Language-Specific Requirements

### Rust
- **Visibility**: `pub`, `pub(crate)`, `pub(super)`, private by default
- **Doc Comments**: `///` for items, `//!` for modules
- **Async/Unsafe**: First-class keywords
- **Generics**: Type parameters and lifetimes
- **Special**: `impl` blocks, traits, macros

### Go
- **Visibility**: Uppercase = public, lowercase = private
- **Doc Comments**: `//` above declaration
- **Methods**: Receiver syntax `(r *Type)`
- **Special**: Multiple return values, error as second return
- **No generics**: (Until Go 1.18+, handle both)

### Python
- **Visibility**: `_` prefix = private, `__` = name mangling, default public
- **Doc Comments**: Docstrings as first statement
- **Async**: `async def` functions
- **Type Hints**: Optional but should extract if present
- **Special**: Decorators, class methods, static methods

### JavaScript/TypeScript
- **Visibility**: `export` keyword, class member modifiers (TS)
- **Doc Comments**: `/** */` JSDoc style
- **Async**: `async` functions, Promise returns
- **Modules**: ES6 imports/exports, CommonJS require
- **Special**: Arrow functions, React components (JSX/TSX)

### Solidity
- **Visibility**: `public`, `private`, `internal`, `external`
- **Doc Comments**: `///` NatSpec format
- **Special**: Modifiers, events, state variables
- **Contract-specific**: Constructor, fallback, receive functions

## Implementation Phases

### Phase 1: Foundation (Week 1)
1. Create trait definition in `src/semantic/extractor/mod.rs`
2. Extract common utilities to `src/semantic/extractor/common.rs`:
   - Tree traversal helpers
   - SQL generation utilities
   - Call graph construction
   - Generic documentation extraction
3. Set up test framework comparing against original output

### Phase 2: Rust Extractor (Week 1)
1. Implement `RustExtractor` as first language
2. Validate against current output for Patina itself
3. Ensure 100% parity with original for Rust files

### Phase 3: Go Extractor (Week 2)
1. Implement `GoExtractor` with uppercase visibility rules
2. Test against Dagger repository
3. Validate function counts match original

### Phase 4: Dynamic Languages (Week 2)
1. Implement `PythonExtractor` with underscore conventions
2. Implement `JavaScriptExtractor` handling all JS variants
3. Test against appropriate repositories

### Phase 5: Solidity (Week 3)
1. Implement `SolidityExtractor` with contract semantics
2. Test against smart contract repositories

### Phase 6: Integration (Week 3)
1. Wire up factory pattern in main scrape command
2. Performance testing and optimization
3. Documentation and cleanup

## Testing Strategy

### Comparison Testing
```bash
# Before refactor
patina scrape --force --repo dagger > before.sql

# After each phase
patina scrape --force --repo dagger > after.sql

# Compare outputs
diff before.sql after.sql
```

### Metrics to Validate
- Function count per language
- Call graph edge count
- Documentation extraction count
- Import/dependency counts
- Public/private classification accuracy

### Test Repositories
- **Rust**: patina (this repo)
- **Go**: dagger/dagger
- **Python**: Popular Python project
- **JavaScript**: React or Vue project
- **TypeScript**: TypeScript compiler
- **Solidity**: OpenZeppelin contracts

## Success Criteria

1. **Functional Parity**: Exact same extraction results as monolithic version
2. **Performance**: No more than 10% slower than original
3. **Maintainability**: Each language extractor < 500 lines
4. **Testability**: 90%+ test coverage per extractor
5. **Extensibility**: Adding new language requires only new extractor impl

## Risk Mitigation

1. **Keep original working**: Don't delete `scrape.rs` until fully validated
2. **Test continuously**: Run comparison tests after each commit
3. **Branch protection**: Work on feature branch, merge only when complete
4. **Incremental rollout**: Can ship with feature flag to toggle implementations

## Key Design Decisions

### Why Traits?
- Enforces consistent interface across languages
- Allows runtime polymorphism for language selection
- Enables testing of individual extractors
- Supports future plugin architecture

### Why Not Generic?
- Each language has fundamentally different AST structure
- Generic extraction loses critical semantic information
- Language-specific rules are features, not implementation details

### Shared vs Specific
**Share**:
- Tree walking algorithms
- Database schema and insertion
- File I/O and parsing setup
- Progress reporting

**Don't Share**:
- Node name extraction
- Visibility determination
- Parameter/return type parsing
- Documentation format parsing

## Next Steps

1. Create trait definition
2. Start with `common.rs` extraction
3. Implement `RustExtractor` first
4. Set up comparison testing framework
5. Proceed language by language with validation

## Notes

- This plan prioritizes correctness over abstraction
- Each language's quirks are treated as first-class requirements
- The monolithic version serves as the reference implementation
- We can always optimize for code reuse after achieving parity