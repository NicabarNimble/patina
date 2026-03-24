---
type: belief
id: typed-boundary-minimalism
persona: architect
facets: [architecture, boundaries, rust, methodology]
entrenchment: medium
status: active
endorsed: true
extracted: 2026-03-24
revised: 2026-03-24
---

# typed-boundary-minimalism

Encode boundary contracts as closed types, pass only required data, isolate side effects behind explicit ports, and migrate in small parity-verified slices without expanding public surface area.

## Statement

Encode boundary contracts as closed types, pass only required data, isolate side effects behind explicit ports, and migrate in small parity-verified slices without expanding public surface area.

## Evidence

- [[session-20260324-105924]] Defined the project rule after aligning on M6c boundary-hardening and black-box child principles (weight: 0.97)
- [[commit-3cc7f4c3]] Completed M6b by moving lake domain orchestration behind explicit `LakeRepository`/`Clock` ports in core (weight: 0.95)
- [[commit-2a63014f]] Updated crate policy guardrails to permit architecture-aligned `patina-core` and `patina-protocol` boundaries (weight: 0.84)
- [[commit-984d4735]] Fixed legacy migration behavior by reading the correct boundary table, reinforcing adapter/core separation discipline (weight: 0.82)

## Supports

- [[protocol-boundaries-must-be-typed]]
- [[parse-at-boundary-type-the-interior]]
- [[children-have-agency-toys-are-capabilities]]
- [[core-verbs-standalone-mother-additive]]

## Attacks

- [[adding-type-is-not-migrating-model]]
- [[question-mark-on-option-is-silent-swallower]]

## Attacked-By

- [[build-correct-not-temporary]] (status: active, note: transitional compatibility adapters must be explicitly deleted in later slices)

## Applied-In

- Typed builtin control-plane contract model in `crates/patina-protocol/src/lib.rs`
- Typed Mother dispatch boundary parsing/routing in `mother/src/builtin_children.rs`
- Adapter client typed bridge in `src/mother/internal.rs`
- Typed request/response callsite migration in `src/commands/spec/mod.rs`, `src/commands/lake.rs`, `src/commands/doctor.rs`, `src/commands/secrets.rs`, `src/mother/mod.rs`, `src/connect/internal/store.rs`
- Lake domain port extraction in `crates/patina-core/src/lake.rs` with adapter effects in `src/mother/lake_runtime.rs`

## Revision Log

- 2026-03-24: Created — metrics computed by `patina scrape`
- 2026-03-24: Enriched with supports/attacks/applied-in links and concrete M6 evidence
