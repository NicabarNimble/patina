---
id: unix-philosophy
status: verified
verification_date: 2025-08-02
oxidizer: nicabar
references: [core/principles.md, sessions/20250715-initial-architecture.md]
tags: [architecture, philosophy, core-principle]
---

# Unix Philosophy

Patina follows Unix philosophy: one tool, one job, done well.

## The Pattern

Each Patina component has a single, clear responsibility:
- `layer/` - Manages knowledge storage and retrieval
- `adapters/` - Handles LLM-specific integration
- `commands/` - Implements user-facing CLI actions
- `environment/` - Detects system capabilities

## Implementation

This philosophy manifests in:

1. **Modular architecture** - Each module can be understood in isolation
2. **Composable commands** - Commands can be piped and combined
3. **Text interfaces** - All output is text, parseable by other tools
4. **No feature creep** - New functionality means new commands, not new flags

## Consequences

- Easy to test individual components
- Clear mental model for users
- Natural composition of functionality
- Predictable behavior