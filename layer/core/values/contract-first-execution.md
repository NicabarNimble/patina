---
type: value
id: contract-first-execution
status: active
entrenchment: very-high
facets: [architecture, control-plane, contracts, wasi, component-model, sdk]
references: [safety-boundaries, dependable-rust, unix-philosophy, spec-driven-design, sdk-vision-lock]
created: 2026-04-21
---
# Contract-First Execution

Separate authority from execution. Mother decides policy and routing, children execute typed WIT operations, and toys are explicit capabilities. Keep boundaries in the WASI component model: declared WIT worlds, declared imports, and `wasm32-wasip2` components.

## Test

Can you point to one Mother authority decision, one typed WIT operation boundary, and one explicit toy scope for this flow? If any is missing, the design is leaking authority or capability.

## Consequence

Control stays centralized while execution stays modular. New callers can be added without changing child behavior, and new children can be authored through the SDK without reopening architecture each session.