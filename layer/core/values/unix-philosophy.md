---
type: value
id: unix-philosophy
status: active
entrenchment: very-high
facets: [architecture, decomposition, philosophy]
references: [dependable-rust]
created: 2026-02-27
distilled_from: layer/core/unix-philosophy.md
---
# Unix Philosophy

One tool, one job, done well. Complex functionality emerges from composition of simple, single-purpose components — not from monolithic systems.

## Test

Is this component a tool (single operation, transforms input to output) or a system (coordinates multiple operations, maintains complex state)? If it's a system, decompose it into tools.

## Consequence

Focused tools are easy to test, replace, and compose. Monolithic systems resist change — modifying one responsibility risks breaking others. New functionality means new commands, not new flags.
