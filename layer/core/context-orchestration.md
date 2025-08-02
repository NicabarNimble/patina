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

## Verification

```bash
#!/bin/bash
# Verify context orchestration flow:

echo "Checking context orchestration..."

# Environment detection feeds context
grep -q "pub struct Environment" src/environment.rs || exit 1
grep -q "pub fn detect()" src/environment.rs || exit 1

# Layer provides patterns for context
grep -q "pub fn get_patterns" src/layer/mod.rs || exit 1
grep -q "PatternType" src/layer/mod.rs || exit 1

# Adapters consume environment + patterns
grep -q "generate_context.*patterns.*Environment" src/adapters/mod.rs || exit 1

# Context files are generated
test -f CLAUDE.md || echo "⚠ CLAUDE.md not found (run patina init first)"

# Update command refreshes context
cargo run -- update --help 2>/dev/null | grep -q "context" || exit 1

echo "✓ Context orchestration verified"
```

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