---
type: belief
id: no-untyped-blobs-at-trust-boundaries
persona: architect
facets: [security, architecture, pipe]
entrenchment: medium
status: active
endorsed: true
extracted: 2026-03-10
revised: 2026-03-10
---

# no-untyped-blobs-at-trust-boundaries

Patina can be concrete before it is generic, but it cannot be vague at trust boundaries. Capability grants use typed structs, not Option<Value>. When a second real case exists, add a second typed field — duplication is acceptable until abstraction emerges.

## Statement

Patina can be concrete before it is generic, but it cannot be vague at trust boundaries. Capability grants use typed structs, not Option<Value>. When a second real case exists, add a second typed field — duplication is acceptable until abstraction emerges.

## Evidence

- [[session-20260310-142000]]: Corrected Option<Value> on InitializeParams to typed DuckLakeGrant after agent review identified it as vague at a security boundary (weight: 0.9)

## Supports

- [[initialize-is-capability-grant]] — if init is a security boundary, the grant must be typed
- [[connector-toy-is-indivisible-authority]] — the connector grant has distinct security roles that deserve type-level expression
- [[safety-boundaries]] — project-scoped, user consent, clear data ownership

## Attacks

- "Generic capability framework" — a universal toy system with untyped payloads would violate this belief. Type the real thing, not the abstraction.

## Attacked-By

- "Velocity" — typing each grant slows down adding new child types. Accepted tradeoff: security boundaries are worth the cost of a new struct.

## Applied-In

- [[ducklake]] DESIGN.md: `DuckLakeGrant`, `ConnectorToy`, `StorageToy` as typed fields on `InitializeParams`
- [[ducklake]] DESIGN.md: child fails closed with `PipeError::Fatal` if grant is missing

## Revision Log

- 2026-03-10: Created — corrected from Option<Value> approach after agent review
