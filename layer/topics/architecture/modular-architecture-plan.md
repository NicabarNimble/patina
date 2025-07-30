---
id: modular-architecture-plan
version: 1
created_date: 2025-07-27
confidence: medium
oxidizer: nicabar
tags: []
promoted_from: projects/patina
---

# Modular Architecture Plan for Patina

## Overview
Refactor Patina into clear module boundaries to enable independent versioning and updates without rebuilding core. This follows the Unix philosophy of "do one thing well" while maintaining a monorepo structure.

## Current State
- Single binary with mixed concerns
- Commands contain adapter-specific logic  
- No independent versioning of components
- Difficult to update adapters without rebuilding everything

## Target Architecture

### Module Structure
```
patina/
├── src/
│   ├── core/           # Core orchestration (patina-core)
│   ├── adapters/
│   │   ├── claude/     # Claude adapter (patina-claude)
│   │   ├── gemini/     # Future: Gemini adapter
│   │   └── traits.rs   # Adapter contracts
│   ├── integrations/
│   │   └── dagger/     # Dagger integration (patina-dagger)
│   └── main.rs         # Thin binary entry point
```

### Module Responsibilities

#### patina-core (v0.1.0)
- Brain management (read/write patterns)
- Project structure (.patina directory)
- Command orchestration (init, add, commit, push)
- LLMAdapter trait definition
- Core business logic

#### patina-claude (v0.2.1)
- Claude-specific adapter implementation
- Session commands (start, update, end)
- CLAUDE.md generation
- Claude-specific resources
- Adapter manifest and versioning

#### patina-dagger (v0.1.0)
- Dagger pipeline templates
- Container orchestration
- Build/test container logic
- Pipeline generation

## Implementation Plan

### Phase 1: Clean Interfaces (Current Focus)
1. **Push adapter logic down**
   - Remove Claude-specific code from commands
   - Add `post_init()`, `post_build()` methods to LLMAdapter trait
   - Let adapters manage their own file creation

2. **Separate version tracking**
   ```rust
   // core/version.rs
   pub const CORE_VERSION: &str = "0.1.0";
   
   // adapters/claude/version.rs
   pub const CLAUDE_VERSION: &str = "0.2.1";
   ```

3. **Resource organization**
   - Move Claude scripts to src/adapters/claude/resources/
   - Move Dagger templates to src/integrations/dagger/templates/

### Phase 2: Module Boundaries
1. **Command refactoring**
   - Core commands only orchestrate
   - Adapters handle their specific logic
   - Integration points through traits

2. **Update mechanism**
   - Each module checks its own version
   - Independent update paths
   - Preserve user modifications

### Phase 3: Future Workspace (Optional)
- Split into cargo workspace
- Separate Cargo.toml per module
- Independent compilation
- Shared trait crate

## Key Principles

1. **Backwards Compatible**: Changes don't break existing installations
2. **Gradual Migration**: Can be done incrementally
3. **Clear Ownership**: Each module owns its resources and logic
4. **Independent Versions**: Update adapters without touching core

## Success Criteria

- [ ] Can update Claude session commands without rebuilding patina binary
- [ ] Can swap claude → gemini adapter with single config change
- [ ] Clear module boundaries with no cross-dependencies
- [ ] Each module has independent version tracking

## Example: Updating Session Commands

Before:
1. Edit session scripts
2. Rebuild entire patina
3. User runs `patina update` (rebuilds everything)

After:
1. Edit session scripts in patina-claude
2. Bump CLAUDE_VERSION
3. User runs `patina update` (only updates Claude files)

## Version Management Concept

### Core Principle
Patina needs a way to update components independently without rebuilding the entire binary. This aligns with our modular architecture and Unix philosophy.

### Git-Native Approach
Since Patina projects already use git, leveraging git's built-in versioning capabilities provides:
- No external dependencies
- Decentralized by default
- Enterprise-familiar workflows
- Built-in history and rollback

### Conceptual Flow
1. **Component Versioning**: Each module (core, claude, dagger) tracks its own version
2. **Update Discovery**: System can check for newer versions of installed components
3. **Selective Updates**: Users can update specific components without affecting others
4. **File-Based Updates**: Components can update their files without binary recompilation

### Key Considerations
- **Local-First**: Must work without network access
- **User Control**: Users decide when and what to update
- **Preserve Modifications**: Respect user changes to generated files
- **Clear Communication**: Show what will change before updating

### Implementation Freedom
The exact mechanism (tags, branches, manifests) should be determined during implementation based on:
- Simplicity of use
- Git best practices
- FOSS community expectations
- Enterprise requirements

## Migration Path

1. Start with Phase 1 refactoring
2. Implement version tracking for each component
3. Build update mechanism that supports independent updates
4. Test with real component updates
5. Iterate based on usage patterns
6. Consider workspace when patterns are clear