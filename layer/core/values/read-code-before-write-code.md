---
type: value
id: read-code-before-write-code
status: active
entrenchment: very-high
facets: [workflow, engineering, evidence, safety]
references: [dependable-rust, contract-first-execution]
created: 2026-05-12
---
# Read Code Before Write Code

Read the relevant implementation and durable project context before changing code. Do not design from memory when the code can answer.

## Test

Before editing, can you point to the files, artifacts, or command surfaces you inspected? If not, stop and read first.

## Consequence

Changes stay aligned with the real system instead of imagined architecture. Surprises become evidence before implementation, not bugs after implementation.
