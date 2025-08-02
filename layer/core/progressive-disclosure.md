---
id: progressive-disclosure
status: verified
verification_date: 2025-08-02
oxidizer: nicabar
references: [core/principles.md]
tags: [design-principle, user-experience, complexity]
---

# Progressive Disclosure

Patina makes simple things simple while keeping complex things possible.

## Verification

```bash
#!/bin/bash
# Verify progressive disclosure:

echo "Checking progressive disclosure..."

# Basic commands are simple
cargo run -- init test-project 2>&1 | grep -q "error" && exit 1 || echo "✓ Simple init works"

# Advanced features available via flags
cargo run -- init --help 2>&1 | grep -q "\-\-llm" || exit 1
cargo run -- init --help 2>&1 | grep -q "\-\-dev" || exit 1

# Default behaviors are sensible
grep -q "Claude" src/commands/init.rs || exit 1  # Claude is default LLM
grep -q "detect" src/dev_env.rs || exit 1  # Auto-detects environment

# Complex features tucked away
test -d "src/adapters" || exit 1  # Multiple adapters available
test -f "src/workspace_client.rs" || exit 1  # Advanced features exist

# Help is progressive
cargo run -- --help 2>&1 | wc -l | grep -q "^[0-9]$\|^[0-9][0-9]$" || exit 1  # Short help
cargo run -- init --help 2>&1 | wc -l | grep -q "[0-9][0-9]" || exit 1  # Detailed subcommand help

echo "✓ Progressive disclosure verified"
```

## The Pattern

Complexity revealed only when needed:

1. **Defaults that work** - `patina init myproject` just works
2. **Options when needed** - `--llm gemini` for different LLM
3. **Advanced tucked away** - Workspace service for power users
4. **Help scales with need** - Brief overview → detailed options

## Implementation

```rust
// Simple default case
patina init myproject  

// Progressive complexity
patina init myproject --llm gemini
patina init myproject --llm gemini --dev dagger
patina agent start --mode explore
```

## Consequences

- New users succeed immediately
- Power users find what they need
- Complexity doesn't overwhelm
- Interface remains clean