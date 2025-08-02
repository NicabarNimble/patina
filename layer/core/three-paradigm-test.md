---
id: three-paradigm-test
status: verified
verification_date: 2025-08-02
oxidizer: nicabar
references: [core/patterns.md, topics/architecture/decisions.md]
tags: [architecture, design-pattern, abstraction]
---

# Three Paradigm Test

Patina tests abstractions against three different approaches to ensure flexibility.

## Verification

```bash
#!/bin/bash
# Verify three-paradigm thinking in codebase:

echo "Checking three-paradigm test implementation..."

# Development environment abstraction handles multiple paradigms
grep -q "pub enum DevEnvironment" src/dev_env.rs || exit 1
grep -q "Dagger" src/dev_env.rs || exit 1
grep -q "Docker" src/dev_env.rs || exit 1
grep -q "Native" src/dev_env.rs || exit 1

# Build command works with different environments
test -f src/commands/build.rs || exit 1
grep -q "DevEnvironment::" src/commands/build.rs || exit 1

# Adapter pattern supports multiple LLMs
test -d src/adapters/claude.rs || exit 1
test -d src/adapters/gemini.rs || exit 1

echo "✓ Three paradigm test verified"
```

## The Pattern

When designing abstractions:
1. **Traditional approach** (e.g., Docker - established, universal)
2. **Modern approach** (e.g., Dagger - cutting-edge, powerful)
3. **Alternative approach** (e.g., Native - different paradigm)

If the abstraction handles all three, it's properly designed.

## Implementation

```rust
// DevEnvironment abstraction passes the test:
pub enum DevEnvironment {
    Docker,    // Traditional container approach
    Dagger,    // Modern pipeline approach  
    Native,    // Alternative local approach
}
```

## Consequences

- Abstractions remain flexible
- Not locked into single paradigm
- Future approaches fit naturally
- Users choose their preferred tool