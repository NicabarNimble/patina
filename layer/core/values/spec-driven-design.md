---
type: value
id: spec-driven-design
status: active
entrenchment: very-high
facets: [governance, specs, process]
references: [dependable-rust, unix-philosophy]
created: 2026-02-27
distilled_from: layer/core/spec-driven-design.md
---
# Spec-Driven Design

Every non-trivial change is authorized by a spec. Sessions discuss, specs decide, code executes. When the AI encounters an edge case the spec doesn't address, the correct action is to stop and ask — not to make a judgment call.

## Test

Can you trace this code change back to a spec? If not, either the change is trivial (typo, formatting) or it's unauthorized scope creep.

## Consequence

Specs prevent agentic drift. Every decision has provenance: code -> commit -> spec -> session -> beliefs. Without this chain, accumulated knowledge is just text.
