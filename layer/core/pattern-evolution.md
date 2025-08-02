---
id: pattern-evolution
status: verified
verification_date: 2025-08-02
oxidizer: nicabar
references: [core/patterns.md, topics/architecture/knowledge-oxidation.md]
tags: [patterns, knowledge-management, evolution]
---

# Pattern Evolution

Patterns in Patina evolve from project-specific discoveries to universal core principles.

## Verification

```bash
#!/bin/bash
# Verify pattern evolution structure:

echo "Checking pattern evolution..."

# Pattern types support hierarchy
grep -q "pub enum PatternType" src/layer/mod.rs || exit 1
grep -q "Core" src/layer/mod.rs || exit 1
grep -q "Topic" src/layer/mod.rs || exit 1
grep -q "Project" src/layer/mod.rs || exit 1

# Layer structure reflects evolution path
test -d "layer/core" || exit 1
test -d "layer/topics" || exit 1
test -d "layer/surface" || exit 1

# Patterns have metadata for tracking
grep -q "promoted_from:" layer/topics/*/patterns.md 2>/dev/null || echo "⚠ No promotion tracking yet"

# Session patterns can be added
grep -q "pub struct SessionPattern" src/session.rs || exit 1

echo "✓ Pattern evolution verified"
```

## The Pattern

Knowledge flows through layers based on proven value:

```
Project Discovery → Session Pattern → Topic Pattern → Core Pattern
     (surface)         (tracked)        (shared)       (universal)
```

## Implementation

1. **Discovery** - New patterns emerge in projects
2. **Validation** - Patterns prove useful across sessions
3. **Promotion** - Successful patterns move up layers
4. **Universality** - Core patterns apply everywhere

```rust
pub enum PatternType {
    Core,               // Universal truths
    Topic(String),      // Domain patterns
    Project(String),    // Local patterns
}
```

## Consequences

- Knowledge accumulates naturally
- Proven patterns rise to top
- Project-specific stays local
- Wisdom transfers between projects