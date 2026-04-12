# Design: refactor: Legacy service children + grammar lane disposition

## Why This Design

This spec is a policy/disposition lock, not runtime migration code.
The design centers on one auditable matrix artifact with deterministic guard checks so disposition drift is fail-closed.

## Build Target

- One committed matrix defining disposition for:
  - typed baseline children
  - legacy service children
  - grammar children
- Explicit decision vocabulary (`KEEP`, `MIGRATE`, `RETIRE/REPLACE`).
- Explicit owner/target/dependency metadata for migrate/retire paths.
- Explicit spec-manager two-phase path.
- Explicit doctor native-service anchor note.
- Guard script that fails when disposition entries regress/missing placeholders appear.

## Resolved Decisions

- Doctor remains a native Mother service boundary; legacy doctor child lane is retire/replace.
- Spec-manager migration is phased and prerequisite-gated (SQL/git toy path).
- Grammar lane is bounded containment in this phase, with milestone re-evaluation.

## Commits

1. `refactor(disposition): lock lgd1 inventory matrix` — adds matrix inventory baseline.
2. `refactor(disposition): lock lgd2 and lgd3 decisions` — adds service decisions and grammar lane contract.
3. `refactor(disposition): close lgd4-lgd6 with matrix guard` — adds owner/target/dependency fields, explicit spec-manager path, and guard script.

## Direct Code Targets

- `layer/surface/build/refactor/legacy-and-grammar-disposition/SPEC.md` — criteria state + verification contract.
- `layer/surface/build/refactor/legacy-and-grammar-disposition/MATRIX.md` — disposition source-of-truth artifact.
- `resources/scripts/check-legacy-disposition-matrix.sh` — deterministic policy guard.

## Verification Plan

```bash
patina spec check legacy-and-grammar-disposition --json
test -s layer/surface/build/refactor/legacy-and-grammar-disposition/MATRIX.md
bash resources/scripts/check-legacy-disposition-matrix.sh
```

## Build Readiness

High for policy closure.

## Open Questions

None for this spec closure phase.
