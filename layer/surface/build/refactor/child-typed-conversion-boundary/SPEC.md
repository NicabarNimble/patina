---
type: refactor
id: child-typed-conversion-boundary
status: draft
created: 2026-04-14
blocked_by:
  - durable-rust-unix-realignment-program
beliefs:
  - "[[dependable-rust]]"
  - "[[spec-driven-design]]"
  - "[[unix-philosophy]]"
related:
  - src/child/internal/child.rs
  - src/child/runtime.rs
  - tests/wasm_integration.rs
  - layer/surface/reports/audit/2026-04-14-durable-rust-unix-realignment-audit.md
exit_criteria:
  - id: ctcb1-conversion-module-extracted
    text: "JSON↔component typed conversion logic is extracted from `child.rs` into dedicated conversion modules with minimal public surface."
    checked: false
  - id: ctcb2-error-contract-locked
    text: "Conversion failures expose stable machine error codes + structured details (text may improve; code contract is stable)."
    checked: false
  - id: ctcb3-no-implicit-coercions
    text: "Transformer is strict/fail-closed: no implicit JSON coercions (e.g., string→int, float→int, missing required fields, unknown variant/enum cases)."
    checked: false
  - id: ctcb4-fail-closed-coverage
    text: "Deterministic tests cover invalid args/type-shape failures and confirm explicit typed error codes."
    checked: false
  - id: ctcb5-conformance-lock
    text: "Table-driven conformance tests lock behavior for representative WIT type families (scalar, record, tuple, list, option, variant, enum, result, flags)."
    checked: false
  - id: ctcb6-roundtrip-policy-locked
    text: "Round-trip policy is explicit: strict where bijective; otherwise normalized equivalence or explicit fail-closed behavior, with tests per policy."
    checked: false
  - id: ctcb7-explicit-results-envelope
    text: "Typed component result lifting uses explicit JSON envelope `{\"results\":[...]}` (including zero/one/many results) instead of shape-overloaded null/value/array output."
    checked: false
  - id: ctcb8-no-runtime-artifact-dependence
    text: "Core conversion tests do not require local `target/` runtime artifacts."
    checked: false
  - id: ctcb9-authority-path-unchanged
    text: "Typed dispatch authority path remains unchanged (`ChildCallRequest` -> registry policy -> child call)."
    checked: false
---

# refactor: child typed conversion boundary

## Problem

Typed conversion logic in `src/child/internal/child.rs` is large and manually branched, increasing drift risk and review complexity as interface shapes evolve.

## Goal

Create a dedicated, test-locked conversion boundary for JSON↔WIT/component values that is easier to reason about and evolve safely.

## HITL decisions (approved)

1. **WIT authority posture:** JSON is transport-only; WIT/component types are contract authority.
2. **Error contract strictness:** stable machine error codes + structured details are the compatibility contract (not exact error text).
3. **Module shape:** conversion extracted into WIT type-family modules (scalar, record, tuple, list, option, variant, enum, result, flags).
4. **Round-trip policy:** strict where WIT↔JSON mapping is bijective; otherwise explicit normalized policy or fail-closed.
5. **Strictness rule:** remove implicit coercions and keep fail-closed behavior deterministic.
6. **Scope posture:** refactor-first with strictness hardening; no contract-layer redesign in this slice.
7. **Result shape decision:** use explicit JSON result envelope `{\"results\":[...]}` for typed component return lifting.

## Conversion invariants (authoritative)

1. **RT-INV-01 (typed identity where bijective)**
   For bijective WIT families, `component_val -> json -> component_val` is identity-preserving.
2. **RT-INV-02 (normalized equivalence where non-bijective)**
   For non-bijective families, round-trip must satisfy documented normalized equivalence (or fail-closed), never silent coercion.
3. **RT-INV-03 (fail-closed parse boundary)**
   Invalid JSON shape/type for the target WIT signature fails with stable machine error code + structured details.
4. **RT-INV-04 (result lifting explicitness)**
   Typed call return lifting always uses `{"results":[...]}` envelope (including empty/one/many), never overloaded null/value/array shape.

## Type-coverage matrix (must be test-locked)

- Scalar (bool/signed/unsigned/float/char/string)
  - Policy: strict parse, no implicit widening/narrowing coercion
  - Round-trip: RT-INV-01 where representable; otherwise RT-INV-03
- Record
  - Policy: required fields required, unknown-field policy explicit (fail-closed unless documented otherwise)
  - Round-trip: RT-INV-01
- Tuple
  - Policy: positional arity strict
  - Round-trip: RT-INV-01
- List
  - Policy: element type strict
  - Round-trip: RT-INV-01
- Option
  - Policy: explicit null/Some mapping documented and stable
  - Round-trip: RT-INV-02 if normalization needed
- Variant
  - Policy: case name + payload shape strict
  - Round-trip: RT-INV-02 or RT-INV-03 per case
- Enum
  - Policy: only declared variants accepted
  - Round-trip: RT-INV-01
- Result
  - Policy: explicit ok/err channel mapping, no implicit success/failure inference
  - Round-trip: RT-INV-02
- Flags
  - Policy: only declared flags accepted
  - Round-trip: RT-INV-01

## Bytecode Alliance alignment target

- Preserve Canonical ABI authority for typed value semantics.
- Keep JSON interpretation minimal and explicit at ingress/egress edge only.
- Prefer generated typed bindings where possible; dynamic JSON transformer remains a strict adapter seam.

## Non-goals

- No child contract redesign.
- No WIT schema changes in this slice.
- No change to typed dispatch authority path.
- No new fallback compatibility mode that weakens strict conversion rules.
- No continuation of shape-overloaded result lifting (`null`/single value/array) on the strict typed path.
