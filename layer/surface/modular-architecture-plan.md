# Modular Architecture Plan

## Overview
Refactoring the monolithic workspace service into focused Dagger modules, aligned with the pattern-selection-framework principles.

## Context
- **Current State**: Single workspace service handling multiple responsibilities
- **Target State**: Decomposed modules with single responsibilities
- **Guiding Principle**: Tools not systems (see: [pattern-selection-framework.md](./pattern-selection-framework.md))

## Module Decomposition

A module is a patch of patina—a coherent spot of oxidized knowledge that shows a thin, durable face to the world while hiding a more complex crystal beneath.

### Planned Modules

```
modules/
├── environment-provider/  # Creates containers (was: CreateWorkspace)
├── environment-registry/  # Tracks active environments (was: Get/List)
├── code-executor/        # Runs commands (was: Execute)
├── git-manager/          # Git operations (was: git methods)
└── api-gateway/          # HTTP coordination (was: handlers)
```

### Module Responsibilities

#### environment-provider
- **Purpose**: Create isolated development/build environments
- **Input**: Configuration (name, base image, mounts)
- **Output**: Running container
- **Pattern Type**: Tool (stateless, clear I/O)

#### environment-registry
- **Purpose**: Track and query active environments
- **Input**: Environment ID or query parameters
- **Output**: Environment details or list
- **Pattern Type**: Tool (stateless registry)

#### code-executor
- **Purpose**: Execute commands in containers
- **Input**: Container + command
- **Output**: Execution results (stdout, stderr, exit code)
- **Pattern Type**: Tool (pure execution)

#### git-manager
- **Purpose**: Handle git operations
- **Input**: Repository + git operation
- **Output**: Updated repository state
- **Pattern Type**: Tool (version control operations)

#### api-gateway
- **Purpose**: HTTP API coordination layer
- **Input**: HTTP requests
- **Output**: HTTP responses
- **Pattern Type**: System (coordinates tools)

## Implementation Plan

### Phase 1: Setup (Current)
- [x] Create modular-architecture branch
- [x] Document plan with pattern-selection-framework reference
- [ ] Create modules/ directory structure

### Phase 2: Extract Read-Only Modules
- [ ] Extract environment-registry (queries only, safe)
- [ ] Create tests for registry module
- [ ] Verify parallel operation with existing workspace

### Phase 3: Extract Git Manager
- [ ] Extract git-manager module
- [ ] Test git operations in isolation
- [ ] Update workspace to optionally use module

### Phase 4: Extract Core Functionality
- [ ] Extract environment-provider
- [ ] Extract code-executor
- [ ] Create api-gateway to coordinate

### Phase 5: Migration
- [ ] Run both systems in parallel
- [ ] Gradually migrate workspace calls to modules
- [ ] Remove old workspace service

### Phase 6: Enhancement
- [ ] Add pattern-selector module
- [ ] Add pattern-validator module
- [ ] Add docling module for PDF processing
- [ ] Implement vector storage (SQLite → Qdrant)

## Success Criteria
1. Each module has single responsibility
2. Modules can be tested independently
3. Clear input → output for each module
4. No circular dependencies
5. LLMs can easily understand module purposes

## Testing Strategy
- Unit tests per module
- Integration tests for module composition
- Parallel testing with existing workspace
- Performance benchmarks before/after

## Rollback Plan
- Keep workspace/ intact during migration
- Feature flag for module usage
- Git branch protection
- All changes reversible

## References
- [Pattern Selection Framework](./pattern-selection-framework.md)
- [Docling Pattern Vector Storage](./docling-pattern-vector-storage.md)
- [Eskil Steenberg Rust Patterns](./eskil-steenberg-rust.md)

## Notes
- Start with simplest extractions first (read-only operations)
- Maintain backward compatibility during migration
- Document each module with "patch of patina" metaphor
- Keep names boring and clear for LLM comprehension

---
*Last Updated: 2025-01-13*
*Status: Planning Phase*