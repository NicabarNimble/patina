---
type: feat
id: mother-view-initial-shape-creation
status: ready
created: 2026-05-09
target: mother-view-composer
release_bump: patch
sessions:
  origin: 20260508-144836-859149000
related:
- mother-view-composer
- layer/allium/mother/mother-view-composer-target.allium
- mother-view-request-composer
- mother-view-shape-library
- mother-view-shape-adaptation
beliefs:
- '[[allium-as-agent-display-lisp]]'
- '[[allium-as-business-backlog]]'
exit_criteria:
- id: mvisc0-allium-code-alignment
  text: The spec starts from a documented Allium/code alignment pass over no-match request composition, shape creation, catalog validation, persistence, API, and daemon seams.
  checked: true
- id: mvisc1-creation-model
  text: Mother has structured initial-shape creation request/result data for no-match composition, including request id, created shape id, non-opening semantics, and explicit proposed requirements.
  checked: false
- id: mvisc2-catalog-guardrails
  text: Initial shapes are created only from explicit proposed requirements whose required fact paths are present in the Mother data catalog; missing/blank requirements fail closed without creating a shape.
  checked: false
- id: mvisc3-initial-shape-creation
  text: A no-match request with a valid structured initial-shape proposal creates an active exploratory ViewShape owned by the local Allium view library without arbitrary executable UI code.
  checked: false
- id: mvisc4-persistence
  text: Created initial shapes persist through the existing shape library and preserve modes, payload contract/version, source_ref, scope, requirements, and optional vision/project projections.
  checked: false
- id: mvisc5-compose-integration
  text: Request composition handles match_kind none by persisting the ShapeMatch and created initial ViewShape, returning a creation outcome instead of opening a buffer immediately.
  checked: false
- id: mvisc6-fail-closed-guardrails
  text: No proposal, non-none proposal, invalid payloads, missing catalog facts, and empty required-fact lists do not create shapes, open buffers, or invent data.
  checked: false
- id: mvisc7-tests-and-trace
  text: Deterministic tests cover successful creation, persisted initial shapes, API response shape, fail-closed behavior, and Allium obligation trace comments.
  checked: false
- id: mvisc8-follow-on-boundaries
  text: Behaviors outside this slice are explicitly left to request UX, revision, maturation, observability workflow, and renderer-frame specs.
  checked: false
---
# feat: Mother View Initial Shape Creation

> Create initial structured Mother view shapes when no existing shape matches a captured display request.

## Problem

[[mother-view-request-composer]] safely records `match_kind = none` as an unable outcome. That is correct fail-closed behavior, but the Allium target has a next rule: when no usable shape matches, Mother should request/create an initial shape artifact instead of leaving the request as an unstructured dead end.

Because Mother must not parse arbitrary natural language or invent facts, this slice needs a structured proposal path: agents may propose a shape title, modes, and explicit catalog-backed requirements; Mother validates those requirements against observed Mother data before creating shape metadata.

## Goal

Implement the bounded no-match creation slice:

1. Accept `match_kind = none` with a structured initial-shape proposal.
2. Require at least one explicit required fact path.
3. Validate proposed required facts against the Mother data catalog.
4. Create an active exploratory `ViewShape` owned by the local Allium view library.
5. Persist the created shape through the existing shape-library store/daemon path.
6. Return structured creation data to the caller.
7. Do **not** open a buffer automatically from the created shape.
8. Preserve no-fake-data and no-executable-UI guardrails.

## Status

Ready for implementation after Allium/code alignment.

## Allium authority

This spec implements the bounded initial creation slice of:

- `layer/allium/mother/mother-view-composer-target.allium`
- `layer/allium/mother/mother-view-composer-target.plan.json`

Primary Allium rule targeted:

- `CreateInitialShapeWhenNoShapeMatches`

Primary Allium entities used:

- `DisplayRequest`
- `ShapeMatch`
- `ViewShape`
- `ViewRequirement`
- `MotherDataCatalog`

## Non-Goals

- Do not parse natural language inside Mother.
- Do not generate Svelte/TypeScript/HTML or arbitrary renderer code.
- Do not open the created initial shape automatically in this slice.
- Do not create shapes from facts absent from the Mother data catalog.
- Do not implement user-facing editing/confirmation UX; that belongs to [[mother-view-request-ux]].
- Do not implement revisions/replacements; that belongs to [[mother-view-buffer-revision]].
- Do not implement maturation; that belongs to [[mother-view-maturation]].

## Target Shape

Given a no-match request with explicit proposed requirements:

```json
{
  "proposed_match": {
    "shape_id": null,
    "match_kind": "none",
    "confidence": 0.0
  },
  "proposed_initial_shape": {
    "title": "Mother Runtime Summary",
    "major_mode": "table",
    "minor_modes": ["pinned"],
    "requirements": [
      {
        "fact_path": "mother.status.version",
        "required": true,
        "purpose": "display Mother binary version"
      }
    ]
  }
}
```

Mother creates:

```text
shape_id: initial::<request_id>::<suffix>
title: Mother Runtime Summary
source_ref: local-allium-view-library
scope: mother_user
version: 1
active: true
major_mode: table
minor_modes: [pinned]
maturity: exploratory
payload_contract: framed_json
payload_version: 1
requirements: explicit proposed catalog-backed requirements
```

The response records creation, but does not open a buffer until a later explicit/exact request selects the new shape.

## Solution

Extend the existing request-composer path:

- add structured initial-shape proposal input;
- add structured creation result output;
- when `match_kind = none`, require the proposal;
- require non-empty title and non-empty required requirements;
- require each required fact path to exist in `DataCatalog`;
- create an exploratory `ViewShape` from the proposal;
- return the shape and creation result;
- have the daemon persist the created shape.

Because Allium `DisplayRequest.outcome` currently has only `pending | buffer_opened | observability_gap_reported | unable`, this slice does not add a new persisted outcome enum. Creation is represented by response data plus persisted `ShapeMatch` and `ViewShape` rows.

## Implementation Order

1. Add initial-shape proposal/result model types.
2. Extend composition response with optional initial-shape creation and created shape fields.
3. Implement no-match creation with catalog-backed requirement validation.
4. Wire daemon persistence for created shapes.
5. Add deterministic service, store/daemon, and HTTP tests.
6. Update follow-on boundaries and umbrella tracking.

## Resolved Decisions

- Initial creation creates a shape; it does not open a buffer automatically.
- Created shapes start `maturity = exploratory`.
- Mother accepts structured proposals only; it does not infer facts from prose.
- Required facts must already be catalogued. Missing facts are refused here; richer observability-improvement proposals belong to [[mother-view-observability-workflow]].
- `DisplayRequestOutcome` remains Allium v1-compatible; creation is represented by response data and persisted shape/match records.

## Follow-on Backlog

- [[mother-view-request-ux]] — user confirmation/editing of created initial shapes.
- [[mother-view-buffer-revision]] — corrections and replacement flows.
- [[mother-view-observability-workflow]] — shape proposals that require missing facts and linked observability work.
- [[mother-view-maturation]] — candidate/stable/promotion behavior.
- [[mother-sveltekit-frame]] — visible renderer/frame attachment.

## Verification

```bash
cargo check -q
cargo test -q -p mother view_initial_shape_creation
cargo test -q -p mother view_request
cargo test -q -p mother view_buffer
patina spec check mother-view-initial-shape-creation --json
allium check layer/allium/mother/mother-view-composer-target.allium
```

## Exit Criteria

- [x] `mvisc0-allium-code-alignment`
- [ ] `mvisc1-creation-model`
- [ ] `mvisc2-catalog-guardrails`
- [ ] `mvisc3-initial-shape-creation`
- [ ] `mvisc4-persistence`
- [ ] `mvisc5-compose-integration`
- [ ] `mvisc6-fail-closed-guardrails`
- [ ] `mvisc7-tests-and-trace`
- [ ] `mvisc8-follow-on-boundaries`

## Build Readiness

Ready to promote after the matching design document records code seams and implementation boundaries.
