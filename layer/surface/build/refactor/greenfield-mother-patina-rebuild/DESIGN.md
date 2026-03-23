# Design: Greenfield Mother + Patina Rebuild

## Why This Design

The active refactor proves we can migrate safely, but migration-safe shape is not always
the same as ideal shape. This design captures the ideal shape explicitly so future work has
a clear north star rather than inheriting accidental boundaries.

## Build Target

Define the architecture we would implement if starting from empty source tree today:

- Patina core as protocol product,
- Mother as standalone runtime daemon,
- children as opt-in extensions,
- toys as least-privilege host capabilities,
- interface runtimes as external guests.

## Design Principles

1. Beliefs are the product; infrastructure serves belief loops.
2. Mother runtime ownership is singular and explicit.
3. Core verbs are local-first and standalone-capable by policy.
4. Child seams are contracts, not convenience wrappers.
5. Session artifacts are runtime-owned outputs, not ad-hoc CLI state.
6. Verification evidence is required before ownership moves.

## Work Plan

### Slice 0: Current-state evidence pass (required)

- Build a path-anchored ownership snapshot before adding any new target claims.
- Capture evidence for child boundary, toy grant model, Mother runtime ownership, and remaining CLI seams.
- Record contradictions explicitly; do not hide them behind "future cleanup" language.

Required anchors:

- `src/child/mod.rs:1`
- `src/child/toy_host/mod.rs:1`
- `mother/src/state.rs:70`
- `mother/src/toys.rs:13`
- `src/commands/mother/daemon.rs:670`
- `src/commands/spec/mod.rs:374`

### Slice A: Architecture map (GF1)

- Define ownership matrix for `core`, `mother`, `children`, `sdk`, `wit`.
- Mark each boundary as `permanent contract` or `migration scaffold`.
- Add explicit "current owner -> target owner" for each row.

### Slice B: Runtime policy matrix (GF2, GF5)

- Build command behavior matrix for Mother on/off.
- Document failure behavior and error message contracts.
- Lock guest-runtime rules (Claude/OpenCode/Gemini).
- Keep control-plane verbs (`spec`, `lake`, `doctor`) under explicit `mother-required` policy.
- Keep core knowledge verbs under explicit `standalone-core` policy.

### Slice C: Data and lifecycle model (GF3, GF4)

- Define canonical data stores and ownership (`events`, projections, sessions).
- Define child lifecycle: install, load, health, invoke, revoke.
- Define toy grants/scopes and enforcement points.
- Ensure schema language matches active child manifest contract (`[needs].toys`, `[needs.scopes]`).

### Slice D: Migration map and risk model (GF6, GF7)

- Translate greenfield target into bounded migration slices.
- Add parity gates, rollback protocol, and blast-radius notes.
- Add one migration ledger row per ownership move before promoting to active.

## Migration Ledger Template (authoritative)

| Slice | Current owner | Target owner | Parity gates | Rollback trigger | Rollback action | Blast radius |
| --- | --- | --- | --- | --- | --- | --- |
| Example: Mother daemon command seam | `src/commands/mother/daemon.rs` | `mother/src/*` | `cargo check -q`; targeted mother on/off probes; `cargo test -q` | Health/status/dispatch mismatch or command regression | Revert seam commit and restore prior routing path | `patina mother`, child dispatch surfaces |

Populate with real rows during execution; do not leave example row as final content.

## Direct Documentation Targets

- `layer/surface/build/refactor/greenfield-mother-patina-rebuild/SPEC.md`
- `layer/surface/build/refactor/greenfield-mother-patina-rebuild/DESIGN.md`
- Optional follow-on references in architecture docs once reviewed.

## Verification Plan

1. Baseline proof pass: `cargo check -q`, `cargo test -q`.
2. Boundary proof pass: every ownership claim references `path:line` evidence.
3. Runtime policy proof pass: Mother on/off matrix rows have reproducible command probes.
4. Migration proof pass: every ledger slice has parity and rollback checks.
5. Review pass: check consistency with locked beliefs and AGENTS runtime policy.

## Risks and Controls

- Risk: greenfield design drifts into fantasy architecture with no migration path.
  - Control: every target decision needs a migration slice and parity gate.
- Risk: accidental reopening of settled beliefs.
  - Control: conflicts require explicit contradiction evidence and rationale.
- Risk: overfitting to one interface runtime.
  - Control: keep runtime-guest contract provider-neutral by default.

## Open Questions

- Should Mother query orchestration remain a permanent adapter seam to core retrieval,
  or move into a shared domain crate long-term?
- Should child manifests adopt stricter version pinning in project manifests by default?
- What minimum observability surface is required before enforcing child hard-fail policies?

## Build Readiness

- [ ] GF1-GF7 have concrete sections and evidence links.
- [ ] Slice 0 evidence anchors are captured and contradictions are explicit.
- [ ] Runtime policy matrix is command-level, not principle-only.
- [ ] Migration ledger has at least one real row per ownership-moving lane.
- [ ] Migration map has executable parity gates.
- [ ] No contradictions with active refactor truth map remain unresolved.
