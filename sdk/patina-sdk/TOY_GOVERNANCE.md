# Patina SDK Toy Governance (MCT Canon)

> **Purpose:** Make `patina-sdk` the canonical construction guide for MCT child authors:
> build a child with typed contracts first, add toys only for true capability gaps,
> and define the approval path for toys to become trusted system surface.

## 1) Scope and intent

This document is normative for first-party MCT authoring and the baseline for
community toy promotion.

It defines:

- What a toy is and is not.
- Which toy lanes exist (WASI / Patina / Community / Local).
- How a toy moves from private experiment to approved system capability.
- What guarantees approved toys provide.
- What changes are still needed in this repo to fully enforce this model.

This document does **not** replace child business contracts; child contracts are
still authored in WIT worlds/interfaces owned by each child.

---

## 2) Core definitions

## 2.1 Child

A `child` is a WASM component implementing business behavior through typed WIT
interfaces.

## 2.2 Toy

A `toy` is a Mother-mediated capability boundary opening in the sandbox wall.

A toy exists when capability requires host authority or shared control-plane
semantics (e.g., git, config, keyvalue, logging, event streams).

## 2.3 Needs and scopes

Children declare capabilities via:

- `[needs].toys = [...]`
- optional `[needs.scopes]`

No ambient authority is allowed.

## 2.4 Toy tiers

- **WASI-approved toy**: standard `wasi:*` capability/package adopted by Patina.
- **Patina-approved toy**: first-party `patina:*` capability accepted into core registry.
- **Community toy**: externally authored toy package available to users but not yet
  approved as core Patina capability.
- **Local/private toy**: project-local toy with no compatibility promises.

---

## 3) Design principles (non-negotiable)

1. **WASI-first baseline**
   - Use standardized WASI interfaces where they fit.
   - Introduce `patina:*` only for real delta.

2. **Typed boundaries first**
   - Toy contracts are WIT packages/interfaces.
   - Avoid stringly payloads for new capability surfaces.

3. **Least privilege and fail-closed**
   - Missing grant or out-of-scope call must deny deterministically.

4. **Control plane authority**
   - Mother validates grants and wiring; child does not self-escalate authority.

5. **Evidence over hype**
   - Approval and roadmap decisions require concrete usage + tests + observability.

---

## 4) Toy lifecycle and promotion model

## 4.1 States

A toy may be in one of these states:

1. `local`
2. `community-experimental`
3. `candidate`
4. `approved`
5. `deprecated`
6. `retired`

## 4.2 Promotion gates

### local -> community-experimental

Required:

- WIT contract published and versioned.
- Minimal docs and examples.
- Basic host implementation and smoke test.

### community-experimental -> candidate

Required:

- At least two independent child use-cases.
- Threat/safety notes (authority boundary, abuse cases).
- Clear reason why existing WASI + approved toys are insufficient.

### candidate -> approved

Required:

- Naming and version review.
- Deterministic deny/fail-closed tests.
- Conformance tests and compatibility guarantees.
- Observability events for grant/use/deny paths.
- Owner assignment and deprecation policy.

### approved -> deprecated

Required:

- Successor toy or removal rationale.
- Published migration path and timeline.
- Compatibility window announcement.

### deprecated -> retired

Required:

- Migration window elapsed.
- Runtime/SDK gates enforce non-use or explicit legacy lane.

---

## 5) Required artifact set for any candidate/approved toy

1. **Contract**
   - WIT package + interfaces/world integration points.
   - Semver version and package id.

2. **Host implementation**
   - Mother-side implementation with explicit grant checks.

3. **SDK surface**
   - Ergonomic wrapper in `sdk/patina-sdk/src/toys/*` for approved toys.

4. **Registry metadata**
   - Entry in `wit/toys/deps/toys-registry.toml` (or successor registry source).

5. **Verification**
   - Positive path tests.
   - Deny/fail-closed tests.
   - Compatibility tests (where applicable).

6. **Documentation**
   - Purpose and boundary.
   - Scope model.
   - Upstreamability stance (`candidate` / `unlikely` / `unknown`).

---

## 6) Async and Preview3 readiness policy

Patina must remain Preview2-operable while being Preview3-ready.

Rules:

- Define async semantics now where needed (`start/status/cancel/events`).
- Use polling/cursor fallback where runtime support is incomplete.
- Keep transport/mechanism separate from contract semantics.
- Version contracts when introducing true `future<T>` / `stream<T>` first-class APIs.

---

## 7) Compatibility and versioning guarantees

## 7.1 Approved toy guarantees

For `approved` toys:

- Minor/patch updates must preserve existing contract semantics.
- Breaking changes require major version bump.
- Deprecation must include migration guidance and timeline.

## 7.2 Community/local toy expectations

For `community-experimental` and `local` toys:

- No stability guarantee.
- Consumers opt in with explicit risk acceptance.

---

## 8) Approval authority and review roles

Minimum decision roles:

- **Contract reviewer**: WIT/API shape correctness.
- **Runtime reviewer**: Mother enforcement/fail-closed behavior.
- **SDK reviewer**: authoring ergonomics and consistency.
- **Security reviewer**: privilege model and abuse resistance.

A toy is only `approved` when all four signoffs exist.

---

## 9) SDK responsibilities (as MCT bible)

`patina-sdk` is not only helper functions; it is the construction contract for
building with MCT.

SDK must teach and enforce:

1. How to author typed children.
2. How to declare toys/scopes correctly.
3. When to request a new toy vs reuse existing capabilities.
4. How to pass approval gates for promotion.

---

## 10) Current repository alignment snapshot (2026-04-22)

## 10.1 Already aligned

- Canonical SDK direction is explicitly locked in [[sdk-vision-lock]].
- Toy registry exists at `wit/toys/deps/toys-registry.toml` with Preview2 + Patina entries.
- Mother registry parsing and tier handling exist (`src/commands/mother/toys.rs`).
- Child grant enforcement model exists (`[needs].toys` + optional scopes, fail-closed intent).

## 10.2 Gaps to close

1. **Promotion flow is policy-defined but not lifecycle-enforced**
   - Stages/gates are documented here, but candidate->approved transitions are not
     yet tied to a required command/check lane.

2. **Community intake lane is still missing**
   - Need explicit commands/workflow for install/validate/stage/promote from
     community toy packages.

3. **Deprecation windows are not enforced by tooling**
   - Metadata can now be represented, but runtime/CLI policy does not yet gate by
     deprecation deadlines.

4. **Preview3 migration tracking is not encoded per toy**
   - Need per-toy readiness/status for async/stream migration and compatibility
     adapter strategy.

---

## 11) Concrete next changes recommended

1. Add promotion/check commands:
   - Candidate validation, reviewer signoff capture, and explicit `candidate -> approved` transition flow.

2. Add community intake commands:
   - Controlled import/validation lane for community toy packages.

3. Add conformance lane:
   - Contract + enforcement tests required before `candidate -> approved`.

4. Enforce deprecation windows:
   - Honor `deprecation.remove_after` with explicit warnings/errors according to policy.

5. Add Preview3 transition rubric:
   - Track toy-by-toy async/stream readiness and versioning strategy.

---

## 12) Litmus test for creating a new toy

Before adding a toy, answer all:

1. Can existing WASI or approved toys satisfy this need?
2. Is this a real host-boundary capability gap?
3. Is this reusable across multiple children?
4. Is authority boundary explicit and least-privilege?
5. Can deny/fail-closed behavior be tested deterministically?

If any answer is "no", do not create/publish as candidate toy.

---

## References

- [SPEC.md](../../layer/surface/build/feat/sdk-vision-lock/SPEC.md)
- [SPEC.md](../../layer/surface/build/feat/sdk-developer-platform/SPEC.md)
- `wit/toys/deps/toys-registry.toml`
- `src/commands/mother/toys.rs`
