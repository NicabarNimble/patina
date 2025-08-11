---
id: dependable-rust-implementation
status: active
created: 2025-08-10
references: [core/black-box-boundaries.md, core/modularity-through-interfaces.md]
tags: [architecture, refactor, dependable-rust, implementation-plan]
---

# Dependable Rust: Completing the Black-Box Architecture

## Our Vision

Patina embraces Dependable Rust principles while maintaining pragmatism. We want modules that are:
- **Independently owned** - One person can understand and maintain each module
- **Implementation-flexible** - Internals can evolve without breaking contracts
- **Progressively enhanced** - Start simple, improve over time
- **Domain-expressive** - Public APIs speak the language of the problem

## Current State Analysis

After deep analysis of Eskil's philosophy, we've identified our fundamental mistake: we created **delegating wrappers** instead of **self-contained black boxes**.

### What We Have
- `workspace_client_refactored` ✅ - Actually self-contained! Has its own implementation
- `claude_refactored` ❌ - Just delegates to original claude.rs
- `indexer_refactored` ❌ - Just delegates to original indexer
- `init_refactored` ❌ - Just delegates to original init command
- `navigate_refactored` ❌ - Just delegates to original navigate

### The Mistake We Made
We misunderstood "gradual migration". We thought it meant:
```
refactored_module → delegates to → original_module
```

What Eskil actually means (from his healthcare example):
```
NEW SYSTEM (complete, self-contained)
     ↕️ (glue code if needed)
OLD SYSTEM (complete, self-contained)
```

### The Core Philosophy We Violated
From Eskil: "It's faster to write five lines of code today than to write one line today and then have to edit it in the future."

By creating wrappers, we created code we KNOW we'll have to edit later. That's exactly what he warns against.

## The Dependable Rust Pattern

### Core Structure
```rust
// Public API (mod.rs) - Express the domain
pub struct ServiceName {
    inner: Box<implementation::Core>,
}

impl ServiceName {
    // Minimal public methods that make sense for the domain
    pub fn new() -> Result<Self> { }
    pub fn process(&mut self, input: DomainType) -> Result<Output> { }
}

// Private implementation (implementation.rs) - Hide the complexity
mod implementation {
    pub(super) struct Core {
        // Implementation can be any size, any complexity
        // Can change completely without affecting public API
    }
}
```

### Key Principles

1. **Domain Primitives Stay Public** - Types like `Pattern`, `Location`, `Confidence` are the shared language
2. **Implementation Mechanisms Stay Private** - Databases, caches, state machines are hidden
3. **Progressive Implementation** - Can start with simple implementation, evolve later
4. **Clear Ownership Boundaries** - Each module has one responsible maintainer

## Implementation Roadmap

### Phase 1: Complete the Black Boxes

For each refactored module, we need to internalize the implementation:

#### Indexer Module
- **Current**: Wrapper delegating to original indexer
- **Target**: Self-contained with internal implementation
- **Approach**: 
  - Move indexer logic into `indexer_refactored/implementation.rs`
  - Keep domain types (Pattern, Location, etc.) as public exports
  - Hide all database and caching logic

#### Claude Adapter
- **Current**: Wrapper around original claude.rs
- **Target**: Self-contained adapter implementation
- **Approach**:
  - Internalize template generation and context building
  - Keep simple `create() -> Box<dyn LLMAdapter>` public API
  - Hide versioning and file management

#### Workspace Client
- **Current**: Some deprecated exports, wrapper pattern
- **Target**: Clean client with no deprecated APIs
- **Approach**:
  - Internalize HTTP client logic
  - Remove CreateWorkspaceRequest and ExecRequest from public API
  - Keep only WorkspaceClient methods public

#### Commands (init, navigate, agent)
- **Current**: Wrappers around original commands
- **Target**: Self-contained command implementations
- **Approach**:
  - Move command logic into implementation modules
  - Keep only `execute()` functions public
  - Hide all helper functions and display logic

### Phase 2: Unify the Modules

Once implementations are internalized:

1. **Remove original modules** - They're no longer needed
2. **Drop `_refactored` suffix** - These become the primary modules
3. **Update imports** - Point to the new unified modules
4. **Remove switching logic** - No more environment variables needed

### Phase 3: Polish and Optimize

With unified modules in place:

1. **Optimize implementations** - Now that they're hidden, we can improve freely
2. **Add better error handling** - Internal errors can be rich, public errors simple
3. **Improve performance** - Cache, parallelize, optimize without API changes
4. **Document public APIs** - Focus documentation on what users need

## Pragmatic Considerations

### What We Keep
- **Environment variables during migration** - Useful for testing
- **Domain types as public** - They're our shared vocabulary
- **Trait-based adapters** - Flexibility for different LLMs/environments
- **Progressive enhancement** - Start simple, improve over time

### What We Avoid
- **Premature optimization** - Simple implementations are fine initially
- **Over-abstraction** - Not everything needs to be generic
- **Breaking changes** - Public APIs should be stable
- **Perfect is the enemy of good** - Working code beats perfect architecture

## Success Metrics

A module is "done" when:
- [ ] Public API is under 150 lines
- [ ] Implementation is completely hidden
- [ ] One person can maintain it
- [ ] Tests pass without knowing internals
- [ ] API expresses domain concepts clearly

## Next Steps

1. **Pick one module to start** - Recommend `workspace_client` as it's simplest
2. **Internalize its implementation** - Move code into private implementation.rs
3. **Test thoroughly** - Ensure behavior hasn't changed
4. **Apply pattern to other modules** - Use lessons learned
5. **Clean up when all complete** - Remove switching and duplicate code

## Philosophy Notes

This isn't about rigid rules or extremism. It's about creating maintainable, understandable code where:
- Modules have clear boundaries
- Implementation can evolve
- APIs are stable
- Ownership is clear

We're building tools that will accumulate wisdom over time. The architecture should support that accumulation without becoming brittle.

## Technical Debt Acknowledgment

The current dual-module state is technical debt, but it's *intentional* technical debt that:
- Allows safe migration
- Enables A/B testing
- Provides rollback capability
- Will be cleaned up systematically

This is pragmatic engineering, not perfection-seeking.

## The Path Forward

Each module refactor is an opportunity to:
1. Understand what the module really does
2. Identify its true public interface
3. Hide unnecessary complexity
4. Improve implementation if needed
5. Document the domain concepts

This is how we build Dependable Rust - incrementally, pragmatically, but with clear architectural vision.