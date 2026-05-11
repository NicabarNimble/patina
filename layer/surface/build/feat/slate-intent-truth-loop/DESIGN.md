# Design: Slate Intent Truth Loop

## Why This Design

Slate is becoming the place where change work happens. The existing `patina spec` system already provides useful todo lifecycle mechanics, but the future Slate product should be grounded by Allium and beliefs rather than replacing them.

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

Slate should represent these current `spec` capabilities as workflow operations:

- discovery: create, list, ready, blocked, next, show, history,
- planning packets: prompt, handoff, packet,
- metadata/work shaping: set, rename, split, reopen,
- lifecycle movement: promote, pause, resume, block, abandon,
- closure: check, complete, archive.

Coverage is not blind old-output parity. Where Slate preserves old behavior, compatibility should hold. Where Slate intentionally changes the workflow to add Allium intent or belief anchoring, the divergence must be documented, tested, and exposed clearly.

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

6. **`spec` compatibility remains, but blind parity is not the goal**
   - Current `patina spec` and `slate-pando-migration` work remains useful as compatibility infrastructure.
   - Most current `spec` actions should be represented in Slate as todo/workflow mechanics.
   - Slate does not need 1:1 parity where the workflow is intentionally changing.
   - Allium is additive intent grounding for Slate, not a replacement for Slate's todo lifecycle.
   - This design defines the product direction: Slate as workbench, not new spec language.

## Commits

1. `spec: draft slate-intent-truth-loop` — created the system spec to lock the direction.
2. `feat: add slate work item model` — add packet-level Slate work item extraction and capability matrix to builtin packets and the Slate child dispatch path.
3. `feat: expose slate intent context over wit` — extend Slate WIT prompt/handoff surfaces with work item, Allium context, proof, capability matrix, and belief harvest fields.
4. `feat: gate slate readiness on intent alignment` — update spec creation templates and promotion readiness lint so build/fix/refactor Slates must capture human request, Allium intent, user alignment, and proof before becoming ready.
5. `feat: add slate allium tool orchestration` — expose Allium check/analyse/plan/model command plans and Allium skill workflow guidance in Slate packet/WIT context.
6. `feat: require slate belief harvest on completion` — add completion gates requiring explicit belief harvest/challenge decisions before non-forced build/fix/refactor Slate closure.

## Direct Code Targets

- `children/slate-manager/src/lib.rs` — current Slate child command handling and packet generation logic.
- `children/slate-manager/wit-contract/slate.wit` — typed Slate result surfaces for prompt/handoff/packet/work context.
- `children/slate-manager/wit/deps/patina-slate.wit` — generated/consumed WIT dependency surface.
- `src/commands/spec/mod.rs` — compatibility route from `patina spec` into Slate.
- `src/spec.rs` — legacy spec command value execution and route backend behavior.
- `src/commands/spec/internal/packets.rs` — current prompt/handoff/packet precedent.
- `src/commands/spec/internal/queries.rs` — current list/check/show/history mechanics.
- `src/commands/spec/internal/mutations.rs` — current lifecycle transitions and completion gates.
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

- Every useful existing `patina spec` command has a Slate workflow equivalent or a documented intentional divergence.
- Slate packet/readiness/closure surfaces expose Allium intent and belief anchoring where relevant.
- Compatibility modes still preserve old behavior when no intentional Slate workflow change applies.

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

- the first structured Slate work-item shape is accepted.

Current `spec` lifecycle mechanics have been reviewed and mapped in this draft. This spec follows and reframes `[[slate-pando-migration]]`: preserve compatibility where behavior remains, but document intentional Slate workflow changes instead of forcing blind parity.

## Open Questions

- Should Slate store Allium CLI outputs verbatim as artifacts, or store normalized summaries with links to raw artifacts?
- Which interface owns HITL dialogue state: Slate itself, the agent interface, or a shared session artifact?
- Should belief harvest be advisory only at first, or should completion require explicit accept/skip decisions for each recommendation?
