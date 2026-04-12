# BA Alignment Primitives

> This file locks the primitives Patina aligns to before discussing repos or talks.

## Why this exists

Prevent markdown sprawl and direction drift.

- Primitive truth first
- Evidence second
- Narrative last

If a note cannot be tied to one primitive, it does not belong in BA direction docs.

## Primitive Set (v1)

## P1 — Component Model composition boundaries

**Definition:** Components compose through typed import/export interfaces; compatibility is checked at interface boundaries.

**Patina implication:** Child composition must remain typed and fail-closed on wiring mismatch.

**Acceptance test (plain):**
- Invalid typed wiring fails deterministically (no silent skip).

## P2 — WIT as contract surface

**Definition:** WIT defines the machine-checkable boundary for capabilities and data shapes.

**Patina implication:** New data-plane child contracts are authored as typed WIT interfaces; avoid stringly contracts by default.

**Acceptance test (plain):**
- New data-plane child exports/imports are typed in WIT worlds, not only `handle(action,payload)`.

## P3 — WASI-first capability baseline

**Definition:** Use standardized WASI capability interfaces where available; introduce Patina-specific interfaces only for missing delta.

**Patina implication:** Prefer `wasi:*` imports (logging, keyvalue, filesystem, etc.), and document every `patina:*` addition as intentional delta.

**Acceptance test (plain):**
- For each child capability, map to `wasi:*` or explicit `patina:*` delta with rationale.

## P4 — Explicit authority and least privilege

**Definition:** Capability grants are explicit and enforced at boundaries.

**Patina implication:** Mother validates manifest grants and call-time access; unauthorized use fails closed.

**Acceptance test (plain):**
- A child without grant cannot call a gated toy (verified by tests/runtime behavior).

## P5 — Evolution by evidence, not hype

**Definition:** Direction decisions are based on confirmed standards + conference signals with confidence labels.

**Patina implication:** BA direction snapshots require source confidence/status tags; unverified claims cannot drive roadmap.

**Acceptance test (plain):**
- Roadmap-affecting statements in `DIRECTION.md` cite confirmed evidence.

## Patina Deltas (allowed)

Patina may extend beyond current standards when needed, but each delta must include:

1. Why WASI/standard surface is insufficient today
2. Safety model and authority boundary
3. Upstreamability stance (`candidate`, `unlikely`, `unknown`)

Examples currently visible in codebase:
- `patina:records` domain interfaces
- `patina:measure`

## Grounding protocol (context / scry / assay)

Before updating BA direction artifacts:

```bash
patina context --topic "ba alignment"
patina scry "component model wit wasi" --all-repos
patina assay search "component model" --all-repos
```

Before writing/updating a BA belief:

```bash
patina assay belief <belief-id>
patina scry --belief <belief-id> --impact
```

## Anti-sprawl rule

- Raw intake lives in structured files (`conferences/catalog.jsonl`)
- Direction markdown is summary + decisions only
- Beliefs hold normative claims and must be grounded
