---
id: context-orchestration
status: verified
verification_date: 2025-08-02
oxidizer: nicabar
references: [topics/architecture/digital-garden-architecture.md, core/principles.md]
tags: [architecture, context, core-function]
---

# Context Orchestration

Patina's core function: orchestrate context between users, LLMs, and accumulated wisdom.

## The Pattern

Context flows through Patina:

```
User → Environment Detection → Pattern Selection → LLM Context Generation
         ↓                      ↓                    ↓
    System State           Layer Wisdom         CLAUDE.md/GEMINI.md
```

## Implementation

1. **Environment** captures system state
2. **Layer** provides relevant patterns
3. **Adapter** generates LLM-specific context
4. **Update** keeps context fresh

```rust
// The orchestration in action
let env = Environment::detect()?;
let patterns = layer.get_patterns(PatternType::Core)?;
let context = adapter.generate_context(&patterns, &env)?;
```

## Consequences

- LLMs always have current context
- Patterns accumulate over time
- Each LLM gets optimized format
- Context stays project-specific