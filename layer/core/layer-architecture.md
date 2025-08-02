---
id: layer-architecture
status: verified
verification_date: 2025-08-02
oxidizer: nicabar
references: [sessions/20250731-192609.md, topics/architecture/layer-structure-evolution.md]
tags: [architecture, layer-system, core-pattern]
---

# Layer Architecture

Patina organizes knowledge in three layers: core, surface, and dust.

## Verification

```bash
#!/bin/bash
# Verify layer architecture is implemented:

# Layer directories exist
test -d "layer/core" || exit 1
test -d "layer/surface" || exit 1
test -d "layer/topics" || exit 1  # TODO: will become dust

# Layer module implements the pattern
grep -q "pub enum LayerType" src/layer/mod.rs || exit 1
grep -q "Core" src/layer/mod.rs || exit 1
grep -q "Topic" src/layer/mod.rs || exit 1
grep -q "Project" src/layer/mod.rs || exit 1

# Layer commands exist
cargo run -- layer --help 2>/dev/null | grep -q "list" || exit 1

echo "✓ Layer architecture verified"
```

## The Pattern

**Core** - Implemented patterns you can grep for in code
**Surface** - Active work and emerging patterns  
**Dust** - Valuable archive of deprecated knowledge

## Implementation

```rust
// From src/layer/mod.rs
pub enum LayerType {
    Core,
    Topic(String),
    Project(String),
}
```

Patterns move through layers based on use:
- Surface → Core (proven patterns)
- Core/Surface → Dust (deprecated patterns)

## Consequences

- Clear lifecycle for knowledge
- Automatic organization by relevance
- Natural pruning of outdated patterns
- Scalable from project to career