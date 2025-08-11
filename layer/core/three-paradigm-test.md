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

## The Pattern

When designing abstractions:
1. **Traditional approach** (e.g., Docker - established, universal)
2. **Modern approach** (e.g., Dagger - cutting-edge, powerful)
3. **Alternative approach** (e.g., Native - different paradigm)

If the abstraction handles all three, it's properly designed.

## Consequences

- Abstractions remain flexible
- Not locked into single paradigm
- Future approaches fit naturally
- Users choose their preferred tool