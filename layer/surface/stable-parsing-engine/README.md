# Stable Parsing Engine

A semantic code analysis engine that gives LLMs a "senior developer's understanding" of your codebase.

## Quick Start

```bash
# Initialize and extract knowledge
patina scrape --init
patina scrape

# Query the knowledge base (coming soon)
patina context "How does authentication work?"
```

## What It Does

**Problem**: LLMs waste thousands of tokens reading entire files for simple questions.

**Solution**: Extract semantic meaning and relationships, store in a queryable database.

**Result**: 10-100x token reduction with better code understanding.

## Architecture

```
Your Code → Tree-sitter → Semantic Extraction → DuckDB → LLM Context
```

- **6 Languages**: Rust, Go, Python, JavaScript/TypeScript, Solidity
- **10 Tables**: Functions, types, docs, call graphs, imports, behaviors
- **Performance**: ~1,000 files/sec extraction, <10ms queries

## Documentation

- **[DESIGN.md](DESIGN.md)** - Complete technical design and roadmap

## Status

- ✅ **Extraction Engine** - Production ready
- 🚧 **Context Retrieval** - In development
- 📋 **LLM Formatters** - Planned