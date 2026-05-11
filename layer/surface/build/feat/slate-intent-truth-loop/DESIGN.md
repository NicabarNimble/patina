# Design: Slate Intent Truth Loop

## Why This Design

Slate is becoming the place where change work happens. `patina spec` remains its own Markdown/git system; Slate must be its own project-living WIT/WASI work system grounded by Allium and beliefs rather than implemented as a `spec` projection.

The central design rule is:

> Slate manages the work transaction. Allium holds intended behavior. Beliefs hold defeasible doctrine and evidence.

This keeps each artifact honest:

- Slate does not become a second specification language.
- Allium does not become unchallenged stale truth.
- Beliefs do not become unpruned doctrine.
- Existing Allium and belief mechanisms remain authoritative for their own domains.

## Build Target

Implement Slate as a structured build/refactor/fix todo system with an intent/proof loop. This is a full-capability target, not a reduced MVP: Slate should cover the useful current `patina spec` lifecycle surface while intentionally improving workflow around Allium and beliefs.

### Capability coverage

Slate should provide native project-living workflow operations with comparable usefulness to these work-management capabilities:

- discovery: create, list, ready, blocked, next, show, history,
- planning packets: prompt, handoff, packet,
- metadata/work shaping: set, rename, split, reopen,
- lifecycle movement: promote, pause, resume, block, abandon,
- closure: check, complete, archive.

Coverage is not old-output parity and must not depend on `SPEC.md` paths. `spec` and Slate are islands; explicit import/export bridges may exist, but Slate's canonical operations target `layer/slate/` artifacts and Mother Slate projections.

### Slate work-item fields

Initial conceptual record:

```text
SlateWorkItem
  id
  title
  work_kind: build | refactor | fix
  human_request
  status: draft | ready | active | blocked | complete
  allium_context
  user_intent_alignment
  relevant_beliefs
  core_doctrine_refs
  implementation_plan
  proof_plan
  execution_evidence
  drift_classification
  belief_harvest
```

### Allium context

```text
AlliumContext
  files: list<path>
  constructs: entities/rules/surfaces/contracts/invariants/open_questions
  check_summary
  analyse_findings
  model_summary
  plan_obligations
  intent_status:
    already_matches
    needs_update
    missing
    ambiguous
    not_behavioral_refactor
```

Slate should collect this from existing Allium CLI/skill workflows and store summaries/links, not reinterpret Allium itself.

### User intent alignment

```text
UserIntentAlignment
  aligned: bool
  captured_at
  captured_by
  statement
  allium_delta_required: bool
  unresolved_questions: list<string>
```

If intended behavior is missing or disputed, Slate blocks until user alignment is captured or the Slate is abandoned.

### Belief harvest

```text
BeliefHarvest
  relevant_existing: list<wikilink>
  evidence_to_add: list<path-or-commit-or-session>
  proposed_new_beliefs: list<statement>
  proposed_scopes: list<wikilink>
  proposed_attacks: list<wikilink>
  proposed_defeats_or_archives: list<wikilink>
```

Slate recommends and records; the existing belief files, `patina scrape`, and belief audit/graph machinery remain the belief system.

## Resolved Decisions

1. **Allium first during Slate creation**
   - The HITL dialogue asks what behavior is intended and whether Allium already says it.
   - Beliefs can constrain the work, but new beliefs generally come after proof.

2. **Slate blocks on unclear truth**
   - If user intent and Allium disagree, Slate does not silently choose.
   - It routes to Allium tending/elicitation or records an explicit non-behavioral refactor reason.

3. **Refactor is special**
   - Refactor work may intentionally leave Allium unchanged.
   - Slate must still capture behavior-preservation intent and proof.

4. **Fix is classification-heavy**
   - Fix work must classify mismatch as code bug, Allium stale, belief stale, ambiguous intent, or implementation-only detail.

5. **Belief updates are evidence-backed**
   - New beliefs should be harvested from proof, repeated pattern, failure mode, or architectural lesson.
   - Beliefs with missing/changing proof should be challenged, scoped, defeated, or archived through existing conventions.

6. **`spec` and Slate are islands**
   - `patina spec` remains a standalone Markdown/git spec system.
   - Slate owns native WIT/WASI operations, project-living artifacts, lifecycle semantics, and Mother projections.
   - Explicit bridges can import/export between islands, but `SPEC.md`/`DESIGN.md` are not Slate storage.
   - Allium is additive intent grounding for Slate, not a replacement for Slate's todo lifecycle.
   - This design defines the product direction: Slate as workbench, not new spec language.

## Commits

1. `spec: draft slate-intent-truth-loop` — created the system spec to lock the direction.
2. `feat: add slate work item model` — first bridge-state exploration of Slate work-item extraction and capability matrix.
3. `feat: expose slate intent context over wit` — first bridge-state WIT prompt/handoff surface expansion.
4. `feat: gate slate readiness on intent alignment` — superseded by the island decision; readiness belongs in Slate, not `patina spec`.
5. `feat: add slate allium tool orchestration` — keep the Allium orchestration shape, but move it to native Slate records.
6. `feat: require slate belief harvest on completion` — superseded by the island decision; completion harvest belongs in Slate, not `patina spec`.
7. `test: cover slate intent and belief gates` — superseded for spec-side gates; native Slate tests should cover the same behavior in Slate.
8. `feat: add project-living slate store foundation` — restore `patina spec` files to their island, add native Slate work WIT operations, project-owned `layer/slate/work/<id>/work.toml` artifacts, and per-project Mother `slate.db` projection scaffolding.

## Direct Code Targets

- `children/slate-manager/src/lib.rs` — native Slate child WIT operations and remaining explicit bridge code.
- `children/slate-manager/wit-contract/slate.wit` — native Slate work records and lifecycle surfaces.
- `children/slate-manager/wit/deps/patina-slate.wit` — consumed WIT dependency surface.
- `children/slate-manager/child.toml` — Slate child contract defaults to native work operations.
- `src/slate.rs` — Mother per-project Slate projection store for project-living Slate artifacts.
- `src/paths.rs` — per-project Mother `slate.db` path.
- `layer/slate/` — project-owned durable Slate work artifacts.
- `src/commands/scrape/beliefs/mod.rs` — existing belief indexing, health, contestation, and evidence metrics to consume, not replace.
- `src/commands/scrape/beliefs/verification/` — existing belief verification mechanism to consume, not replace.

## Verification Plan

### Static validation

```bash
patina spec check slate-intent-truth-loop --json
cargo check -q --workspace
```

### Targeted tests

```bash
cargo test -q -p patina-ai-child-slate-manager
cargo test -q --lib spec
cargo test -q --lib belief
```

### Capability coverage checks

- Every useful work-management capability has a Slate-native workflow equivalent or a documented intentional divergence.
- Slate-native WIT/readiness/closure surfaces expose Allium intent and belief anchoring where relevant.
- `patina spec` remains independent; any bridge is explicit and non-authoritative.

### Behavioral fixtures

Create or extend fixtures for:

1. **Build Slate**
   - User requests a new behavior.
   - Slate finds missing Allium intent.
   - Slate blocks until Allium intent is added/confirmed.
   - Slate packet includes proof obligations.

2. **Refactor Slate**
   - User requests structure change without behavior change.
   - Slate records Allium no-change rationale.
   - Slate closes only after behavior-preservation proof.

3. **Fix Slate**
   - User dislikes observed behavior.
   - Slate compares user intent, Allium, implementation, and beliefs.
   - Slate classifies the mismatch before changing code.

4. **Belief harvest**
   - Work proof supports an existing belief: Slate recommends adding evidence.
   - Work proof defeats or scopes a belief: Slate recommends attack/scope/defeat using existing belief conventions.
   - No evidence-backed lesson emerges: Slate recommends no belief change.

5. **Allium stale truth**
   - Allium states old behavior but user confirms new business intent.
   - Slate requires Allium update before completion.

## Build Readiness

Ready when:

- the first structured, project-living Slate work-item shape and Mother projection are accepted.

Current `spec` lifecycle mechanics have been reviewed as precedent only. This spec now treats `[[slate-pando-migration]]` as compatibility exploration, not as Slate's substrate.

## Open Questions

- Should Slate store Allium CLI outputs verbatim as artifacts, or store normalized summaries with links to raw artifacts?
- Which interface owns HITL dialogue state: Slate itself, the agent interface, or a shared session artifact?
- Should belief harvest be advisory only at first, or should completion require explicit accept/skip decisions for each recommendation?
