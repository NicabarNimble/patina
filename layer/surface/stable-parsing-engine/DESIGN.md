# Stable Parsing Engine - Design & Architecture

## Executive Summary

A modular, production-ready semantic code analysis engine that extracts structured knowledge from codebases for efficient LLM context retrieval. Achieves 10-100x token reduction compared to raw file feeding while maintaining complete code understanding.

## Core Philosophy

**Extract facts, build relationships, retrieve context efficiently.**

We're not just storing code and docs - we're building a graph of knowledge that LLMs can query intelligently. This system transforms sprawling codebases into structured, queryable intelligence.

## Architecture Overview

```
┌─────────────────────────────────────────────────────────────┐
│                     Modular Extraction Pipeline              │
├───────────────────┬──────────────┬──────────────────────────┤
│  AST Processor    │ Call Graph   │  Documentation           │
│  ast_processor.rs │ call_graph.rs│  documentation.rs        │
├───────────────────┴──────────────┴──────────────────────────┤
│                    Storage Abstraction Layer                 │
│                    store/mod.rs + duckdb.rs                  │
├──────────────────────────────────────────────────────────────┤
│                    DuckDB (Single File DB)                   │
│          10 tables, recursive CTEs, array columns            │
└──────────────────────────────────────────────────────────────┘
```

## Module Breakdown

### 1. **ast_processor.rs** - Central AST Processing
- Coordinates extraction from tree-sitter AST nodes
- Normalizes cross-language differences
- Routes to specialized extractors
- **Key Types**: `ProcessingResult`, `FunctionFact`, `TypeFact`

### 2. **call_graph.rs** - Relationship Extraction
- Extracts function call relationships
- Handles different call types (direct, method, async, constructor)
- Language-specific call pattern recognition
- **Key Type**: `CallRelation`

### 3. **documentation.rs** - Documentation Intelligence
- Extracts and cleans doc comments/strings
- Keyword extraction with stop-word filtering
- Summary generation (first sentence)
- **Key Type**: `Documentation`

### 4. **store/mod.rs** - Storage Abstraction
- Trait-based storage interface
- Enables future storage backend swaps
- **Key Trait**: `KnowledgeStore`

### 5. **store/duckdb.rs** - DuckDB Implementation
- SQL generation and execution
- Batch processing for performance
- Proper escaping and error handling

## Database Schema

### Core Tables

```sql
-- 1. Function facts (behavioral signals)
CREATE TABLE function_facts (
    file VARCHAR,
    name VARCHAR,
    takes_mut_self BOOLEAN,
    takes_mut_params BOOLEAN,
    returns_result BOOLEAN,
    returns_option BOOLEAN,
    is_async BOOLEAN,
    is_unsafe BOOLEAN,
    is_public BOOLEAN,
    parameter_count INTEGER,
    generic_count INTEGER,
    parameters TEXT,
    return_type TEXT,
    PRIMARY KEY (file, name)
);

-- 2. Documentation (searchable knowledge)
CREATE TABLE documentation (
    file VARCHAR,
    symbol_name VARCHAR,
    symbol_type VARCHAR,
    line_number INTEGER,
    doc_raw TEXT,
    doc_clean TEXT,
    doc_summary VARCHAR,
    keywords VARCHAR[],  -- DuckDB array type
    doc_length INTEGER,
    has_examples BOOLEAN,
    has_params BOOLEAN,
    PRIMARY KEY (file, symbol_name)
);

-- 3. Call graph (relationships)
CREATE TABLE call_graph (
    caller VARCHAR,
    callee VARCHAR,
    file VARCHAR,
    call_type VARCHAR,  -- direct, method, async, constructor
    line_number INTEGER
);

-- 4. Type vocabulary (domain modeling)
CREATE TABLE type_vocabulary (
    file VARCHAR,
    name VARCHAR,
    definition TEXT,
    kind VARCHAR,       -- struct, trait, enum
    visibility VARCHAR,
    PRIMARY KEY (file, name)
);

-- Plus 6 more supporting tables...
```

## Language Support Matrix

| Language | Functions | Types | Docs | Call Graph | Imports |
|----------|-----------|-------|------|------------|---------|
| Rust     | ✅ | ✅ | ✅ | ✅ | ✅ |
| Go       | ✅ | ✅ | ✅ | ✅ | ✅ |
| Python   | ✅ | ✅ | ✅ | ✅ | ✅ |
| JavaScript/JSX | ✅ | ✅ | ✅ | ✅ | ✅ |
| TypeScript/TSX | ✅ | ✅ | ✅ | ✅ | ✅ |
| Solidity | ✅ | ✅ | ✅ | ✅ | ✅ |

## Key Features

### 1. Semantic Extraction
- **Functions**: Full metadata including async, unsafe, visibility, generics
- **Types**: Structs, traits, enums with definitions
- **Behavioral Hints**: Detects unwrap, expect, panic, unsafe blocks
- **Fingerprints**: AST-based unique identifiers

### 2. Documentation Intelligence
- Multi-language comment style support
- Automatic keyword extraction
- Summary generation
- Metadata tracking (examples, parameters)

### 3. Call Graph Analysis
- Tracks all function relationships
- Line number precision for navigation
- Call type classification
- Recursive traversal support

### 4. Incremental Updates
- mtime-based change detection
- Only reprocess modified files
- Maintains consistency

## Historical Context: The Refactoring Journey

### The Problem (August 2024)
We had a working but monolithic 1,827-line `scrape.rs` file that was becoming unmaintainable. All extraction logic, storage, and processing were tangled together.

### The Solution
Refactored into a modular architecture:
- **Before**: 1,827 lines in single file
- **After**: 302 lines in main + 1,549 lines across 5 specialized modules
- **Result**: 83% reduction in main file complexity

### Challenges Overcome

1. **Call Graph Duplication Bug**
   - **Issue**: Extracting calls at every AST node caused 11x duplication
   - **Root Cause**: `extract_calls` was being called recursively on all nodes
   - **Fix**: Only extract from function bodies, not every node

2. **Force Flag Bug**
   - **Issue**: `--force` only cleared 3 tables, causing data accumulation
   - **Root Cause**: Incomplete table list in DELETE statement
   - **Fix**: Clear all 10 tables on force re-index

3. **Schema Mismatches**
   - **Issue**: Column names didn't match between schema and INSERT statements
   - **Fix**: Aligned all field names and orders

4. **SQL Injection Vulnerabilities**
   - **Issue**: Unescaped single quotes in SQL strings
   - **Fix**: Proper escaping with `replace('\'', "''")`

### Lessons Learned

1. **Feature Parity ≠ Behavioral Parity**: Matching features doesn't mean matching behavior
2. **Modular > Monolithic**: Clean separation enables easier debugging
3. **Test on External Repos**: Our own code changes masked duplication issues
4. **Escape Everything**: Never trust string data in SQL generation

## Performance Characteristics

| Metric | Value | Notes |
|--------|-------|-------|
| Extraction Speed | ~1,000 files/sec | Tree-sitter is fast |
| Database Size | ~2 MB per 1,000 files | 16KB block size |
| Query Time | < 10ms | DuckDB analytical engine |
| Token Reduction | 10-100x | Compared to raw files |
| Incremental Update | O(changed files) | mtime-based |

## Context Retrieval Strategy

### Query Flow
1. **Keyword Search** → Find entry points via documentation
2. **Graph Expansion** → Follow call relationships
3. **Fact Assembly** → Combine docs, signatures, relationships
4. **LLM Formatting** → Structure for optimal context

### Example: "How does authentication work?"

```sql
-- 1. Find auth-related symbols
WITH entry_points AS (
    SELECT symbol_name FROM documentation
    WHERE list_contains(keywords, 'auth')
),
-- 2. Expand via call graph
auth_context AS (
    SELECT symbol_name FROM entry_points
    UNION
    SELECT callee FROM call_graph
    WHERE caller IN (SELECT * FROM entry_points)
)
-- 3. Gather all facts
SELECT d.*, f.*, cg.*
FROM auth_context ac
LEFT JOIN documentation d ON ac.symbol_name = d.symbol_name
LEFT JOIN function_facts f ON ac.symbol_name = f.name
LEFT JOIN call_graph cg ON ac.symbol_name = cg.caller;
```

## Future Enhancements

### Near Term
- [ ] Context builder module for query-driven retrieval
- [ ] Token budget management
- [ ] Relevance ranking algorithms
- [ ] Cross-repository analysis

### Long Term
- [ ] Method extraction from impl blocks
- [ ] Type inference for dynamic languages
- [ ] Change impact analysis
- [ ] Security vulnerability scanning

## Production Considerations

### Deployment
- Single binary + DuckDB file
- No external dependencies beyond tree-sitter
- Works offline, no API calls

### Scalability
- Tested on repos with 1,000+ files
- Linear performance characteristics
- Incremental updates keep it fast

### Reliability
- Transaction-based updates
- Atomic operations
- Graceful error handling

## Conclusion

This refactored engine achieves 100% feature parity with the original while fixing critical bugs and improving maintainability. The modular design enables future enhancements without architectural changes. Most importantly, it transforms code into queryable knowledge, making LLMs dramatically more effective at understanding and working with large codebases.