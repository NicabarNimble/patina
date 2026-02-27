---
type: value
id: patina-identity
status: active
entrenchment: very-high
facets: [identity, architecture, protocol]
references: [dependable-rust, unix-philosophy, spec-driven-design]
created: 2026-02-27
distilled_from: layer/core/patina-identity.md
---
# Patina Identity

Patina is a knowledge protocol for AI-assisted development defined by five verbs: capture, index, search, believe, evolve. The binary is the pipeline, the layer is the product, the protocol is the contract.

## Test

Before adding a module: is it a protocol operation, protocol tooling, or protocol infrastructure? If none of the above, it's a plugin — don't add it to the binary.

## Consequence

The protocol core stays small and hardens as tooling extracts to plugins. Adding features that don't serve the five verbs dilutes the core.
