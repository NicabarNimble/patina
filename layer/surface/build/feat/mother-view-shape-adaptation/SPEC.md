---
type: feat
id: mother-view-shape-adaptation
status: active
created: 2026-05-09
sessions:
  origin: 20260508-144836-859149000
related:
- layer/allium/mother/mother-view-composer-target.allium
- mother-view-composer
- mother-view-request-composer
- mother-view-shape-library
beliefs:
- '[[allium-as-agent-display-lisp]]'
- '[[allium-as-business-backlog]]'
exit_criteria:
- id: mvsa0-allium-code-alignment
  text: The spec starts from a documented Allium/code alignment pass over similar-match, request-composer, shape-library, persistence, API, and daemon seams.
  checked: true
- id: mvsa1-adaptation-model
  text: Mother has structured adaptation result data for similar-shape composition, including precedent shape id, adapted shape id, and non-open request outcome semantics.
  checked: true
- id: mvsa2-adapted-shape-creation
  text: A similar match above the confidence threshold creates a new exploratory active ViewShape derived from the precedent shape without arbitrary executable UI code.
  checked: true
- id: mvsa3-adapted-shape-persistence
  text: Adapted shapes persist through the existing shape library and preserve requirements, mode metadata, payload contract/version, source_ref, scope, and optional vision/project projections.
  checked: true
- id: mvsa4-compose-integration
  text: Request composition handles similar matches by persisting the ShapeMatch and adapted ViewShape, returning an adaptation outcome instead of opening a buffer immediately.
  checked: true
- id: mvsa5-fail-closed-guardrails
  text: Missing/inactive precedent shapes, low-confidence similar matches, and invalid adaptation payloads do not create shapes, open buffers, or invent data.
  checked: true
- id: mvsa6-tests-and-trace
  text: Deterministic tests cover successful adaptation, persisted adapted shapes, low-confidence/missing/inactive fail-closed behavior, and Allium obligation trace comments.
  checked: true
- id: mvsa7-follow-on-backlog
  text: Behaviors outside this slice are explicitly split into follow-on specs for adapted-shape editing UX, opening an adapted shape after user/agent confirmation, initial shape creation, revision, maturation, and renderer frames.
  checked: true
validated_against_commit: f35b5a01b3bef4ca2f8a2618cd7eb282dee5ba93
last_freshness_check: 2026-05-09T17:40:48Z
freshness_scope:
- Allium target, request composer, shape library, service composition, daemon/store persistence, HTTP API tests, fail-closed guardrails
---
# feat: Mother View Shape Adaptation

> Adapt similar existing Mother view shapes into new exploratory shapes when exact matches do not satisfy a display request.

## Problem

[[mother-view-request-composer]] intentionally persists `similar` matches but does not create new shapes or open buffers from them. That keeps the request composer safe, but the Allium target has a next rule: when an agent proposes a sufficiently similar shape, Mother should create an adapted exploratory shape as structured metadata.

Without this slice, the request path can say “similar adaptation is deferred” but cannot progress toward a usable local shape.

## Goal

Implement the next bounded Allium slice:

1. Accept a `similar` proposed match above the configured confidence threshold.
2. Resolve the precedent shape from the Mother shape library.
3. Create an adapted exploratory `ViewShape` from the precedent shape.
4. Persist the adapted shape through the existing shape library.
5. Return structured adaptation data to the caller.
6. Do **not** open a buffer automatically from the adapted shape in this slice.
7. Preserve no-fake-data and no-executable-UI guardrails.

## Status

Ready for implementation after Allium/code alignment.

## Allium authority

This spec implements the bounded adaptation slice of:

- `layer/allium/mother/mother-view-composer-target.allium`
- `layer/allium/mother/mother-view-composer-target.plan.json`

Primary Allium rule targeted:

- `AdaptSimilarShapeWhenNoExactShapeExists`

Primary Allium entities used:

- `DisplayRequest`
- `ShapeMatch`
- `ViewShape`
- `ViewRequirement`

## Non-Goals

- Do not parse natural language inside Mother.
- Do not generate Svelte/TypeScript/HTML or arbitrary renderer code.
- Do not open an adapted shape automatically in this slice.
- Do not implement no-match initial shape creation; that belongs to [[mother-view-initial-shape-creation]].
- Do not implement user corrections/revisions; that belongs to [[mother-view-buffer-revision]].
- Do not implement renderer UX; that belongs to [[mother-view-request-ux]] and [[mother-sveltekit-frame]].

## Target Shape

Given a similar match:

```text
request_id: req_...
shape_id: mother.status.default
match_kind: similar
confidence: 0.82
```

Mother creates a structured adapted shape:

```text
shape_id: mother.status.default::adapted::<suffix>
title: Adapted Mother Status
source_ref: local-allium-view-library
scope: mother_user
version: 1
active: true
major_mode: <copied from precedent>
minor_modes: <copied from precedent>
maturity: exploratory
payload_contract: framed_json
payload_version: 1
requirements: <copied from precedent for this first slice>
```

The response records adaptation, but does not open a buffer until a later explicit/exact request selects the adapted shape.

## Solution

Extend the existing request-composer path:

- add an adaptation outcome to composition results;
- when `match_kind = similar`, require `confidence >= 0.60`;
- require `shape_id` to identify an active precedent shape;
- derive a new exploratory shape from the precedent using Allium-compatible fields;
- persist the adapted shape through the shape-library store/daemon path;
- keep request outcome as `unable` or an explicit non-open adaptation status until the model grows a first-class outcome beyond Allium v1.

Because Allium `DisplayRequest.outcome` has only `pending | buffer_opened | observability_gap_reported | unable`, this slice should not invent a new persisted outcome enum. Use structured response fields and persisted match/shape data to represent adaptation.

## Implementation Order

1. Confirm current request-composer similar-match fail-closed behavior.
2. Add adaptation response/model data without changing existing Allium outcome enums.
3. Implement adapted shape derivation in `ViewBufferService`.
4. Wire daemon persistence of adapted shapes.
5. Add deterministic tests for success and guardrails.
6. Update follow-on backlog.

## Resolved Decisions

- Adaptation creates a shape; it does not open a buffer automatically.
- Adapted shapes start `maturity = exploratory`.
- Adapted shapes copy precedent requirements in this first slice; requirement editing/composition can come later.
- `DisplayRequestOutcome` remains Allium v1-compatible; adaptation is represented by response data and persisted shape/match records.

## Follow-on Backlog

- [[mother-view-request-ux]] — adapted-shape confirmation, editing, and request UX.
- [[mother-view-initial-shape-creation]] — no-match initial shape creation.
- [[mother-view-buffer-revision]] — user corrections and replacement flows.
- [[mother-view-maturation]] — candidate/stable/promotion behavior for adapted shapes.
- [[mother-sveltekit-frame]] — visible renderer/frame attachment.
- Later request-composer UX/API slice — explicit opening of adapted shapes after user or agent confirmation.

## Verification

```bash
cargo check -q
cargo test -q -p mother view_shape_adaptation
cargo test -q -p mother view_request
cargo test -q -p mother view_buffer
patina spec check mother-view-shape-adaptation --json
allium check layer/allium/mother/mother-view-composer-target.allium
```

## Exit Criteria

- [x] `mvsa0-allium-code-alignment`
- [x] `mvsa1-adaptation-model`
- [x] `mvsa2-adapted-shape-creation`
- [x] `mvsa3-adapted-shape-persistence`
- [x] `mvsa4-compose-integration`
- [x] `mvsa5-fail-closed-guardrails`
- [x] `mvsa6-tests-and-trace`
- [x] `mvsa7-follow-on-backlog`

## Build Readiness

Ready to promote as the next implementation slice after the matching design document records the alignment pass and required design sections.
