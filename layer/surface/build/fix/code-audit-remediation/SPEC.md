---
type: fix
id: code-audit-remediation
status: ready
created: 2026-03-31
sessions:
  origin: 20260331-224232-852361000
related:
- layer/surface/build/fix/car-safety-a1-a6/SPEC.md
- layer/surface/build/fix/car-architecture-a7-a25/SPEC.md
- layer/surface/build/fix/car-deadcode-a8-a24/SPEC.md
- layer/surface/build/fix/car-dedup-a9-a21/SPEC.md
- layer/surface/build/fix/car-cleanup-non-a/SPEC.md
references:
- layer/core/patina-identity.md
- layer/core/unix-philosophy.md
- layer/core/spec-driven-design.md
- layer/core/dependable-rust.md
- layer/core/adapter-pattern.md
- layer/core/session-capture.md
exit_criteria:
- id: car-program-split
  text: Remediation scope is split into five focused child specs (safety, architecture, dead code, dedup, cleanup) and this umbrella spec no longer authorizes direct code edits.
  checked: true
- id: car-program-ordering
  text: 'Execution order is explicit: safety -> architecture -> dead code -> dedup -> cleanup.'
  checked: true
- id: car-program-traceability
  text: Each child spec carries scope-limited file lists, test proof requirements, and direct references to layer/core values.
  checked: true
---

# fix: Code Audit Remediation (Umbrella)

This is the umbrella coordination spec for remediation from the full audit of `[[session-20260331-224232-852361000]]`.

Per [[spec-driven-design]] and [[unix-philosophy]], this spec is intentionally narrowed to orchestration only. Implementation authorization lives in focused child specs.

## Why Split

The original 30-gate single spec mixed correctness/safety, architecture inversion, dead code deletion, dedup, and cleanup into one contract. That violates "one tool, one job" and increases review/rollback risk.

Split outcome:

- smaller scope per spec,
- tighter test proofs,
- safer commit boundaries,
- better alignment with [[dependable-rust]] and Gjengset-style incremental safety.

## Child Specs

1. `car-safety-a1-a6` — correctness and safety first. Capability check (A3) is security-critical per [[children-have-agency-toys-are-capabilities]].
2. `car-architecture-a7-a25` — A7 only (retrieval must stand alone for Mother-served scry). A25 dropped: spec dispatch inversion is intentional daemon-first architecture.
3. `car-deadcode-a8-a24` — delete dead paths after safety and architecture settle. A22 requires care: toy host functions may be "dead to Rust" but live in the toybox.
4. `car-dedup-a9-a21` — converge duplicated logic without behavior expansion.
5. `car-cleanup-non-a` — deprecated/stale cleanup and docs alignment.

## Execution Rules

- Do not implement code changes against this umbrella spec directly.
- Child specs must be executed in the declared order.
- Each child spec must satisfy its own exit criteria before next child activates.
- Every gate commit should be rollback-safe and narrow (scalpel, not shotgun).

See DESIGN.md for program-level sequencing and governance.
