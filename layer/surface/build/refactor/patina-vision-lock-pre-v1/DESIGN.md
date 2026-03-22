# Design: refactor: Patina Vision Lock Pre-v1

## Why This Design

Pre-v1 has enough implementation to move quickly, but not enough governance to prevent semantic drift. This design prioritizes trust infrastructure: contract clarity, scaffold truth, lifecycle safeguards, and reproducible evidence.

## Build Target

Produce a spec/runtime system where completion signals cannot outpace architecture truth.

Concretely:

- architecture semantics are explicit and consistent,
- canonical paths are scaffold-free,
- spec lifecycle requires human confirmation for irreversible transitions,
- and EC closure is reproducible from evidence artifacts.

## Resolved Decisions

- Preserve history; reconcile through explicit amendments and lock gates.
- Add features to spec lifecycle only when they improve trust (`rename`, `reopen`, HITL complete/abandon).
- Treat spec-manager child migration as a decision with bootstrap/recovery constraints, not an assumed rewrite.

## Commits
1. `spec: lock vision contract and runtime policy` — codify protocol/Mother/child semantics and living-vs-snapshot requirement.
2. `audit: inventory scaffold paths and canonical command truth` — produce baseline capability and scaffold report.
3. `spec: add reopen and rename lifecycle commands` — enable deterministic correction workflows.
4. `spec: require human confirmation for complete/abandon` — prevent premature irreversible transitions.
5. `spec: enforce criteria amendment metadata after active` — prevent silent gate drift.
6. `spec: enforce evidence tiers for EC closure` — require command/output/artifact proof.
7. `doc: publish mother capability map and link from specs` — separate real vs partial vs deferred.
8. `decision: spec-manager child migration ADR + bootstrap plan` — choose route with recovery guarantees.
9. `test: zero-context reproducibility harness` — independent reviewer pass/fail parity.

## Direct Code Targets
- `src/commands/spec/mod.rs`
- `src/commands/spec/internal/`
- `src/commands/spec/internal/lifecycle.rs` (or equivalent lifecycle modules)
- `src/commands/spec/internal/queries.rs`
- `src/commands/spec/internal/archive.rs`
- `src/commands/spec/internal/mutations.rs`
- `src/commands/spec/internal/packets.rs`
- `src/commands/spec/internal/split.rs`
- `layer/surface/build/refactor/patina-vision-lock-pre-v1/SPEC.md`
- `layer/surface/build/refactor/patina-vision-lock-pre-v1/DESIGN.md`
- `layer/surface/build/refactor/patina-vision-lock-pre-v1/CAPABILITY-MAP.md`
- (if chosen) `children/spec-manager/` and associated WIT/toy interfaces

## Verification Plan

1. **Lifecycle correctness tests**
   - complete/abandon blocked without HITL confirmation token.
   - `rename` updates id/path/references and preserves history.
   - `reopen` restores from archive with status transition safety.

2. **Criteria governance tests**
   - active spec criteria edits require amendment block (`amended_at`, `amended_by`, `rationale`).

3. **Evidence-tier checks**
   - EC closure fails when proof command or output artifact is missing.

4. **Scaffold truth checks**
   - canonical command route smoke test fails if placeholder text appears.

5. **Independent review pass**
   - zero-context reviewer receives packet and reproduces same pass/fail gate outputs.

## Build Readiness

Ready to execute now; this is governance-first and intentionally bounded. It should run before further pre-v1 closure/follow-on specs are finalized.

## Open Questions

- Should `spec-manager` child become mandatory in this spec, or should this spec produce a signed decision + phased migration spec?
- If Mother-required living mode is adopted, what exact CLI behavior is allowed in snapshot mode (read-only commands vs hard fail)?
