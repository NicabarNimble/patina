---
id: safety-boundaries
status: verified
verification_date: 2025-08-02
oxidizer: nicabar
references: [core/constraints.md]
tags: [safety, security, boundaries]
---

# Safety Boundaries

Patina respects system boundaries and operates safely within designated areas.

## Verification

```bash
#!/bin/bash
# Verify safety boundaries:

echo "Checking safety boundaries..."

# No unsafe code blocks
if grep -r "unsafe {" src/ 2>/dev/null | grep -v "test" | grep -v "comment"; then
    echo "✗ Unsafe code detected"
    exit 1
fi

# File operations confined to project
grep -q "project_root" src/session.rs || exit 1
grep -q "layer/" src/layer/mod.rs || exit 1

# No hardcoded system paths
if grep -r "/usr/local" src/ 2>/dev/null | grep -v "test"; then
    echo "✗ Hardcoded system paths found"
    exit 1
fi

# User consent required for operations
grep -q "confirm" src/commands/ -r || echo "⚠ No confirmation prompts found"

# .gitignore respects privacy
grep -q ".claude/context/sessions" .gitignore || exit 1
grep -q ".patina/session.json" .gitignore || exit 1

echo "✓ Safety boundaries verified"
```

## The Pattern

Patina operates within clear boundaries:

1. **No unsafe code** - Rust's safety guarantees maintained
2. **Project-scoped files** - Never modify system files
3. **User consent** - Ask before major operations
4. **Privacy respected** - Personal sessions stay local

## Implementation

- All paths relative to project root
- Session data in gitignored directories
- No network calls without consent
- Clear separation of user/shared data

## Consequences

- Users trust Patina's operations
- No accidental system changes
- Clear data ownership
- Safe to use anywhere