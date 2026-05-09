---
type: feat
id: mother-view-request-composer
status: ready
created: 2026-05-09
sessions:
  origin: 20260508-144836-859149000
related:
- layer/allium/mother/mother-view-composer-target.allium
- layer/surface/done/feat/mother-view-shape-library/SPEC.md
- mother/src/view_buffer
beliefs:
- '[[spec-driven-design]]'
- '[[allium-as-agent-display-lisp]]'
- '[[allium-as-business-backlog]]'
exit_criteria:
- id: mvrc0-allium-code-alignment
  text: The spec starts from a documented Allium/code alignment pass over request, match, shape-library, buffer-open, store, API, route, and daemon seams.
  checked: true
- id: mvrc1-request-model
  text: Mother has first-class DisplayRequest and ShapeMatch model records aligned with Allium v1 fields, including pending/buffer_opened/observability_gap_reported/unable outcomes and exact/explicit_user_choice/similar/none match kinds.
  checked: true
- id: mvrc2-request-persistence
  text: Display requests and shape matches persist in Mother state with deterministic create/get/list/update behavior independent of renderer state.
  checked: true
- id: mvrc3-compose-api
  text: Mother exposes a structured request-composition API that captures raw user display requests and accepts agent-proposed shape matches without parsing natural language inside Mother.
  checked: true
- id: mvrc4-explicit-exact-open
  text: Explicit and exact active-shape matches can open buffers through the existing shape-library path, preserve required-data validation, and update request outcome to buffer_opened or observability_gap_reported.
  checked: true
- id: mvrc5-fail-closed-outcomes
  text: Missing, inactive, low-confidence, similar-only, or no-match proposals do not invent data or open buffers; they persist request/match state and return unable or follow-on requested outcomes.
  checked: true
- id: mvrc6-tests-and-trace
  text: Deterministic tests cover request capture, match persistence, explicit/exact open, missing/inactive/low-confidence fail-closed behavior, and Allium obligation trace comments.
  checked: true
- id: mvrc7-follow-on-backlog
  text: Behaviors outside this slice are explicitly split into follow-on specs for similar-shape adaptation, initial-shape creation, request UX, revision, maturation, and renderer frames.
  checked: true
validated_against_commit: 0cbc03c7997a5b5d506dd672a7f891a19a24105e
---
# feat: Mother View Request Composer

> Persist user display requests and agent-proposed shape matches, then compose explicit/exact matches into Mother-owned view buffers without inventing data.

## Problem

[[mother-view-shape-library]] made Mother-owned `ViewShape` records real and allowed buffers to open from persisted active shapes. But a user still cannot ask Mother for a display in Allium terms. The current API requires the caller to know and directly open a `shape_id`.

The Allium target describes a request-composition layer:

- capture a user display request;
- let an agent propose a shape match;
- accept explicit/exact matches when active and sufficiently confident;
- request adaptation or new-shape creation when no exact usable shape exists;
- validate backing data before opening a buffer;
- record outcomes instead of silently losing context.

## Goal

Build the next backend slice from the Allium target:

1. Add durable `DisplayRequest` and `ShapeMatch` records.
2. Expose a structured request-composition API.
3. Let callers submit agent-proposed matches explicitly as data.
4. Compose explicit and exact active-shape matches into buffer opens through the existing library path.
5. Preserve the no-fake-data rule: missing facts or unavailable sources create observability gaps and no buffer.
6. Persist request outcomes so agents/renderers can show what happened.

## Status

Ready for implementation after a short Allium/code alignment pass.

## Allium authority

This spec implements a bounded request-composition slice of:

- `layer/allium/mother/mother-view-composer-target.allium`
- `layer/allium/mother/mother-view-composer-target.plan.json`

Primary Allium entities targeted here:

- `DisplayRequest`
- `ShapeMatch`
- `ViewShape` as selected library data
- `Buffer` only through existing open-buffer service
- `ObservabilityGap` only through existing fail-closed open behavior

Primary Allium rules targeted here:

- `CaptureUserDisplayRequest`
- `SelectExplicitUserRequestedShape`
- `SelectExactShapeMatch`
- `OpenLiveBufferWhenRequiredFactsAreObserved` through the existing shape-library buffer path
- `RecordObservabilityGapWhenRequiredFactIsMissing` through the existing shape-library buffer path

Primary Allium rules intentionally deferred:

- `AdaptSimilarShapeWhenNoExactShapeExists`
- `CreateInitialShapeWhenNoShapeMatches`
- `ReplaceBufferWhenUserRevisesViewShape`
- maturation and observability workflow rules
- concrete renderer frame behavior

## Non-Goals

- Mother does not parse natural language in this slice.
- Mother does not embed LLM prompts or arbitrary agent code in request records.
- Similar-shape adaptation does not create new shapes yet; it belongs to [[mother-view-shape-adaptation]].
- No-match requests do not auto-generate shapes yet; it belongs to [[mother-view-initial-shape-creation]].
- User-facing request UX belongs to [[mother-view-request-ux]].
- User correction/revision flow remains in [[mother-view-buffer-revision]].
- SvelteKit/TUI/Emacs renderer behavior remains outside this spec and begins with [[mother-sveltekit-frame]].

## Target Shape

A request composition input is structured data, not executable logic:

```text
raw_request: "show mother status"
user_id: "local-user"
agent_id: "pi"
proposed_match:
  shape_id: mother.status.default
  match_kind: explicit_user_choice
  confidence: 1.0
```

A successful explicit/exact composition should persist:

```text
DisplayRequest:
  request_id: req_...
  raw_request: "show mother status"
  outcome: buffer_opened

ShapeMatch:
  request_id: req_...
  shape_id: mother.status.default
  match_kind: explicit_user_choice
  confidence: 1.0
```

and return the existing opened-buffer payload envelope.

## Solution

Add a request-composer layer to `mother/src/view_buffer/`:

- extend the model with `DisplayRequest`, `DisplayRequestOutcome`, `ShapeMatch`, and `ShapeMatchKind`;
- persist requests and matches in Mother state;
- expose a structured composition API;
- resolve proposed shape ids through the shape library;
- for `explicit_user_choice`, require active shape and open through existing `view_buffer_open` path;
- for `exact`, require active shape and confidence above `shape_match_confidence_threshold = 0.60` before opening;
- for `similar` and `none`, persist the request/match and return a structured non-open outcome for follow-on work;
- reject arbitrary script/UI fields in request payloads.

## Implementation Order

1. Read Allium and current shape-library/buffer seams; record findings in `DESIGN.md`.
2. Add request/match model types.
3. Add request/match persistence.
4. Add request-composition service behavior.
5. Add HTTP/API/route/daemon wiring.
6. Add deterministic tests and Allium trace comments.
7. Split deferred adaptation/creation UX into follow-on specs.

## Resolved Decisions

- Agents propose matches as structured data; Mother validates and records outcomes.
- Explicit/exact match handling belongs in this slice because it uses already-persisted shapes.
- Similar adaptation and initial shape creation are deferred because they need shape-edit semantics beyond safe selection.
- Missing data remains an observability gap, not a generated placeholder view.

## Verification

```bash
cargo check -q
cargo test -q -p mother view_request
cargo test -q -p mother view_buffer
patina spec check mother-view-request-composer --json
allium check layer/allium/mother/mother-view-composer-target.allium
```

## Exit Criteria

- [x] `mvrc0-allium-code-alignment`
- [x] `mvrc1-request-model`
- [x] `mvrc2-request-persistence`
- [x] `mvrc3-compose-api`
- [x] `mvrc4-explicit-exact-open`
- [x] `mvrc5-fail-closed-outcomes`
- [x] `mvrc6-tests-and-trace`
- [x] `mvrc7-follow-on-backlog`

## Build Readiness

Ready to promote after the matching design document records the Allium/code alignment pass and required Patina design sections.
