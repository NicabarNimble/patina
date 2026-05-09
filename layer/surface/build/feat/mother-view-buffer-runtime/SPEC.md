---
type: feat
id: mother-view-buffer-runtime
status: ready
created: 2026-05-09
sessions:
  origin: 20260508-144836-859149000
related:
- layer/allium/mother/mother-view-composer-target.allium
- layer/allium/mother/mother-view-composer-target.plan.json
- layer/core/spec-driven-design.md
- layer/core/dependable-rust.md
- layer/core/unix-philosophy.md
- layer/core/adapter-pattern.md
- layer/core/values/contract-first-execution.md
- mother/src/
- src/commands/mother/
beliefs:
- '[[spec-driven-design]]'
- '[[core-verbs-standalone-mother-additive]]'
- '[[eventlog-is-infrastructure]]'
- '[[contracts-before-consumers]]'
exit_criteria:
- id: mvbr0-read-before-write
  text: Implementation begins from documented reads of existing Mother route/runtime/state code before adding new view-buffer code.
  checked: false
- id: mvbr1-buffer-model
  text: 'Mother has a first-class persisted buffer model with Emacs vocabulary: buffer, frame, window, major mode, minor modes, live/stale/blocked/replaced/killed states.'
  checked: false
- id: mvbr2-buffer-api
  text: Mother exposes read-only/control-plane buffer APIs for listing buffers, opening a buffer from a known shape, connecting/disconnecting a window, and killing a buffer.
  checked: false
- id: mvbr3-catalog-guardrail
  text: Opening a buffer validates declared backing requirements against a minimal Mother data catalog and refuses to open when required facts are missing.
  checked: false
- id: mvbr4-observability-gap
  text: When required data is missing, Mother records an observability-gap artifact and the caller receives a truthful missing-data response; no display payload is invented.
  checked: false
- id: mvbr5-proof-shape
  text: A minimal built-in proof shape opens a live buffer over existing Mother data, using WIT-framed JSON payload semantics without generating Svelte/TypeScript code.
  checked: false
- id: mvbr6-tests
  text: Deterministic tests cover buffer lifecycle, source requirement validation, missing-data gaps, route/API behavior, and fail-closed paths.
  checked: false
- id: mvbr7-docs-and-allium-trace
  text: Implementation docs and test names reference the Allium obligations from `mother-view-composer-target.plan.json` so the build spec remains traceable to the behavior spec.
  checked: false
---
# feat: Mother View Buffer Runtime

> Implement the first Mother-owned Emacs-like live buffer runtime slice from `mother-view-composer-target.allium`.

## Problem

The target behavior for Mother views is now captured in Allium, but there is no runtime implementation. Atlas was removed as a hardcoded visibility prototype, leaving no active display architecture except the new spec target.

We need a small, concrete slice that proves the core model without prematurely building SvelteKit, a full shape compiler, or the whole observability/maturation loop.

## Goal

Build the first Mother view-buffer runtime foundation:

1. Mother owns buffers, not renderers.
2. Buffers use Emacs vocabulary and lifecycle.
3. Buffers open only from declared backing requirements over observed Mother data.
4. Missing data creates an observability-gap artifact instead of a fake view.
5. Initial payloads use WIT-framed JSON semantics.
6. The slice is testable through Mother API/CLI surfaces before SvelteKit exists.

## Core value anchors

This spec is grounded in `layer/core` values:

- `spec-driven-design.md`: this SPEC is the authority; code must not outrun the contract.
- `dependable-rust.md`: expose a small stable view-buffer interface and keep implementation details private.
- `unix-philosophy.md`: keep catalog lookup, buffer lifecycle, payload framing, and routing as focused jobs.
- `adapter-pattern.md`: do not introduce speculative traits; add seams only where real boundaries exist.
- `values/contract-first-execution.md`: Mother remains authority; framed payloads are contracts, not renderer-owned state.

## Working rules

- Read existing Mother route/runtime/state code before writing view-buffer code.
- Commit with a scalpel as work progresses: focused commits per boundary/model/API/test slice, not one shotgun commit at the end.
- Keep every implementation commit traceable to this SPEC and the Allium target.

## Allium authority

This spec implements the first code slice of:

- `layer/allium/mother/mother-view-composer-target.allium`

Primary Allium concepts implemented in this slice:

- `MotherDisplayContext`
- `MotherDataCatalog`
- `CataloguedSource`
- `CataloguedFact`
- `ViewShape`
- `ViewRequirement`
- `Buffer`
- `Frame`
- `Window`
- `ObservabilityGap`

Primary Allium rules targeted in this slice:

- `OpenLiveBufferWhenRequiredFactsAreObserved`
- `RecordObservabilityGapWhenRequiredFactIsMissing`
- `ConnectWindowToExistingBuffer`
- `DisconnectWindowWithoutKillingBuffer`
- `KillBufferWhenUserClosesBuffer`
- degraded state rules may be stubbed or partially implemented if live invalidation is not yet available

## Non-goals

- Full local Allium view-shape compiler/indexer.
- SvelteKit frame implementation.
- TUI or Emacs client implementation.
- Arbitrary generated frontend code.
- Shared/multiplayer buffers.
- Mature typed WIT promotion.
- Maturation of derivations/patterns into build work items.

## Initial proof shape

Use a minimal built-in shape over existing Mother data, preferably Mother health/status or eventlog facts, because those are already observable.

Example conceptual request:

> Open a live Mother status buffer.

The proof shape should declare required facts explicitly, such as:

- `mother.status.version`
- `mother.status.control_plane_ready`
- `mother.status.registered_projects`
- `mother.status.children_ready_count`
- `mother.status.children_total`

If a requested proof shape requires a fact not in the minimal catalog, Mother must not open the buffer and must record an observability gap.

## Verification

```bash
cargo check -q
cargo test -q -p mother view_buffer
cargo test -q -p patina-ai mother_view
patina spec check mother-view-buffer-runtime --json
allium check layer/allium/mother/mother-view-composer-target.allium
```

## Exit Criteria

- [ ] `mvbr0-read-before-write`
- [ ] `mvbr1-buffer-model`
- [ ] `mvbr2-buffer-api`
- [ ] `mvbr3-catalog-guardrail`
- [ ] `mvbr4-observability-gap`
- [ ] `mvbr5-proof-shape`
- [ ] `mvbr6-tests`
- [ ] `mvbr7-docs-and-allium-trace`
