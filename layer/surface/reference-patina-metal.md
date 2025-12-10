---
id: reference-patina-metal
status: active
created: 2025-08-24
updated: 2025-12-09
oxidizer: nicabar
tags: [reference, parser, tree-sitter, metal, guide]
references: [architecture-patina-metal]
---

# Patina Metal: Unified Parser Architecture

## Core Concept

**Patina Metal** is a unified parser subsystem that handles multi-language code analysis through tree-sitter. Different programming languages are treated as different "metals" - each with unique properties but processed through a common interface.

## The Metal Metaphor

```rust
pub enum Metal {
    Rust,     // Ferrous - stable, oxidizes predictably
    Go,       // Copper - conducts well, green patina
    Solidity, // Precious - immutable, doesn't tarnish
    Python,   // Alloy - flexible, composite properties
    JavaScript, // Mercury - fluid, adaptable
    TypeScript, // Steel - structured JavaScript
}
```

## Architecture

```
patina-metal/
├── src/
│   ├── lib.rs         # Unified Analyzer API
│   ├── metal.rs       # Language enum
│   ├── grammars.rs    # FFI bindings to C parsers
│   └── parser.rs      # Parser wrapper
├── grammars/          # Git submodules (exact versions)
│   ├── rust/         
│   ├── go/           
│   ├── solidity/     
│   ├── python/       
│   ├── javascript/   
│   └── typescript/   
└── build.rs           # Compiles C parsers with cc crate
```

## Key Features

### 1. Self-Built Parsers
- Compiles tree-sitter grammars from source via Git submodules
- Complete control over versions - no crates.io dependency conflicts
- All parsers use tree-sitter 0.24 consistently

### 2. Unified API
```rust
let mut analyzer = Analyzer::new()?;
let parsed = analyzer.parse(source_code, Metal::Solidity)?;
let symbols = analyzer.extract_symbols(&parsed);
let complexity = analyzer.calculate_complexity(&parsed);
let fingerprint = analyzer.generate_fingerprint(node, source);
```

### 3. Fact Extraction (Not Interpretation)
Extracts only verifiable facts from code:
- Function signatures, parameters, return types
- Type definitions and constants
- Module structure and visibility
- Complexity metrics
- Git history correlation

## Current Capabilities

### Language Support
| Language | Status | Files Tested | Symbols Extracted |
|----------|--------|--------------|-------------------|
| Rust | ✅ Full | 151 | 3,500+ |
| Go | ✅ Full | 832 | 6,420+ |
| Solidity | ✅ Full | 209 | 1,200+ |
| Python | ✅ Full | 200+ | 1,328+ |
| JavaScript | ✅ Full | 100+ | 212+ |
| TypeScript | ✅ Full | 150+ | 513+ |

### Extraction Features
- **Functions**: Name, parameters, return type, complexity, visibility
- **Types**: Structs, enums, type aliases, interfaces
- **Constants**: Values, types, visibility
- **Imports**: Dependencies, re-exports
- **Behavioral Hints**: Unsafe blocks, panics, TODOs

### Performance
- Incremental indexing: Only processes changed files
- Handles 1000+ file repositories without timeout
- Respects `.gitignore` and `.ignore` patterns
- Parallel processing capability

## Token-Efficient Context for LLMs

### The Mission
Provide LLMs with compressed, factual context about codebases without requiring them to read entire files.

### Example Efficiency
```sql
-- Instead of reading 5000 tokens of code:
-- Query returns 5 tokens:
SELECT name, parameters, return_type 
FROM functions 
WHERE name = 'connect';
-- Result: "connect | async Result<Connection> | retry-pattern"
```

### Three-Tier Intelligence
1. **DATABASE** - Fast structured lookups (patina-metal extracts)
2. **CODE FILES** - Deep dive when needed (actual source)
3. **WEB** - External validation (docs, issues)

## Design Principles

1. **Facts Over Interpretations**: Extract what IS, not what claims to be
2. **Language Agnostic**: Same API regardless of language
3. **Version Control**: Git submodules lock exact grammar versions
4. **Escape Hatches**: Never lock users into specific versions
5. **Token Efficiency**: Every byte extracted saves 100x in LLM context

## Usage in Patina

### Scrape Command Integration
```bash
patina scrape                    # Current project
patina scrape --repo=dagger      # External repo
patina scrape --force            # Full re-index
```

### Database Schema
```sql
-- Core tables populated by patina-metal
code_fingerprints   -- Function signatures and complexity
type_vocabulary     -- Type definitions  
import_facts        -- Module dependencies
behavioral_hints    -- Safety and error patterns
git_metrics        -- Change frequency and importance
```

## Technical Implementation

### Build Process
1. `build.rs` compiles C parsers from grammar submodules
2. FFI bindings connect Rust to compiled parsers
3. Unified `Metal` enum abstracts language differences
4. StreamingIterator API for efficient AST traversal

### Grammar Handling
- Each language grammar as Git submodule
- Pinned to specific compatible versions
- Custom node mappings for language differences
- Example: Solidity parameters are children, not fields

## Future Enhancements

### Near Term
- Cairo support (pending stable grammar)
- Enhanced parameter type extraction
- Call graph analysis
- Error type cataloging

### Long Term
- Cross-language pattern detection
- Semantic diff capabilities
- Doc drift detection
- API compatibility checking

## Value Proposition

**For Developers**: Navigate unfamiliar codebases quickly with factual summaries

**For LLMs**: Understand massive codebases in minimal tokens

**For Teams**: Track code evolution and pattern emergence over time

**For Documentation**: Verify claims against actual implementation

## Summary

Patina Metal solves the multi-language parsing problem by building parsers from source, providing a unified interface, and focusing on fact extraction rather than interpretation. It's the foundation that enables Patina to be a token-efficient memory system for AI-assisted development.