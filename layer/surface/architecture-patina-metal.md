---
id: architecture-patina-metal
status: active
created: 2025-08-22
updated: 2025-12-09
oxidizer: nicabar
tags: [architecture, parser, tree-sitter, multi-language, metal]
references: [reference-patina-metal, architecture-patina-system]
---

# Patina Metal: Unified Language Parser Architecture

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
scrape command → SQLite
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

## Solution Implemented (2025-08-22 Sessions #2-3)

### The Fix: Self-Built Parsers via Git Submodules
We implemented the original vision of patina-metal: **building parsers from source** instead of using crates.io packages.

#### Implementation Details:

**Architecture:**
```
patina-metal/
├── src/
│   ├── lib.rs         # Main analyzer with streaming iterator support
│   ├── metal.rs       # Metal enum (Rust, Go, Solidity, Cairo)
│   ├── grammars.rs    # FFI bindings to compiled C parsers
│   ├── parser.rs      # Parser wrapper
│   └── queries.rs     # Tree-sitter query loader
├── grammars/          # Git submodules
│   ├── rust/         → tree-sitter-rust v0.23.0
│   ├── go/           → tree-sitter-go v0.23.1
│   └── solidity/     → tree-sitter-solidity v1.2.3
├── queries/           # Pattern matching queries
│   ├── rust/
│   │   ├── symbols.scm
│   │   ├── complexity.scm
│   │   └── patterns.scm
│   └── go/
│       └── ...
└── build.rs           # Compiles C parsers with cc crate
```

**Key Changes:**
1. **Git Submodules**: Added rust, go, solidity grammars as submodules
2. **Custom Build**: `build.rs` compiles parser.c files directly using `cc` crate
3. **FFI Bindings**: Direct `extern "C"` bindings to tree_sitter_rust(), etc.
4. **Version Control**: Pinned to language version 14 (compatible with tree-sitter 0.24)
5. **Streaming Iterator**: Updated to use tree-sitter 0.24's StreamingIterator API
6. **Clean Dependencies**: Only depends on tree-sitter 0.24, no parser crates

#### Current Status:
✅ **Rust Parser**: Fully working, 151 files indexed successfully
✅ **Go Parser**: Fully working, 832 files in Dagger repo processed
✅ **Solidity Parser**: Compiles and links, 209 files detected
✅ **Scrape Integration**: patina-metal is the sole parser provider
✅ **Performance**: Dagger repo (6,420 functions) indexes without timeout
✅ **Version Stability**: No conflicts, single tree-sitter 0.24 everywhere
⚠️  **Cairo**: Not implemented (no stable tree-sitter grammar yet)

## Additional Improvements (Session #3)

### Incremental Updates System
Implemented smart incremental indexing to avoid re-processing unchanged files:

**Features:**
- `index_state` table tracks file paths, mtimes, and index timestamps
- Change detection compares filesystem mtimes with stored values
- Only processes new/modified files, skips unchanged
- `--force` flag available for full re-index when needed
- Cleanup removes only changed file data before re-indexing

**Performance Impact:**
- First run: Full index of all files
- No changes: Instant return (< 1 second)
- Single file change: Only that file re-processed
- Massive time savings for large codebases

## Next Steps

### Completed ✅
- ✅ Git submodules for grammar repositories
- ✅ Self-built parsers from source
- ✅ Version conflict resolution
- ✅ Incremental update system
- ✅ Rust and Go parser integration
- ✅ Basic Solidity support

### Future Improvements
1. **Query System Enhancement**: Fix .scm query syntax for complex patterns
2. **Cairo Support**: Wait for stable tree-sitter-cairo or use cairo-lang-parser
3. **More Languages**: Add Python, JavaScript, TypeScript parsers
4. **Query Documentation**: Document tree-sitter query patterns and usage
5. **Parser Updates**: Automate grammar version updates with compatibility checks
6. **Performance**: Add parallel parsing for multiple files

## Technical Achievements

### What We Solved
1. **Version Hell**: Single tree-sitter 0.24 everywhere, no conflicts
2. **Parser Control**: We compile from source, not dependent on crates.io
3. **Clean Architecture**: patina-metal owns all parser complexity
4. **Performance**: Handles 1000+ file repos without timeout
5. **Incremental Updates**: Smart caching based on file modification times

### How It Works
1. **Build Time**: `cc` crate compiles C parsers from grammar submodules
2. **Runtime**: FFI calls to compiled parsers via `extern "C"`
3. **API**: Uniform `Metal` enum abstracts language differences
4. **Integration**: Main patina uses patina-metal exclusively for parsing

## Conclusion

`patina-metal` successfully achieves its vision: a unified parser system that builds from source, giving us complete control over tree-sitter versions and parser compatibility. By treating languages as different "metals" with unique properties, we maintain the Patina metaphor while providing a robust, extensible foundation for multi-language code analysis.

The system now handles Rust, Go, and Solidity code reliably, with incremental updates making it practical for continuous use during development. The architecture is clean, maintainable, and ready for expansion to additional languages.