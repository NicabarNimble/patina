# Six-Child BA Alignment Matrix

> Maps the six canon children to BA/WASI/component-model primitives and Patina deltas.

## Legend

- **Aligns:** directly follows primitive intent
- **Extends:** uses Patina-specific delta intentionally
- **Gap:** known deviation or missing piece to resolve

## Summary

- Overall: strong alignment with typed WIT composition direction
- Main intentional deltas: `patina:records`, `patina:measure`
- Main gap to watch: catalog lane currently keyvalue-centric (not yet SQL-facing in child world)

## Matrix

| Child | World imports/exports (contract) | Primitive alignment | Delta/Gaps |
|---|---|---|---|
| `file-system-monitor` | imports `wasi:logging`, `patina:measure`; exports `patina:records/source` | **Aligns** P1/P2 (typed world), **Aligns** P3 for logging | **Extends** with `patina:records` and `patina:measure` |
| `content-extractor` | imports `wasi:logging`; exports `patina:records/extract` | **Aligns** P1/P2/P3 | **Extends** with `patina:records` |
| `schema-enforcer` | imports `wasi:logging`, `patina:measure`; exports `patina:records/transform` | **Aligns** P1/P2/P3, **Aligns** P4 via host gating model | **Extends** with `patina:records` + `patina:measure` |
| `dedup-filter` | imports `wasi:logging`, `wasi:keyvalue`, `patina:measure`; exports `patina:records/transform` | **Aligns** P1/P2/P3 strongly (standard keyvalue + typed WIT) | **Extends** with `patina:records` + `patina:measure` |
| `record-writer` | imports `wasi:logging`, `wasi:keyvalue`, `patina:measure`; exports `patina:records/write` | **Aligns** P1/P2/P3; explicit typed write contract | **Extends** with `patina:records` + `patina:measure` |
| `lakehouse-catalog` | imports `wasi:logging`, `wasi:keyvalue`; exports `patina:records/catalog` | **Aligns** P1/P2/P3 partially | **Gap:** catalog currently uses keyvalue only in child contract; SQL-facing trajectory should be explicit if required by roadmap |

## Primitive mapping

- **P1 (component composition boundaries):** all six use typed world exports/imports.
- **P2 (WIT contracts):** all six publish typed `patina:records/*` interfaces.
- **P3 (WASI-first):** logging/keyvalue use WASI-shaped contracts where present.
- **P4 (authority/least privilege):** Mother-side grant checks and call-time gating enforce fail-closed behavior.
- **P5 (evidence-driven evolution):** this matrix is input to BA beliefs and direction snapshots.

## Decisions enabled by this matrix

1. Keep six children as canonical typed baseline (vision-lock).
2. Treat `patina:records` and `patina:measure` as intentional deltas requiring explicit rationale and evolution tracking.
3. Track catalog lane decision (keyvalue-only vs SQL-integrated contract) as a deliberate roadmap choice, not accidental drift.

## Next grounding actions

- Write/refresh at least one belief from this matrix:
  - `ba-aligns-typed-child-composition.md` (likely)
  - `ba-extends-record-domain-contract.md` (likely)
  - `ba-diverges-<topic>.md` only if a true deliberate divergence is accepted
