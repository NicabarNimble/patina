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