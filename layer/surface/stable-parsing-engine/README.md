# Stable Parsing Engine

A production-ready semantic code analysis engine that transforms codebases into queryable knowledge for LLMs.

## Status: ✅ Production Ready

The core extraction engine is complete with 100% feature parity plus critical bug fixes from the original implementation.

## Quick Overview

- **Purpose**: Extract structured knowledge from code for efficient LLM context retrieval
- **Performance**: 10-100x token reduction vs raw file feeding
- **Languages**: Rust, Go, Python, JavaScript/TypeScript, Solidity
- **Storage**: DuckDB with 10 specialized tables
- **Architecture**: Modular design with 5 specialized components

## Key Files

- **[DESIGN.md](DESIGN.md)** - Complete architecture, schema, and implementation details
- **[TODO.md](TODO.md)** - Current status and future roadmap

## What It Does

1. **Extracts** semantic information from code:
   - Functions with full metadata (async, unsafe, generics, etc.)
   - Types (structs, traits, enums)
   - Documentation with searchable keywords
   - Call graphs with line numbers
   - Behavioral hints (unwrap, panic, unsafe blocks)

2. **Stores** in queryable format:
   - DuckDB with array columns for keyword search
   - Recursive CTEs for graph traversal
   - Incremental updates for performance

3. **Enables** intelligent context retrieval:
   - Find code by documentation keywords
   - Follow call relationships
   - Assemble complete context for LLMs

## Usage

```bash
# Initialize database
patina scrape --init

# Extract knowledge from codebase
patina scrape

# Query the knowledge base
patina scrape --query "SELECT * FROM documentation WHERE list_contains(keywords, 'auth')"

# Scrape external repository
patina scrape --repo dagger

# Force complete re-index
patina scrape --force
```

## Historical Note

This engine is the result of a successful refactoring from a monolithic 1,827-line file into a modular architecture. The refactoring fixed critical bugs including:
- 11x call graph duplication
- Incomplete force flag behavior
- SQL injection vulnerabilities

See [DESIGN.md](DESIGN.md#historical-context-the-refactoring-journey) for the complete story.

## Next Steps

The extraction engine is complete. Next phase focuses on context retrieval:
- Query interface for answering questions
- LLM-specific formatters
- Token budget management

See [TODO.md](TODO.md#next-phase-context-retrieval-system-) for details.