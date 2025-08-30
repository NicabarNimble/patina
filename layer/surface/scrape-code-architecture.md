# Scrape Code Architecture

## What It Does

The `patina scrape code` command builds a semantic knowledge database from source code. It parses code using tree-sitter, extracts semantic information, analyzes git history, and stores everything in a DuckDB database at `.patina/knowledge.db`.

## Core Design: Language Registry Pattern

All language-specific logic lives in a centralized registry. Each language has a single `LanguageSpec` struct that defines:

```rust
struct LanguageSpec {
    extensions: &'static [&'static str],           // File extensions
    function_nodes: &'static [&'static str],       // AST nodes for functions
    struct_nodes: &'static [&'static str],         // AST nodes for structs
    trait_nodes: &'static [&'static str],          // AST nodes for interfaces
    import_nodes: &'static [&'static str],         // AST nodes for imports
    
    // Parsing functions
    is_doc_comment: fn(&str) -> bool,
    parse_visibility: fn(&Node, &str, &[u8]) -> bool,
    has_async: fn(&Node, &[u8]) -> bool,
    has_unsafe: fn(&Node, &[u8]) -> bool,
    
    // Extraction functions  
    extract_params: fn(&Node, &[u8]) -> Vec<String>,
    extract_return_type: fn(&Node, &[u8]) -> Option<String>,
    extract_generics: fn(&Node, &[u8]) -> Option<String>,
    get_symbol_kind: fn(&str) -> &'static str,
    get_symbol_kind_complex: fn(&Node, &[u8]) -> Option<&'static str>,
    extract_call_target: fn(&Node, &[u8]) -> Option<String>,
}
```

## Supported Languages

8 languages via tree-sitter parsers:
- Rust
- Go  
- Python
- JavaScript
- TypeScript
- JSX (JavaScript JSX)
- TSX (TypeScript TSX)
- Solidity

## Database Schema

The ETL pipeline populates these tables:

- **code_fingerprints**: AST patterns, complexity metrics, symbol locations
- **function_facts**: Behavioral signals (async, unsafe, mutability, error handling)
- **git_metrics**: Code age, change frequency, authorship, survival rates
- **call_graph**: Function dependencies and call relationships
- **documentation**: Extracted doc comments with searchable keywords
- **pattern_references**: References to layer/ patterns found in code

## Architecture Decisions

### Why a Monolithic File (3166 lines)

Previous attempts to modularize failed because:
1. The pipeline stages are tightly coupled - separation added complexity without benefit
2. Passing context between modules created boilerplate
3. Performance suffered from abstraction overhead
4. The "brutal monolith" is easier to understand and maintain

### Why the Registry Pattern

Before: 19+ scattered match statements for language-specific logic
After: Single registry with all logic centralized

Benefits:
- Adding a language requires touching 2 places (create spec, register it)
- Compiler enforces completeness - missing fields = compilation error
- Language logic is testable in isolation
- Clear separation between data (specs) and behavior (pipeline)

### Why Some Match Statements Remain

About 10 match statements weren't moved to the registry because they:
- Perform side effects (adding to context, database writes)
- Implement fundamentally different algorithms per language
- Would require passing mutable context into specs
- Represent genuine behavioral differences, not just data

This follows the 80/20 rule - we got 80% of the benefit without the complexity.

### Why DuckDB Instead of SQLite

DuckDB provides:
- Columnar storage optimized for analytics
- Better query performance for the types of queries we run
- Native JSON support for complex data
- Smaller file sizes with better compression

### Why Incremental Updates

The scraper tracks:
- File modification times
- Git commit hashes
- Previous scan timestamps

This allows skipping unchanged files, making re-scans fast.

## File Organization

The code is structured in logical chapters:

1. **Language Registry** - All language specifications
2. **Public Interface** - initialize() and extract() entry points
3. **ETL Orchestration** - Main pipeline control flow
4. **Git Metrics** - Repository analysis
5. **Pattern References** - Scanning for layer/ patterns
6. **Semantic Extraction** - Core parsing logic
7. **Database Operations** - Schema and data loading
8. **AST Processing** - Tree-sitter utilities
9. **Utilities** - Helpers and common functions
10. **Languages Module** - Language enum and detection

## Performance Characteristics

Typical performance on a medium codebase:
- ~800 files processed in 2-3 seconds
- Database size: 2-4 MB
- Incremental updates: <1 second
- Memory usage: ~50-100 MB

The pipeline is I/O bound, not CPU bound. Most time is spent reading files and writing to the database.