# Legacy and Grammar Disposition Matrix

## Scope

Disposition matrix for child lanes under `legacy-and-grammar-disposition`.

Current progress in this artifact:
- Inventory coverage (lgd1)
- Legacy service-child decision coverage (lgd2)
- Grammar lane contract coverage (lgd3)
- Owner/target/dependency coverage for migrate/retire decisions (lgd4)
- Explicit spec-manager path coverage (lgd5)
- No-implicit-carryover coverage via matrix + validation script (lgd6)

## Typed baseline children (current typed lane)

| child | kind | sdk lane | current runtime lane | risk notes |
|---|---|---|---|---|
| file-system-monitor | child | patina-sdk | typed child | Low: typed lane active; monitor WIT dependency drift |
| content-extractor | child | patina-sdk | typed child | Low: typed lane active; monitor records/extract contract drift |
| schema-enforcer | child | patina-sdk | typed child | Low: typed lane active; monitor transform contract evolution |
| dedup-filter | child | patina-sdk | typed child | Low: typed lane active; monitor keyvalue + transform contract stability |
| record-writer | child | patina-sdk | typed child | Low: typed lane active; monitor write contract + provenance fields |
| lakehouse-catalog | child | patina-sdk | typed child | Low: typed lane active; monitor catalog contract + sql host assumptions |

## Legacy service children (current legacy lane)

Decision vocabulary in this matrix:
- **KEEP (bounded)**: remain on legacy lane for now to preserve stability.
- **MIGRATE (phased)**: move to typed/modern lane in a planned follow-on spec slice.
- **RETIRE/REPLACE**: remove legacy child lane and use replacement runtime surface.

| child | kind | sdk lane | current runtime lane | disposition decision | owner | target window | dependency notes | rationale |
|---|---|---|---|---|---|---|---|---|
| belief-verifier | child | patina-sdk-legacy | service handle lane | MIGRATE (phased) | child-runtime | svl9 migration-playbook window | typed service-child contract follow-on; `sdk-vision-lock` svl9/svl11 | Keep behavior active, but move off legacy SDK once typed service-child contract (including event/task/drain semantics) is explicitly locked. |
| session-writer | child | patina-sdk-legacy | service handle lane | KEEP (bounded) | interface-session | review at next sdk-vision-lock disposition checkpoint | interface/session authority and workflow stability slices | Session lifecycle remains sensitive to interface/runtime orchestration changes; keep stable for now and revisit after current interface/session roadmap slices settle. |
| spec-manager | child | patina-sdk-legacy | service handle lane (with builtin overlap today) | MIGRATE (phased) | spec-runtime | phase after SQL/git toy prerequisites | `spec-manager-wasm-child`; typed SQL/git toy prerequisites; `sdk-vision-lock` svl9/svl11 | End split authority by converging on one wasm child path; maintain current behavior until SQL/git toy prerequisites and parity path are executed (`spec-manager-wasm-child`). |
| doctor | child | patina-sdk-legacy | service handle lane | RETIRE/REPLACE | mother-runtime | next service-lane pruning window before sdk-vision-lock completion | host-native doctor runtime parity (`src/mother/doctor_runtime.rs`) | Doctor behavior already has host-native runtime path; reduce duplicate authority by retiring legacy child lane in favor of the host-native doctor surface. |

### Spec-manager explicit path (lgd5)

Spec-manager path is intentionally two-phase and explicit:

1. **Temporary state (remain service-handle lane):** keep current service-handle behavior to preserve working spec lifecycle operations while prerequisites are incomplete.
2. **Target state (migrate to wasm child path):** converge on one wasm child authority path once SQL/git toy prerequisites and parity tests are in place.

Governing specs linked:
- `layer/surface/build/explore/spec-manager-wasm-child/SPEC.md`
- `layer/surface/build/feat/sdk-vision-lock/SPEC.md`
- `layer/surface/build/refactor/legacy-and-grammar-disposition/SPEC.md`

### Captured note: doctor belongs to native Mother service boundary

Keep this rationale explicit to prevent re-drift back into legacy child lane:

- Current production execution for doctor is already host-native (`src/commands/doctor.rs` → builtin child action; `mother/src/builtin_children.rs`; `src/mother/doctor_runtime.rs`).
- Doctor needs host-level introspection (environment tools, git/worktree state, project/event-store integrity).
- Re-forcing doctor through a child lane would either broaden toy authority too far or degrade diagnostic coverage.
- Therefore doctor disposition remains **RETIRE/REPLACE legacy child lane in favor of native Mother doctor service**.

## Grammar children (current pipeline lane)

| child | kind | sdk lane | current runtime lane | lane contract decision | risk notes |
|---|---|---|---|---|---|
| grammar-c | pipeline | n/a (grammar crate) | pipeline lane | KEEP (bounded, containment) | Medium: pipeline-only contract; contained until typed integration decision checkpoint |
| grammar-cairo | pipeline | n/a (grammar crate) | pipeline lane | KEEP (bounded, containment) | Medium: pipeline-only contract; contained until typed integration decision checkpoint |
| grammar-cpp | pipeline | n/a (grammar crate) | pipeline lane | KEEP (bounded, containment) | Medium: pipeline-only contract; contained until typed integration decision checkpoint |
| grammar-go | pipeline | n/a (grammar crate) | pipeline lane | KEEP (bounded, containment) | Medium: pipeline-only contract; contained until typed integration decision checkpoint |
| grammar-javascript | pipeline | n/a (grammar crate) | pipeline lane | KEEP (bounded, containment) | Medium: pipeline-only contract; contained until typed integration decision checkpoint |
| grammar-python | pipeline | n/a (grammar crate) | pipeline lane | KEEP (bounded, containment) | Medium: pipeline-only contract; contained until typed integration decision checkpoint |
| grammar-rust | pipeline | n/a (grammar crate) | pipeline lane | KEEP (bounded, containment) | Medium: pipeline-only contract; contained until typed integration decision checkpoint |
| grammar-solidity | pipeline | n/a (grammar crate) | pipeline lane | KEEP (bounded, containment) | Medium: pipeline-only contract; contained until typed integration decision checkpoint |
| grammar-typescript | pipeline | n/a (grammar crate) | pipeline lane | KEEP (bounded, containment) | Medium: pipeline-only contract; contained until typed integration decision checkpoint |

### Grammar lane contract (lgd3)

Long-term contract for grammar lane in this phase: **legacy pipeline containment (bounded)**.

Constraints while contained:
- No expansion of grammar capability scope beyond pipeline parsing contract in this phase.
- Grammar children stay pipeline-lane components (`kind = "pipeline"`) and remain isolated from typed child composition wiring by default.
- Any cross-lane integration must occur through explicit adapter/composition seams tracked by typed-composition specs, not ad hoc runtime coupling.

Owner and target milestones:
- **Owner:** grammar-runtime
- **Milestone A:** maintain containment through current `child-typed-composition` execution window.
- **Milestone B:** at `sdk-vision-lock` migration-playbook/disposition checkpoints (`svl9`, `svl11`), re-evaluate whether grammar lane remains contained or gets a typed integration plan.
