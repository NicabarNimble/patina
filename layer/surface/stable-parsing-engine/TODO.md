# Stable Parsing Engine - TODO

## Phase 1: Foundation (Vendor & Pin) ✅ IN PROGRESS
- [x] Create `grammar-pack.toml` configuration
- [x] Add metadata.toml for Rust grammar version tracking
- [x] Build system already compiles C via cc crate
- [x] Record grammar commits in binary via build.rs
- [ ] Vendor remaining grammars (go, python, js, ts, solidity)
- [ ] Remove git submodule references from .gitmodules
- [ ] Create `Lang` trait abstraction
- [ ] Implement basic file cache table (path, blob_sha, grammar_commit)

## Phase 2: Query System
- [ ] Co-locate queries with vendored grammars
- [ ] Define stable capture set (@decl, @name, @doc, @export)
- [ ] Version queries with grammar commits
- [ ] Create query contract tests
- [ ] Build golden corpus test suite per language

## Phase 3: Incremental Indexing (HIGH PRIORITY)
- [ ] Implement git blob SHA retrieval (`git hash-object`)
- [ ] Build FileCache with blob_sha tracking
- [ ] Create "changed files only" reindex algorithm
- [ ] Add grammar_commit change detection
- [ ] Implement single-transaction DuckDB updates
- [ ] Target: 30x speedup for incremental updates

## Phase 4: Fallback Chain
- [ ] Design Micro-CST trait for lightweight parsing
- [ ] Implement Micro-CST for problematic languages (Solidity)
- [ ] Create text outline fallback
- [ ] Build fallback orchestration (tree-sitter → micro → text)
- [ ] Ensure output shape consistency across tiers

## Phase 5: Outline-First Strategy
- [ ] Default to outline-only extraction
- [ ] Define "hot file" heuristics (recent/large/complex)
- [ ] Implement shallow CST extraction (depth ≤ 3)
- [ ] Add node count caps per file
- [ ] Measure token reduction (target: 10-100x)

## Phase 6: DuckDB Schema
- [ ] Design language-agnostic schema
- [ ] Create file tracking table
- [ ] Create symbol table
- [ ] Optional: CST node table
- [ ] Optional: Pattern mining table
- [ ] Build migration from current schema

## Phase 7: Dagger/Docker Pipeline
- [ ] Create Dockerfile with vendored grammars
- [ ] Build deterministic pipeline
- [ ] Add LANGUAGE_PACKS_SHA environment
- [ ] Tag artifacts with repo@commit+packs-hash
- [ ] Create reproducible build tests

## Phase 8: Testing Infrastructure
- [ ] Golden corpus collection (real files per language)
- [ ] Symbol snapshot tests
- [ ] Query contract validation
- [ ] Grammar upgrade workflow
- [ ] Performance benchmarks vs current system

## Phase 9: Migration & Integration
- [ ] Create migration path from current patina-metal
- [ ] Build compatibility layer for existing code
- [ ] Update CLI commands
- [ ] Document breaking changes
- [ ] Create upgrade guide

## Phase 10: Production Hardening
- [ ] Handle large repositories efficiently
- [ ] Add progress reporting
- [ ] Implement error recovery
- [ ] Add telemetry/metrics
- [ ] Create debugging tools

## Research Questions
- [ ] Optimal blob SHA strategy (git vs custom)
- [ ] Query schema versioning approach
- [ ] Most valuable CST patterns for LLMs
- [ ] Migration strategy (parallel vs in-place)

## Success Metrics
- [ ] 10x faster incremental indexing
- [ ] Zero grammar-related runtime failures
- [ ] 90% reduction in token usage for outlines
- [ ] Reproducible builds across time
- [ ] Support for "dead" languages via fallback