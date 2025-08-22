# Patina Metal: Unified Language Parser Architecture

Created: 2025-08-22
Status: In Development
Branch: `patina-metal-parser`

## Problem Statement

We need to analyze code across multiple languages (Rust, Go, Solidity, Cairo) to extract semantic patterns, but:
- Tree-sitter versions conflict between language parsers
- Each language has different AST node names for similar concepts
- Crates.io packages often lag behind their GitHub sources
- Managing multiple parser dependencies creates version hell

## Solution: patina-metal

A unified parser subsystem that:
1. **Centralizes all tree-sitter complexity** in one workspace member
2. **Provides a clean, uniform API** regardless of underlying language
3. **Handles version conflicts** by controlling the build process
4. **Maps language-specific ASTs** to common concepts

## Architecture

```
patina (main crate)
    ↓ depends on
patina-metal (parser crate)
    ↓ wraps
tree-sitter-{rust,go,solidity,cairo}
    ↓ generates
Unified AST + Metadata
    ↓ consumed by
scrape command → DuckDB
```

### Core Abstractions

```rust
pub enum Metal {
    Rust,     // Ferrous - stable, oxidizes predictably
    Go,       // Copper - conducts well, green patina
    Solidity, // Precious - immutable, doesn't tarnish
    Cairo,    // Rare earth - exotic properties
}

pub struct Analyzer {
    parsers: HashMap<Metal, Parser>,
    queries: HashMap<(Metal, QueryType), Query>,
}
```

## Why "Metal"?

Fits the Patina metaphor perfectly:
- Different metals (languages) oxidize differently
- Each metal has unique properties (language features)
- Scraping metal reveals patterns underneath
- Patina forms on metal surfaces over time

## Implementation Strategy

### Phase 1: Parser Consolidation ✅
- Create `patina-metal` workspace member
- Move all tree-sitter dependencies there
- Build unified `Metal` enum and `Analyzer` API

### Phase 2: Language Normalization ✅
- Map language-specific node types to common categories
  - `function_item` (Rust) → `function`
  - `function_declaration` (Go) → `function`
  - `contract_declaration` (Solidity) → `struct`
- Normalize complexity calculation across languages

### Phase 3: Query System (In Progress)
- Add `.scm` query files for pattern matching
- Support tree-sitter's powerful query syntax
- Enable language-specific and cross-language queries

### Phase 4: Git Submodules (Planned)
```bash
patina-metal/
├── metals/           # Git submodules
│   ├── rust/        # → github.com/tree-sitter/tree-sitter-rust
│   ├── go/          # → github.com/tree-sitter/tree-sitter-go
│   ├── solidity/    # → github.com/JoranHonig/tree-sitter-solidity
│   └── cairo/       # → github.com/JoranHonig/tree-sitter-cairo
└── queries/         # .scm files
    ├── rust/
    │   ├── symbols.scm
    │   ├── complexity.scm
    │   └── patterns.scm
    └── solidity/
        ├── contracts.scm
        └── security.scm
```

## Technical Decisions

### Why Workspace Member?
- **Isolation**: Parser complexity doesn't leak into main crate
- **Parallel compilation**: Speeds up builds
- **Clear boundaries**: Easy to test and reason about
- **Future flexibility**: Could become standalone crate

### Why Not Use Language-Specific Parsers?
- **Uniform interface**: Same API for all languages
- **Incremental parsing**: Tree-sitter's killer feature
- **Query system**: .scm files enable powerful pattern matching
- **Battle-tested**: Used by GitHub, Neovim, Helix

### Version Conflict Resolution
1. **Use Git dependencies** when crates.io is stale
2. **Control tree-sitter version** in one place
3. **Build from source** via submodules if needed
4. **Pin versions** for reproducibility

## Usage Example

```rust
use patina_metal::{Analyzer, Metal};

let mut analyzer = Analyzer::new()?;

// Parse any language with same API
let parsed = analyzer.parse(source_code, Metal::Solidity)?;

// Extract symbols uniformly
let symbols = analyzer.extract_symbols(&parsed);

// Calculate complexity consistently
let complexity = analyzer.calculate_complexity(&parsed);

// Generate fingerprint for pattern matching
let fingerprint = analyzer.generate_fingerprint(node, source);
```

## Current Status

### Working ✅
- Rust parsing and analysis
- Go parsing and analysis (tested on Dagger: 7,201 symbols)
- Unified Metal enum abstraction
- Symbol extraction and complexity calculation
- AST fingerprinting for pattern detection
- Tree-sitter query system with .scm files (with some syntax issues)
- Graceful handling of parser failures

### In Progress 🔄
- Solidity parser (version conflicts with tree-sitter-solidity v15 vs expected v13-14)
- Cairo parser (missing LANGUAGE export)

### Planned 📋
- Python, JavaScript, TypeScript support
- Custom query builder
- Cross-language pattern detection
- Incremental parsing optimization
- Git submodules for exact grammar versions

## Integration Issues (2024-08-22 Session)

### The Problem
After integrating patina-metal as a parser provider for the scrape command, we encountered performance issues where Dagger repo (750 Go + 53 Rust files) times out during processing. This worked fine before the integration.

### Root Cause: Version Conflict Hell
1. **Original Patina**: Used tree-sitter 0.24 with tree-sitter-rust/go 0.23
2. **patina-metal**: Uses tree-sitter 0.23 (required by tree-sitter-rust/go crates)  
3. **Current State**: Trying to use both versions causes Cargo "links" conflict
   - Both versions try to link the same native C library
   - Cargo refuses to build with conflicting tree-sitter versions

### What We Changed
- Modified `create_parser()` in `src/semantic/languages.rs` to use patina-metal
- Added error handling in scrape to skip languages without working parsers
- Kept original fingerprinting and database logic intact (good decision)

### Current Blockers
1. **Version Mismatch**: Can't have tree-sitter 0.24 and 0.23 in same project
2. **Dependency Chain**: 
   - Main patina needs tree-sitter for semantic module
   - patina-metal needs different tree-sitter version
   - Can't satisfy both requirements
3. **Performance**: Even with reverted code, Dagger scrape times out (suggests version issue)

### Lessons Learned
- The patina-metal abstraction is good for managing parsers
- Should have been a drop-in replacement, not a rewrite
- Original fingerprinting/database code was already efficient
- Version conflicts are the core problem we're solving

## Benefits

1. **Maintainability**: All parser logic in one place
2. **Extensibility**: Adding languages is straightforward
3. **Performance**: Native C parsers, no overhead
4. **Consistency**: Same analysis interface for all languages
5. **Version Control**: Submodules lock exact grammar versions

## Future Vision

`patina-metal` becomes the foundation for:
- **Cross-language pattern detection**: Find similar patterns across Rust/Go/Solidity
- **Multi-language refactoring**: Apply transformations uniformly
- **Language migration assistance**: Map concepts between languages
- **Semantic code search**: Query by structure, not syntax

## Testing Strategy

```bash
# Test each metal individually
cargo test -p patina-metal --features rust
cargo test -p patina-metal --features go
cargo test -p patina-metal --features solidity

# Integration test with scrape
patina scrape --repo=dagger  # Go
patina scrape --repo=dust    # Solidity
patina scrape               # Rust (patina itself)
```

## Known Issues

1. **tree-sitter-solidity**: Crates.io version has wrong ABI version
   - Solution: Use Git dependency or rebuild from source
   
2. **tree-sitter-cairo**: Missing LANGUAGE export
   - Solution: Fork and fix, or use cairo-lang-parser

3. **Version conflicts**: Different parsers want different tree-sitter versions
   - Solution: Standardize on tree-sitter 0.23 for now

## Next Steps

### Immediate (Fix Integration)
1. **Resolve version conflict**: Either:
   - Fully remove tree-sitter deps from main Cargo.toml and refactor semantic module
   - OR upgrade patina-metal to tree-sitter 0.24 (may break parsers)
   - OR re-export tree-sitter types from patina-metal for semantic module to use
2. **Test performance**: Ensure Dagger scrape works as fast as before
3. **Clean up**: Remove duplicate tree-sitter dependencies

### Future Improvements
1. Fix Solidity parser integration (needs tree-sitter version compatibility)
2. Add Cairo support (either tree-sitter or cairo-lang-parser)
3. Fix .scm query syntax issues for complex queries
4. Set up Git submodules for grammar repositories
5. Add Python, JavaScript, TypeScript support
6. Document query syntax and patterns

## Conclusion

`patina-metal` provides a clean abstraction over the complexity of multi-language parsing. By treating languages as different "metals" with unique properties, we maintain the Patina metaphor while building a powerful, extensible system for code analysis across any language ecosystem.